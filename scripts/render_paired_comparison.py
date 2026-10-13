#!/usr/bin/env python3
"""Create a standalone reader for a pinned uma-sim.paired-batches.v1 report."""
from __future__ import annotations

import argparse
import base64
import hashlib
import html
import json
import math
import os
from pathlib import Path
import re
import stat
import sys

MAX_BYTES = 64 * 1024 * 1024
MAX_PAIRS = 100_000
DELTA_LIMIT = (1 << 32) - 1
TOP_KEYS = {"format", "direction", "identity_fields", "summary", "comparisons"}
ROW_KEYS = {"scenario", "trainee", "seed", "baseline_score", "input_score",
            "score_delta", "baseline_grade", "input_grade"}
SUMMARY_KEYS = {"count", "wins", "ties", "losses", "mean_score_delta",
                "median_score_delta", "min_score_delta", "max_score_delta",
                "population_stddev_score_delta"}


def need(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def unique_object(pairs: list) -> dict:
    result = {}
    for key, value in pairs:
        need(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise ValueError("nonfinite JSON number")


def exact_keys(value: object, keys: set, context: str) -> None:
    need(type(value) is dict and set(value) == keys, context + ": unsupported fields")


def integer(value: object, low: int, high: int, context: str) -> None:
    need(type(value) is int and low <= value <= high, context + ": invalid integer")


def text_field(value: object, limit: int, context: str) -> None:
    need(type(value) is str, context + ": expected string")
    need(1 <= len(value.encode("utf-8")) <= limit, context + ": invalid UTF-8 byte length")


def validate(raw: bytes) -> dict:
    need(len(raw) <= MAX_BYTES, "input exceeds 64 MiB")
    report = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object,
                        parse_constant=reject_constant)
    exact_keys(report, TOP_KEYS, "report")
    need(report["format"] == "uma-sim.paired-batches.v1", "unsupported report format")
    need(report["direction"] == "input_minus_baseline", "unsupported score direction")
    need(report["identity_fields"] == ["scenario", "trainee", "seed"], "unsupported identity")
    rows = report["comparisons"]
    need(type(rows) is list and 1 <= len(rows) <= MAX_PAIRS, "expected 1..100000 pairs")
    previous = None
    deltas = []
    for offset, row in enumerate(rows):
        context = "pair " + str(offset + 1)
        exact_keys(row, ROW_KEYS, context)
        for field in ("scenario", "trainee"):
            text_field(row[field], 256, context + " " + field)
        for field in ("baseline_grade", "input_grade"):
            text_field(row[field], 64, context + " " + field)
        integer(row["seed"], -(1 << 63), (1 << 63) - 1, context + " seed")
        for field in ("baseline_score", "input_score"):
            integer(row[field], -(1 << 31), (1 << 31) - 1, context + " " + field)
        integer(row["score_delta"], -DELTA_LIMIT, DELTA_LIMIT, context + " delta")
        need(row["score_delta"] == row["input_score"] - row["baseline_score"],
             context + ": score delta does not match scores")
        identity = (row["scenario"], row["trainee"], row["seed"])
        need(previous is None or previous < identity, "pairs must have unique sorted identities")
        previous = identity
        deltas.append(row["score_delta"])
    deltas.sort()
    count = len(deltas)
    mean = sum(deltas) / count
    median = (deltas[count // 2] if count % 2 else
              (deltas[count // 2 - 1] + deltas[count // 2]) / 2)
    # Same sorted accumulation order as native batch_pairs.rs. Small platform
    # floating differences are admitted below; all integer facts remain exact.
    variance = sum((value - mean) * (value - mean) for value in deltas) / count
    expected = {"count": count, "wins": sum(v > 0 for v in deltas),
                "ties": deltas.count(0), "losses": sum(v < 0 for v in deltas),
                "min_score_delta": deltas[0], "max_score_delta": deltas[-1],
                "mean_score_delta": mean, "median_score_delta": median,
                "population_stddev_score_delta": math.sqrt(variance)}
    summary = report["summary"]
    exact_keys(summary, SUMMARY_KEYS, "summary")
    for field in ("count", "wins", "ties", "losses", "min_score_delta", "max_score_delta"):
        need(type(summary[field]) is int and summary[field] == expected[field],
             "summary " + field + " does not match complete pairs")
    for field in ("mean_score_delta", "median_score_delta", "population_stddev_score_delta"):
        value = summary[field]
        need(type(value) in (int, float) and -DELTA_LIMIT <= value <= DELTA_LIMIT,
             "summary " + field + ": invalid finite value")
        if field == "population_stddev_score_delta":
            need(value >= 0, "summary population standard deviation must be nonnegative")
        need(math.isclose(value, expected[field], rel_tol=1e-12, abs_tol=1e-9),
             "summary " + field + " does not match complete pairs")
    return report


def fingerprint(s: os.stat_result) -> tuple:
    return (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns)


def read_regular(path: Path) -> tuple[bytes, tuple]:
    initial = path.lstat()
    need(stat.S_ISREG(initial.st_mode), "input must be a regular nonsymlink file")
    flags = (os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) |
             getattr(os, "O_BINARY", 0) | getattr(os, "O_NONBLOCK", 0))
    fd = os.open(path, flags)
    with os.fdopen(fd, "rb") as stream:
        opened = os.fstat(stream.fileno())
        need(fingerprint(initial) == fingerprint(opened), "input changed while opening")
        raw = stream.read(MAX_BYTES + 1)
        need(len(raw) <= MAX_BYTES, "input exceeds 64 MiB")
        need(fingerprint(opened) == fingerprint(os.fstat(stream.fileno())), "input changed while reading")
    need(fingerprint(path.lstat()) == fingerprint(initial), "input path changed while reading")
    return raw, fingerprint(initial)


def visible(value: str) -> str:
    """Make nonprinting controls visible; original bytes remain downloadable."""
    value = "".join("⟦U+%04X⟧" % ord(c) if (ord(c) < 32 and c not in "\t\n\r") or
                    127 <= ord(c) <= 159 else c for c in value)
    # Literal CR in HTML source is normalized by browsers. A character reference
    # retains it in the DOM; CSS preserves line breaks and whitespace.
    return html.escape(value, quote=True).replace("\r", "&#13;")


def number(value: float) -> str:
    return format(value, ",.3f").rstrip("0").rstrip(".") if type(value) is float else format(value, ",")


SCRIPT = r"""(() => {
  'use strict';
  const body = document.querySelector('#pairs tbody');
  const rows = Array.from(body.rows);
  const query = document.querySelector('#query');
  const outcome = document.querySelector('#outcome');
  const order = document.querySelector('#order');
  function update() {
    const term = query.value.toLocaleLowerCase();
    const selected = outcome.value;
    const sorted = rows.slice().sort((a, b) => {
      const x = Number(a.dataset.delta), y = Number(b.dataset.delta);
      const tie = Number(a.dataset.index) - Number(b.dataset.index);
      if (order.value === 'descending') return y - x || tie;
      if (order.value === 'ascending') return x - y || tie;
      if (order.value === 'absolute') return Math.abs(y) - Math.abs(x) || tie;
      return tie;
    });
    const fragment = document.createDocumentFragment();
    let matches = 0;
    for (const row of sorted) {
      row.hidden = !((selected === 'all' || row.dataset.outcome === selected) &&
        row.textContent.toLocaleLowerCase().includes(term));
      if (!row.hidden) matches += 1;
      fragment.appendChild(row);
    }
    body.appendChild(fragment);
    document.querySelector('#match-count').textContent = `${matches} of ${rows.length} pairs shown`;
  }
  query.addEventListener('input', update);
  outcome.addEventListener('change', update);
  order.addEventListener('change', update);
  document.querySelector('#reset').addEventListener('click', () => {
    query.value = ''; outcome.value = 'all'; order.value = 'identity'; update(); query.focus();
  });
  document.querySelector('#controls').hidden = false;
})();"""

CSS = """*{box-sizing:border-box}body{margin:0;background:#f3f1e9;color:#17342f;font:16px/1.5 system-ui,-apple-system,Segoe UI,sans-serif}main{max-width:1320px;margin:auto;padding:48px 28px 60px}.eyebrow{font-size:12px;font-weight:750;letter-spacing:.16em;text-transform:uppercase;color:#47665c}h1{font-family:Georgia,serif;font-size:clamp(34px,5vw,56px);line-height:1.06;letter-spacing:-.035em;margin:12px 0 18px}h2{font-size:23px;margin:0 0 18px}.intro{max-width:800px;font-size:18px;color:#466158}.direction{display:inline-block;background:#e2e9df;border-radius:6px;padding:7px 12px;margin:0 0 18px;font-weight:650}.cards{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:14px;margin:30px 0 18px}.card,.panel{background:#fffef9;border:1px solid #d8dfd3;border-radius:12px;padding:22px}.card dt{font-size:13px;color:#4b645b}.card dd{font-size:31px;line-height:1.2;margin:8px 0 0;font-variant-numeric:tabular-nums}.panel{margin:18px 0}.bar{display:flex;height:24px;border-radius:5px;overflow:hidden;background:#dfe5df}.win{background:#167567}.tie{background:#9caa9b}.loss{background:#bb593f}.legend{display:flex;gap:24px;flex-wrap:wrap;margin:12px 0 0}.dot{display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:6px}.note{color:#52685f;font-size:14px}.stats{display:flex;gap:28px;flex-wrap:wrap;margin-top:20px}.stats div{font-size:14px}.stats strong{display:block;font-size:19px}.controls{display:grid;grid-template-columns:minmax(160px,1fr) 170px 200px auto;gap:12px;align-items:end;margin:20px 0}label{display:block;font-size:13px;font-weight:650}input,select,button{font:inherit;color:inherit;background:#fffef9;border:1px solid #9bad9d;border-radius:6px;padding:10px;width:100%;margin-top:5px}button{cursor:pointer;background:#e5eee4}button:hover{background:#d3e3d2}:focus-visible{outline:3px solid #3066ba;outline-offset:2px}.table-scroll{overflow:auto;max-height:70vh;border:1px solid #d8dfd3;border-radius:8px}table{border-collapse:collapse;width:100%;font-size:14px}caption{text-align:left;padding:0 0 12px;font-size:14px;color:#52685f}th{background:#e9eee4;text-align:left;position:sticky;top:0;z-index:1;white-space:nowrap}th,td{padding:13px 14px;border-bottom:1px solid #e1e7dd;vertical-align:top}td.literal{white-space:pre-wrap;overflow-wrap:anywhere;min-width:110px;max-width:240px}td.numeric,th.numeric{text-align:right;white-space:nowrap;font-variant-numeric:tabular-nums}td.delta{font-weight:750}.positive{color:#0d7161}.negative{color:#a8442d}tr:nth-child(even){background:#f8f9f3}[hidden]{display:none!important}details{margin:16px 0}summary{cursor:pointer;font-weight:650}code{overflow-wrap:anywhere;font-size:12px}a{color:#135f53;text-underline-offset:3px}.download{display:inline-block;padding:9px 13px;border:1px solid #809888;border-radius:6px;text-decoration:none;margin:8px 0}footer{margin-top:28px;border-top:1px solid #ccd6c9;padding-top:20px}.count{font-weight:650;margin:0 0 14px}@media(max-width:760px){main{padding:28px 16px}.cards{grid-template-columns:repeat(2,minmax(0,1fr))}.controls{grid-template-columns:1fr 1fr}.card,.panel{padding:16px}.card dd{font-size:26px}}@media print{body{background:white}main{max-width:none;padding:0}.controls,.download{display:none}.table-scroll{max-height:none;overflow:visible}th{position:static}.card,.panel{break-inside:avoid}tr{break-inside:avoid}.panel:has(table){break-inside:auto}}"""


def render(report: dict, raw: bytes, baseline_label: str, input_label: str) -> bytes:
    for label in (baseline_label, input_label):
        text_field(label, 128, "label")
        need(not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in label), "label contains control characters")
    baseline, current = visible(baseline_label), visible(input_label)
    summary = report["summary"]
    count = summary["count"]
    table_rows = []
    for index, row in enumerate(report["comparisons"]):
        delta = row["score_delta"]
        outcome = "wins" if delta > 0 else "losses" if delta < 0 else "ties"
        tone = "positive" if delta > 0 else "negative" if delta < 0 else ""
        cells = [f'<td class="literal">{visible(row[field])}</td>' for field in ("scenario", "trainee")]
        cells += [f'<td class="numeric">{row[field]}</td>' for field in ("seed", "baseline_score", "input_score")]
        cells += [f'<td class="numeric delta {tone}">{delta:+d}</td>']
        cells += [f'<td class="literal">{visible(row[field])}</td>' for field in ("baseline_grade", "input_grade")]
        table_rows.append(f'<tr data-index="{index}" data-delta="{delta}" data-outcome="{outcome}">'+"".join(cells)+"</tr>")
    cards = "".join(f'<div class="card"><dt>{label}</dt><dd>{number(value)}</dd></div>' for label, value in
                    (("Complete pairs", count), ("Mean score difference", summary["mean_score_delta"]),
                     ("Median score difference", summary["median_score_delta"]),
                     ("Population standard deviation", summary["population_stddev_score_delta"])))
    bar = "".join(f'<span class="{name}" style="width:{summary[field]/count*100:.9f}%"></span>'
                  for name, field in (("win", "wins"), ("tie", "ties"), ("loss", "losses")))
    legend = "".join(f'<span><i class="dot {name}" aria-hidden="true"></i>{summary[field]:,} {field}</span>'
                     for name, field in (("win", "wins"), ("tie", "ties"), ("loss", "losses")))
    script_pin = base64.b64encode(hashlib.sha256(SCRIPT.encode()).digest()).decode()
    digest = sha(raw)
    document = f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'sha256-{script_pin}'; base-uri 'none'; form-action 'none'">
<title>Paired career results · UMA</title><style>{CSS}</style></head><body><main>
<header><div class="eyebrow">UMA · Saved comparison</div><h1>Paired career results</h1>
<p class="intro">Explore every matched career in this saved report. Each difference compares the same scenario, trainee and exact seed.</p>
<p class="direction">{current} − {baseline}</p><p class="note">Labels supplied by the reader. Positive differences mean a higher Input score; grades are shown as recorded.</p></header>
<section aria-label="Complete cohort summary"><dl class="cards">{cards}</dl>
<div class="panel"><h2>Across the complete cohort</h2><div class="bar" aria-hidden="true">{bar}</div><div class="legend">{legend}</div>
<div class="stats"><div>Lowest difference<strong>{summary['min_score_delta']:+,d}</strong></div><div>Highest difference<strong>{summary['max_score_delta']:+,d}</strong></div></div>
<p class="note">Summary values always cover all {count:,} pairs. Displayed averages are rounded to three decimal places; the original JSON retains their full values.</p></div></section>
<section class="panel" aria-labelledby="pairs-heading"><h2 id="pairs-heading">Every paired result</h2>
<div id="controls" class="controls" hidden><label>Search displayed fields<input id="query" type="search" autocomplete="off" placeholder="Scenario, trainee, seed or grade"></label>
<label>Outcome<select id="outcome"><option value="all">All pairs</option><option value="wins">Wins</option><option value="ties">Ties</option><option value="losses">Losses</option></select></label>
<label>Order<select id="order"><option value="identity">Scenario / trainee / seed</option><option value="descending">Difference: high to low</option><option value="ascending">Difference: low to high</option><option value="absolute">Largest absolute difference</option></select></label><button id="reset" type="button">Reset view</button></div>
<p id="match-count" class="count" aria-live="polite">{count} of {count} pairs shown</p><noscript><p class="note">JavaScript is disabled. The complete summary and all rows are available below.</p></noscript>
<div class="table-scroll" tabindex="0" role="region" aria-label="Complete paired results table"><table id="pairs"><caption>Difference = {current} score − {baseline} score. Initial order is scenario, trainee, signed seed.</caption>
<thead><tr><th scope="col">Scenario</th><th scope="col">Trainee</th><th scope="col" class="numeric">Seed</th><th scope="col" class="numeric">{baseline} score</th><th scope="col" class="numeric">{current} score</th><th scope="col" class="numeric">Difference</th><th scope="col">{baseline} grade</th><th scope="col">{current} grade</th></tr></thead><tbody>{''.join(table_rows)}</tbody></table></div></section>
<footer><h2>What this report establishes</h2><p>These are descriptive results for the recorded cohort. They do not establish independent sampling, statistical significance or a causal policy benefit. The saved comparison does not authenticate simulator version, policy, deck or random-number consumption.</p>
<details><summary>Source and exact data</summary><p>All identities, scores, deltas and summary values were checked before this document was written. Seeds remain exact signed 64-bit decimal text. Search and filters change the table view only.</p>
<p class="note">Nonprinting C0/C1 controls other than tab, line feed and carriage return appear as ⟦U+XXXX⟧ markers. Whitespace is retained for display; download the original bytes for exact data processing.</p>
<p>Format: <code>uma-sim.paired-batches.v1</code><br>Input SHA256: <code id="input-sha256">{digest}</code><br>Input bytes: {len(raw):,}</p>
<a class="download" download="paired-comparison.json" href="data:application/json;base64,{base64.b64encode(raw).decode()}">Download original comparison JSON</a></details>
<p class="note">Standalone offline reader · No external assets or requests · Keep the original batch files and study context alongside this report.</p></footer>
</main><script>{SCRIPT}</script></body></html>'''
    return document.encode("utf-8")


def create_report(input_path: Path, expected_sha: str, output_path: Path,
                  baseline_label: str = "Baseline", input_label: str = "Input") -> dict:
    need(re.fullmatch(r"[0-9a-fA-F]{64}", expected_sha) is not None, "expected SHA256 must contain 64 hex digits")
    need(not os.path.lexists(output_path), "output must be a new file")
    raw, identity = read_regular(input_path)
    need(sha(raw) == expected_sha.lower(), "input SHA256 mismatch")
    report = validate(raw)
    result = render(report, raw, baseline_label, input_label)
    need(read_regular(input_path) == (raw, identity), "input changed before output creation")
    created = None
    try:
        with output_path.open("xb") as stream:
            created = fingerprint(os.fstat(stream.fileno()))[:2]
            stream.write(result)
            stream.flush()
            os.fsync(stream.fileno())
        need(read_regular(input_path) == (raw, identity), "input changed during output creation")
    except BaseException:
        # Remove only this invocation's incomplete file, never an existing or
        # concurrently substituted destination.
        if created is not None:
            try:
                if fingerprint(output_path.lstat())[:2] == created:
                    output_path.unlink()
            except OSError:
                pass
        raise
    return {"format": "uma-sim.paired-html-receipt.v1", "status": "ok",
            "input_sha256": sha(raw), "input_bytes": len(raw),
            "output_sha256": sha(result), "output_bytes": len(result),
            "pairs": len(report["comparisons"]), "output": str(output_path)}


class Parser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        raise ValueError(message)


def main(argv: list[str] | None = None) -> int:
    try:
        parser = Parser(description=__doc__)
        parser.add_argument("--input", type=Path, required=True)
        parser.add_argument("--input-sha256", required=True)
        parser.add_argument("--output", type=Path, required=True)
        parser.add_argument("--baseline-label", default="Baseline")
        parser.add_argument("--input-label", default="Input")
        args = parser.parse_args(argv)
        receipt = create_report(args.input, args.input_sha256, args.output,
                                args.baseline_label, args.input_label)
        print(json.dumps(receipt, sort_keys=True))
        return 0
    except (OSError, ValueError, UnicodeError, RecursionError) as error:
        print(json.dumps({"status": "error", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
