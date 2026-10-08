import { useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { StatsPanel } from '../../uma-sim-ui/src/components/StatsPanel';
import { ChoicePanel } from '../../uma-sim-ui/src/components/ChoicePanel';
import type { CareerState, Choice, RunSnapshot, StepResponse } from '../../uma-sim-ui/src/api/types';
import '../../uma-sim-ui/src/styles/app.css';
import './demo.css';
import fixtureJSON from '../fixture.json';

type RecordedOutcome = { actionId: string; response: StepResponse };
type Checkpoint = {
  id: string; seed: string; trainingActionsBefore: number;
  preparation: string[]; snapshot: RunSnapshot; choices: Choice[]; outcomes: RecordedOutcome[];
};
type Fixture = {
  schema: string; source: { repository: string; commit: string; tree: string; binarySha256: string };
  setup: { scenario: string; trainee: string; raceModel: string; policy: string };
  seeds: string[]; checkpoints: Checkpoint[];
};
const fixture = fixtureJSON as unknown as Fixture;
const stats = ['speed', 'stamina', 'power', 'guts', 'wit'] as const;
const title = (text: string) => text.charAt(0).toUpperCase() + text.slice(1).toLowerCase();
const change = (before: number, after: number) => after > before ? `+${after - before}` : String(after - before);
const conditions = (state: CareerState) => state.statuses.length ? state.statuses.join(', ') : 'None';
const checkpointLabel = (checkpoint: Checkpoint) => `${checkpoint.trainingActionsBefore === 0 ? 'First training turn' : `After ${checkpoint.trainingActionsBefore} training choices`} · turn ${checkpoint.snapshot.state.turn} · ${checkpoint.snapshot.state.energy} energy`;

function Changes({ before, after }: { before: CareerState; after: CareerState }) {
  const rows = [
    ...stats.map(key => ({ key, label: title(key), before: before.stats[key], after: after.stats[key] })),
    { key: 'energy', label: 'Energy', before: before.energy, after: after.energy },
    { key: 'skillPoints', label: 'Skill points', before: before.skillPoints, after: after.skillPoints },
    { key: 'fans', label: 'Fans', before: before.fans, after: after.fans },
  ];
  return <table className="delta-table" data-testid="changes">
    <caption>What changed from the checkpoint</caption>
    <thead><tr><th scope="col">Measure</th><th scope="col">Before</th><th scope="col">After</th><th scope="col">Change</th></tr></thead>
    <tbody>{rows.map(row => <tr key={row.key} data-measure={row.key}>
      <th scope="row">{row.label}</th><td>{row.before}</td><td>{row.after}</td>
      <td className={row.after > row.before ? 'gain' : row.after < row.before ? 'loss' : 'unchanged'}>{change(row.before, row.after)}</td>
    </tr>)}</tbody>
  </table>;
}

function App() {
  const [seed, setSeed] = useState('7');
  const [depth, setDepth] = useState(0);
  const [actionId, setActionId] = useState<string | null>(null);
  const [announcement, setAnnouncement] = useState('Choose a recorded action to compare its outcome.');
  const [replayCount, setReplayCount] = useState(0);
  const seedCheckpoints = useMemo(() => fixture.checkpoints.filter(checkpoint => checkpoint.seed === seed), [seed]);
  const checkpoint = seedCheckpoints.find(candidate => candidate.trainingActionsBefore === depth)!;
  const recordedChoices = useMemo(() => checkpoint.choices.filter(choice => checkpoint.outcomes.some(outcome => outcome.actionId === choice.id)), [checkpoint]);
  const selected = checkpoint.outcomes.find(outcome => outcome.actionId === actionId);
  const actionLabel = recordedChoices.find(choice => choice.id === actionId)?.label;
  const before = checkpoint.snapshot.state;
  const after = selected?.response.state.state;

  function reset(message = 'Comparison reset. Choose an action from the unchanged checkpoint.') {
    setActionId(null); setReplayCount(0); setAnnouncement(message);
  }
  function choose(id: string) {
    const outcome = checkpoint.outcomes.find(candidate => candidate.actionId === id);
    if (!outcome) return;
    const label = recordedChoices.find(choice => choice.id === id)!.label;
    setActionId(id); setReplayCount(count => count + 1);
    setAnnouncement(`${label}: recorded outcome from seed ${seed}, turn ${before.turn}. Result turn ${outcome.response.state.state.turn}, energy ${outcome.response.state.state.energy}.`);
  }
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      const target = event.target;
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey || event.repeat || (target instanceof HTMLElement && (target.isContentEditable || target.closest('input, select, textarea')))) return;
      const index = /^[1-7]$/.test(event.key) ? Number(event.key) - 1 : -1;
      if (recordedChoices[index]) { event.preventDefault(); choose(recordedChoices[index].id); }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [checkpoint, recordedChoices]);

  return <main className="demo-app">
    <a className="skip-link" href="#recorded-choices">Skip to choices</a>
    <header className="demo-header">
      <div><p className="eyebrow">uma-sim / training turn</p><h1>Compare a training turn</h1></div>
      <span className="mode-badge">Unofficial · Recorded fixture</span>
    </header>
    <p className="intro">Pick a seed and checkpoint, then compare training and recovery. Every choice starts from the <strong>same saved state</strong> and shows an outcome recorded from the native simulator.</p>

    <section className="card setup-card" aria-labelledby="setup-heading">
      <div className="setup-title"><h2 id="setup-heading">Choose your checkpoint</h2><span>{fixture.setup.trainee} · {fixture.setup.scenario.toUpperCase()}</span></div>
      <div className="demo-fields">
        <label className="field"><span>Seed</span><select value={seed} onChange={event => {
          if (!fixture.seeds.includes(event.target.value)) return;
          setSeed(event.target.value); reset(`Seed ${event.target.value} selected. The comparison starts from its recorded checkpoint.`);
        }}>{fixture.seeds.map(value => <option key={value} value={value}>{value}</option>)}</select></label>
        <label className="field checkpoint-field"><span>Checkpoint</span><select value={depth} onChange={event => {
          const next = Number(event.target.value);
          if (!seedCheckpoints.some(candidate => candidate.trainingActionsBefore === next)) return;
          setDepth(next); reset('Checkpoint changed. Choose a recorded action to compare.');
        }}>{seedCheckpoints.map(candidate => <option key={candidate.id} value={candidate.trainingActionsBefore}>{checkpointLabel(candidate)}</option>)}</select></label>
        <button className="reset-button" onClick={() => reset()} disabled={!selected}>Reset comparison</button>
      </div>
      <div className="checkpoint-summary" data-testid="checkpoint-summary">
        <span>Turn <strong>{before.turn}</strong></span><span>Energy <strong>{before.energy}/{before.maxEnergy}</strong></span>
        <span>Mood <strong>{title(before.mood)}</strong></span><span>Conditions <strong>{conditions(before)}</strong></span>
      </div>
    </section>

    <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">{announcement} {replayCount > 0 ? `Comparisons viewed at this checkpoint: ${replayCount}.` : ''}</p>
    <div className="demo-layout">
      <section className="choices-column" id="recorded-choices" aria-label="Recorded training and recovery choices">
        <ChoicePanel choices={recordedChoices} state={before} busy={false} onChoose={choose} />
        <p className="choice-note">Use the buttons or keys <kbd>1</kbd>–<kbd>7</kbd>. Each choice compares an alternative from turn {before.turn}; choices do not build on the previous result.</p>
        <details className="provenance"><summary>How this checkpoint was reached</summary><p>Start seed {seed} with the physics race model and default policy. Replay these native action IDs in order:</p><ol data-testid="preparation">{checkpoint.preparation.map((action, index) => <li key={`${index}-${action}`}><code>{action}</code></li>)}</ol><p>{checkpoint.snapshot.rngCalls} RNG calls before the recorded choice.</p></details>
      </section>

      <section className={`card outcome-card ${selected ? 'has-result' : ''}`} aria-labelledby="outcome-heading" data-testid="outcome" data-action={actionId ?? ''}>
        <p className="eyebrow">Recorded outcome</p>
        <h2 id="outcome-heading">{selected ? actionLabel : 'Choose an action to see its outcome'}</h2>
        {selected && after ? <>
          <p className="result-context">Seed {seed} · turn {before.turn} → {after.turn} · {after.phase}</p>
          <Changes before={before} after={after} />
          <dl className="conditions" data-testid="condition-changes"><div><dt>Mood</dt><dd>{title(before.mood)} → {title(after.mood)}</dd></div><div><dt>Conditions</dt><dd>{conditions(before)} → {conditions(after)}</dd></div></dl>
          <details className="native-response" open><summary>Original simulator response</summary><pre data-testid="native-response">{selected.response.text}</pre></details>
          <details className="next-choices"><summary>Next choices reported by the simulator</summary><p>These are part of the recorded result. This demo compares one action at a time from the selected checkpoint.</p><ul data-testid="next-choices">{selected.response.choices.map(choice => <li key={choice.id}>{choice.label} <code>{choice.id}</code></li>)}</ul><p>Career ended: {selected.response.careerEnded ? 'Yes' : 'No'} · RNG calls after: {selected.response.state.rngCalls}</p></details>
          <button className="replay-button" onClick={() => choose(selected.actionId)}>Replay this recording</button>
        </> : <div className="empty-state"><span className="empty-mark" aria-hidden="true">↗</span><p>Try training at full energy, then switch to a later checkpoint and compare rest.</p><p>The nine checkpoints include low energy and injury. The native response explains what actually happened.</p></div>}
      </section>

      <section className="status-column" aria-label={selected ? 'Status after the recorded action' : 'Checkpoint status'} data-testid="status-panel">
        <div className="status-label">{selected ? 'After the recorded action' : 'At the checkpoint'}</div>
        <StatsPanel state={after ?? before} />
      </section>
    </div>

    <footer className="demo-footer">
      <p><strong>Recorded fixture mode.</strong> Three seeds, nine checkpoints, 63 native outcomes. The page runs offline and sends no API requests. It replays the captured results; it does not run a new simulation or continue a full career.</p>
      <details><summary>Source and reproducibility</summary><p>Simulator revision <a href={`${fixture.source.repository}/tree/${fixture.source.commit}`}><code>{fixture.source.commit}</code></a>. Captured with the <strong>{fixture.setup.raceModel}</strong> race model and <strong>{fixture.setup.policy}</strong> policy. The existing uma-sim status and choice components are reused unchanged.</p><p>The package includes the complete fixture, the capture command, and a native replay verifier. The same seed, preparation, and action reproduce the recorded state and response.</p></details>
      <p className="attribution">Unofficial fan simulator; not affiliated with Cygames. Umamusume: Pretty Derby and related trademarks belong to Cygames, Inc. · <a href={`${fixture.source.repository}/blob/${fixture.source.commit}/LICENSE`}>GPL-3.0-only</a> · <a href={`${fixture.source.repository}/blob/${fixture.source.commit}/NOTICE`}>Attribution</a></p>
    </footer>
  </main>;
}

const root = document.getElementById('root');
if (root) createRoot(root).render(<App />);
