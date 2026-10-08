import { useEffect, useId, useState } from "react";
import type { CareerState } from "../api/types";
import { parseSkillCatalog, skillRows, type SkillCatalog, type SkillRow } from "./skillsView";
import "../styles/skills.css";

function SkillList({ title, rows, empty }: { title: string; rows: SkillRow[]; empty: string }) {
  return (
    <section className="skills-group" aria-label={title}>
      <h3>{title} <span className="skills-count">{rows.length}</span></h3>
      {rows.length === 0 ? <p className="hint">{empty}</p> : (
        <ul className="skills-list" tabIndex={0} aria-label={title}>
          {rows.map((row, index) => (
            <li key={row.id + ":" + index}>
              <div className="skills-row-title">
                <strong>{row.name ?? row.id}</strong>
                {row.level !== undefined && <span className="skills-level">Hint level {row.level}</span>}
              </div>
              {row.name && row.name !== row.id && <code className="skills-id">{row.id}</code>}
              {row.kind === "skill" && !row.name && (
                <span className="skills-missing-name">Name unavailable</span>
              )}
              {row.description && <p className="skills-description">{row.description}</p>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export function SkillsPanel({ state }: { state: CareerState }) {
  const titleId = useId();
  const [catalog, setCatalog] = useState<SkillCatalog>(() => new Map());
  const [status, setStatus] = useState<"loading" | "ready" | "unavailable">("loading");
  const [retry, setRetry] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setStatus("loading");
    void fetch("/v1/catalog/skills", { signal: controller.signal })
      .then(async (response) => {
        if (!response.ok) throw new Error("Skill catalog request failed");
        return parseSkillCatalog(await response.json());
      })
      .then((items) => {
        if (controller.signal.aborted) return;
        setCatalog(items);
        setStatus("ready");
      })
      .catch(() => {
        if (!controller.signal.aborted) setStatus("unavailable");
      });
    return () => controller.abort();
  }, [retry]);

  const rows = skillRows(state.learnedSkillIds ?? [], state.hintLevels ?? {}, catalog);

  return (
    <section className="card skills-panel" aria-labelledby={titleId}>
      <h2 id={titleId}>Skills &amp; hints</h2>
      <p className="hint">Skills and hint levels retained in this career.</p>
      {status === "loading" && <p className="hint" role="status">Loading skill names…</p>}
      {status === "unavailable" && (
        <div className="skills-catalog-status" role="status">
          <p className="hint">Skill names are unavailable. Retained IDs and levels are shown.</p>
          <button type="button" onClick={() => setRetry((value) => value + 1)}>Retry names</button>
        </div>
      )}
      <SkillList title="Learned skills" rows={rows.learned} empty="No learned skills recorded." />
      <SkillList title="Skill hints" rows={rows.skillHints} empty="No skill hints recorded." />
      <SkillList title="Training & other hints" rows={rows.otherHints} empty="No other hints recorded." />
    </section>
  );
}
