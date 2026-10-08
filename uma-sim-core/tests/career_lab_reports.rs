//! Receiving tests for the downloadable career-lab report.
//!
//! These construct the public comparison type directly, so text admission and
//! rendering are exercised without changing game rules or invoking a policy.
//! The small helpers inspect the fixed report's rows and standard inline
//! escaping; they are not a replacement for the separate real GFM/browser check.

use serde_json::{json, Value};
use uma_sim_core::career_lab::{render_json, render_markdown, BranchComparison};

fn outcome(base: i32) -> Value {
    json!({
        "steps": base + 3, "finalTurn": base + 11, "careerComplete": base != 0,
        "completedRaces": [format!("race-{base}")],
        "stats": {"speed": base + 101, "stamina": base + 102,
            "power": base + 103, "guts": base + 104, "wit": base + 105},
        "energy": base + 40, "mood": format!("MOOD-{base}"),
        "fans": base + 1234, "skillPoints": base + 56,
        "learnedSkills": [format!("skill-{base}")],
        "sparks": [{"color": format!("color-{base}"), "factorId": "hidden-factor-id",
            "stars": 2, "label": format!("spark-{base}")}],
        "scenarioResources": {"tokens": base + 7},
        "totalRngCalls": base + 789, "telemetryRecords": base + 90
    })
}

fn timeline_row(step: i32, same_action: bool, same_outcome: bool) -> Value {
    json!({
        "stepIndex": step, "turnA": step + 7, "turnB": step + 7,
        "actionA": "hidden-action-a", "actionB": "hidden-action-b",
        "labelA": format!("action-A-{step}"), "labelB": format!("action-B-{step}"),
        "sameAction": same_action, "sameOutcome": same_outcome,
        "energyA": step + 20, "energyB": step + 30,
        "moodA": "unrendered-timeline-mood-a", "moodB": "unrendered-timeline-mood-b",
        "fansA": step + 100, "fansB": step + 200,
        "skillPointsA": step + 40, "skillPointsB": step + 50
    })
}

fn fixture() -> BranchComparison {
    let mut missing = timeline_row(3, false, false);
    for field in [
        "turnA",
        "actionA",
        "labelA",
        "energyA",
        "moodA",
        "fansA",
        "skillPointsA",
    ] {
        missing[field] = Value::Null;
    }
    serde_json::from_value(json!({
        "aId": "hidden-branch-a", "aName": "Rest branch",
        "bId": "hidden-branch-b", "bName": "Train branch",
        "checkpointName": "before-debut", "checkpointTurn": 7,
        "seed": -17, "sameCheckpoint": false, "comparedAt": "2026-10-08T09:15:00Z",
        "firstDivergence": {
            "stepIndex": 1, "turn": 8, "dateLabel": "Junior year, May",
            "phase": "FREE", "kind": "decision", "actionA": "rest",
            "actionB": "train_speed", "labelA": "Take a rest", "labelB": "Train speed",
            "rngCallsAAfter": 31, "rngCallsBAfter": 32, "note": "The decisions differ."
        },
        "aligned": [timeline_row(0, true, true), timeline_row(1, false, false),
            timeline_row(2, true, false), missing],
        "outcomeA": outcome(0), "outcomeB": outcome(100),
        "caveats": ["Synthetic fixture; no learning or game benefit claim.", "RNG streams can diverge."]
    }))
    .expect("complete public comparison fixture")
}

// Every public string location that the existing renderer displays. Hidden
// identity, scenario-resource and telemetry fields are checked separately.
const DISPLAYED_FIELDS: &[&str] = &[
    "/aName",
    "/bName",
    "/checkpointName",
    "/comparedAt",
    "/firstDivergence/dateLabel",
    "/firstDivergence/phase",
    "/firstDivergence/kind",
    "/firstDivergence/actionA",
    "/firstDivergence/actionB",
    "/firstDivergence/labelA",
    "/firstDivergence/labelB",
    "/firstDivergence/note",
    "/aligned/0/labelA",
    "/aligned/1/labelB",
    "/outcomeA/mood",
    "/outcomeB/mood",
    "/outcomeA/completedRaces/0",
    "/outcomeB/completedRaces/0",
    "/outcomeA/learnedSkills/0",
    "/outcomeB/learnedSkills/0",
    "/outcomeA/sparks/0/color",
    "/outcomeB/sparks/0/color",
    "/outcomeA/sparks/0/label",
    "/outcomeB/sparks/0/label",
    "/caveats/0",
    "/caveats/1",
];

fn with_text(field: &str, text: &str) -> BranchComparison {
    let mut value = serde_json::to_value(fixture()).unwrap();
    *value.pointer_mut(field).expect("existing displayed field") = Value::String(text.into());
    serde_json::from_value(value).unwrap()
}

fn escaped_at(text: &str, offset: usize) -> bool {
    text.as_bytes()[..offset]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 1
}

// Recognize a single-line CommonMark code span by a matching whole backtick
// run. Its contents are literal: prose escapes/entities do not apply there.
// Block parsing still belongs to the independent real GFM receiver.
fn code_span(text: &str) -> Option<(&str, &str)> {
    let delimiter = text.bytes().take_while(|&ch| ch == b'`').count();
    if delimiter == 0 {
        return None;
    }
    let line_end = text.find('\n').unwrap_or(text.len());
    let mut offset = delimiter;
    while offset < line_end {
        let start = offset + text[offset..line_end].find('`')?;
        let run = text[start..line_end]
            .bytes()
            .take_while(|&ch| ch == b'`')
            .count();
        if run == delimiter {
            return Some((&text[delimiter..start], &text[start + run..]));
        }
        offset = start + run;
    }
    None
}

fn code_text(text: &str) -> String {
    if text.starts_with(' ') && text.ends_with(' ') && !text.chars().all(|ch| ch == ' ') {
        text[1..text.len() - 1].into()
    } else {
        text.into()
    }
}

fn has_unprotected(text: &str, needle: &str) -> bool {
    let mut remaining = text;
    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix('\\') {
            if let Some(ch) = rest.chars().next().filter(char::is_ascii_punctuation) {
                remaining = &rest[ch.len_utf8()..];
                continue;
            }
        }
        if let Some((_, rest)) = code_span(remaining) {
            remaining = rest;
            continue;
        }
        if remaining.starts_with(needle) {
            return true;
        }
        remaining = &remaining[remaining.chars().next().unwrap().len_utf8()..];
    }
    false
}

fn cells(line: &str) -> Vec<String> {
    let separators: Vec<_> = line
        .char_indices()
        .filter_map(|(offset, ch)| (ch == '|' && !escaped_at(line, offset)).then_some(offset))
        .collect();
    assert_eq!(
        separators.first(),
        Some(&0),
        "table row begins with a delimiter: {line}"
    );
    assert_eq!(
        separators.last(),
        Some(&(line.len() - 1)),
        "table row ends with a delimiter: {line}"
    );
    separators
        .windows(2)
        .map(|pair| line[pair[0] + 1..pair[1]].trim().to_owned())
        .collect()
}

fn section_rows(markdown: &str, heading: &str) -> Vec<Vec<String>> {
    let mut lines = markdown.lines().skip_while(|line| *line != heading);
    assert_eq!(
        lines.next(),
        Some(heading),
        "required report section {heading}"
    );
    lines
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| line.starts_with('|'))
        .map(cells)
        .collect()
}

fn assert_topology(markdown: &str) {
    let headings: Vec<_> = markdown
        .lines()
        .filter(|line| line.starts_with('#'))
        .collect();
    assert_eq!(
        headings.len(),
        5,
        "source data must not add document headings"
    );
    assert!(headings[0].starts_with("# Branch comparison: "));
    assert_eq!(
        &headings[1..],
        [
            "## First divergence",
            "## Timeline (per step)",
            "## Final outcomes",
            "## Caveats"
        ]
    );
    let timeline = section_rows(markdown, "## Timeline (per step)");
    assert_eq!(
        timeline.len(),
        6,
        "four authored steps plus header and separator"
    );
    assert!(
        timeline.iter().all(|row| row.len() == 8),
        "timeline keeps eight columns: {timeline:?}"
    );
    let outcomes = section_rows(markdown, "## Final outcomes");
    assert_eq!(
        outcomes.len(),
        14,
        "twelve outcome rows plus header and separator"
    );
    assert!(
        outcomes.iter().all(|row| row.len() == 3),
        "outcomes keep their A/B columns: {outcomes:?}"
    );
}

fn entity(text: &str) -> Option<char> {
    match text {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let scalar =
                if let Some(hex) = text.strip_prefix("#x").or_else(|| text.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    text.strip_prefix('#')?.parse().ok()?
                };
            char::from_u32(scalar)
        }
    }
}

// Decode standard literal escapes/entities exactly once. This is deliberately
// an inline text check, not a second implementation of Markdown rendering.
fn inline_text(text: &str) -> String {
    let mut out = String::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix('\\') {
            if let Some(ch) = rest.chars().next().filter(char::is_ascii_punctuation) {
                out.push(ch);
                remaining = &rest[ch.len_utf8()..];
                continue;
            }
        }
        if let Some((literal, rest)) = code_span(remaining) {
            out.push_str(&code_text(literal));
            remaining = rest;
            continue;
        }
        if let Some(rest) = remaining.strip_prefix('&') {
            if let Some(end) = rest.find(';').filter(|&end| end <= 12) {
                if let Some(ch) = entity(&rest[..end]) {
                    out.push(ch);
                    remaining = &rest[end + 1..];
                    continue;
                }
            }
        }
        if let Some(br) = ["<br>", "<br/>", "<br />"]
            .iter()
            .find(|br| remaining.starts_with(**br))
        {
            out.push('\n');
            remaining = &remaining[br.len()..];
            continue;
        }
        let ch = remaining.chars().next().unwrap();
        out.push(ch);
        remaining = &remaining[ch.len_utf8()..];
    }
    out
}

fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn ordinary_reports_keep_numeric_columns_and_timeline_meaning() {
    let report = render_markdown(&fixture());
    assert_topology(&report);
    let outcomes: Vec<Vec<_>> = section_rows(&report, "## Final outcomes")
        .into_iter()
        .map(|row| row.iter().map(|cell| inline_text(cell)).collect())
        .collect();
    let row = |name: &str| outcomes.iter().find(|row| row[0] == name).unwrap();
    for (name, a, b) in [
        ("Branch", "Rest branch", "Train branch"),
        ("Steps", "3", "103"),
        ("Career complete", "false", "true"),
        ("Final turn", "11", "111"),
        ("Fans", "1234", "1334"),
        ("Skill points", "56", "156"),
        ("RNG calls", "789", "889"),
        ("Races", "1 (race-0)", "1 (race-100)"),
        ("Skills learned", "1: skill-0", "1: skill-100"),
    ] {
        assert_eq!(
            &row(name)[1..],
            [a, b],
            "{name} keeps the supplied A/B values"
        );
    }
    assert_eq!(
        &row("Stats")[1..],
        [
            "Spd 101 / Sta 102 / Pow 103 / Gut 104 / Wit 105",
            "Spd 201 / Sta 202 / Pow 203 / Gut 204 / Wit 205"
        ]
    );
    assert_eq!(
        &row("Energy / mood")[1..],
        ["40 / MOOD-0", "140 / MOOD-100"]
    );
    assert_eq!(
        &row("Sparks")[1..],
        ["color-0★2 spark-0", "color-100★2 spark-100"]
    );
    let timeline: Vec<Vec<_>> = section_rows(&report, "## Timeline (per step)")
        .into_iter()
        .map(|row| row.iter().map(|cell| inline_text(cell)).collect())
        .collect();
    assert_eq!(
        timeline[2],
        [
            "0",
            "7",
            "action-A-0",
            "action-B-0",
            "",
            "20→30",
            "100→200",
            "40→50"
        ]
    );
    assert_eq!(timeline[3][4], "≠ act");
    assert_eq!(timeline[4][4], "≠ out");
    assert_eq!(
        timeline[5],
        [
            "3",
            "–",
            "–",
            "action-B-3",
            "≠ act",
            "–→33",
            "–→203",
            "–→53"
        ]
    );
    assert!(inline_text(&report).contains("seed -17, turn 7"));
}

#[test]
fn branch_names_remain_literal_and_cannot_split_outcome_columns() {
    for name in [
        "Rest | train",
        "Rest \\| train",
        "Rest `|` train",
        "日本語 🐎 | retain",
        r"literal \*stars\* and \_under\_",
        "&lt;b&gt; &amp; &#124;",
        "<u data-report='x'>label</u>",
    ] {
        for field in ["/aName", "/bName"] {
            let report = render_markdown(&with_text(field, name));
            assert_topology(&report);
            let rows = section_rows(&report, "## Final outcomes");
            let branch = rows.iter().find(|row| row[0] == "Branch").unwrap();
            let index = if field == "/aName" { 1 } else { 2 };
            assert_eq!(
                inline_text(&branch[index]),
                name,
                "literal {field}: {name:?}"
            );
        }
    }
}

#[test]
fn every_displayed_field_keeps_line_breaks_out_of_report_structure() {
    let expected_lines = render_markdown(&fixture()).lines().count();
    let text = "prefix\r\n## injected heading\n| injected | row |\rsuffix\t日本語";
    for field in DISPLAYED_FIELDS {
        let report = render_markdown(&with_text(field, text));
        assert_topology(&report);
        assert_eq!(
            report.lines().count(),
            expected_lines,
            "{field} created a report line"
        );
        assert!(
            !report.contains('\r'),
            "{field} leaked a raw carriage return"
        );
        assert!(
            words(&inline_text(&report)).contains(&words(text)),
            "{field} lost authored text"
        );
    }
}

#[test]
fn every_displayed_field_preserves_punctuation_and_entity_text() {
    let text = r"literal \*stars\* \_under\_ \\ path `code` [label](urn:fixture) &lt;b&gt; &amp; &#124; 日本語";
    for field in DISPLAYED_FIELDS {
        let report = render_markdown(&with_text(field, text));
        assert_topology(&report);
        assert!(
            inline_text(&report).contains(text),
            "{field} changed literal escape/entity text: {report}"
        );
    }
}

#[test]
fn every_displayed_field_prevents_raw_html_elements() {
    let text = "<u data-report='fixture'>keep this literal</u><img src='urn:fixture'>";
    for field in DISPLAYED_FIELDS {
        let report = render_markdown(&with_text(field, text));
        assert!(
            !has_unprotected(&report, "<u data-report="),
            "raw HTML from {field}"
        );
        assert!(
            !has_unprotected(&report, "<img src="),
            "raw image from {field}"
        );
        assert!(
            inline_text(&report).contains(text),
            "{field} must retain literal tag text"
        );
    }
}

#[test]
fn inline_code_fields_keep_backtick_runs_entities_and_significant_padding() {
    let cases = [
        "",
        "`",
        "``",
        "```",
        "`edge`",
        "``one`two```three``",
        "leading ",
        " trailing",
        " both ",
        "   ",
        "first\r\nsecond\rlast\nline",
        r"`<tag>&amp;\*literal\*|[url](urn:fixture)`",
        "日本語 🐎 `値`",
    ];
    for (field, prefix, suffix) in [
        ("/checkpointName", "- Checkpoint: ", " (seed -17, turn 7)"),
        ("/firstDivergence/phase", " · phase ", " · kind `decision`"),
        ("/firstDivergence/kind", " · kind ", ""),
        ("/firstDivergence/actionA", "- A: ", " — Take a rest"),
        ("/firstDivergence/actionB", "- B: ", " — Train speed"),
    ] {
        for value in cases {
            let report = render_markdown(&with_text(field, value));
            assert_topology(&report);
            let line = report.lines().find(|line| line.contains(prefix)).unwrap();
            let fragment = line.split_once(prefix).unwrap().1;
            let fragment = fragment.strip_suffix(suffix).unwrap();
            let expected = value.replace("\r\n", " ").replace(['\r', '\n'], " ");
            assert_eq!(
                inline_text(fragment),
                expected,
                "{field} code span changed {value:?}: {fragment:?}"
            );
        }
    }
}

#[test]
fn rendering_does_not_rewrite_source_data_or_json_report() {
    let text = "Keep | \\ `ticks` <b>&amp;</b>\r\n日本語\tfinal";
    let mut value = serde_json::to_value(fixture()).unwrap();
    for field in DISPLAYED_FIELDS {
        *value.pointer_mut(field).unwrap() = Value::String(format!("{field}: {text}"));
    }
    let comparison: BranchComparison = serde_json::from_value(value.clone()).unwrap();
    let json_before = render_json(&comparison);
    let _ = render_markdown(&comparison);
    assert_eq!(serde_json::to_value(&comparison).unwrap(), value);
    assert_eq!(render_json(&comparison), json_before);
    let json_after: Value = serde_json::from_str(&json_before).unwrap();
    assert_eq!(json_after, value);
    assert_eq!(json_after["aId"], "hidden-branch-a");
    assert_eq!(json_after["outcomeA"]["scenarioResources"]["tokens"], 7);
    assert_eq!(json_after["outcomeB"]["telemetryRecords"], 190);
}

#[test]
fn changed_numeric_input_and_absent_divergence_are_rendered_from_the_comparison() {
    let mut comparison = fixture();
    comparison.seed = i64::MIN;
    comparison.checkpoint_turn = 29;
    comparison.outcome_a.stats.speed = i32::MIN;
    comparison.outcome_b.stats.wit = i32::MAX;
    comparison
        .outcome_a
        .completed_races
        .extend(["second".into(), "third".into()]);
    comparison
        .outcome_b
        .learned_skills
        .push("another-skill".into());
    comparison.first_divergence = None;
    let report = render_markdown(&comparison);
    assert_topology(&report);
    let visible = inline_text(&report);
    assert!(visible.contains("seed -9223372036854775808, turn 29"));
    assert!(visible.contains("Spd -2147483648 / Sta 102"));
    assert!(visible.contains("Gut 204 / Wit 2147483647"));
    assert!(visible.contains("3 (race-0, second, third)"));
    assert!(visible.contains("2: skill-100, another-skill"));
    assert!(visible.contains("No divergence: the recorded timelines are identical."));
    assert!(!visible.contains("- Step 1"));
}

#[test]
fn the_report_uses_display_labels_without_substituting_hidden_identifiers() {
    let report = inline_text(&render_markdown(&fixture()));
    for hidden in [
        "hidden-branch-a",
        "hidden-branch-b",
        "hidden-action-a",
        "hidden-action-b",
        "hidden-factor-id",
        "unrendered-timeline-mood-a",
        "unrendered-timeline-mood-b",
    ] {
        assert!(
            !report.contains(hidden),
            "unexpected hidden field substituted: {hidden}"
        );
    }
    for display in [
        "action-A-0",
        "action-B-1",
        "Take a rest",
        "Train speed",
        "spark-0",
        "spark-100",
    ] {
        assert!(report.contains(display), "display field missing: {display}");
    }
}
