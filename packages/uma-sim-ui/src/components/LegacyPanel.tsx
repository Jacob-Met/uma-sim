import type { CatalogItem } from "../api/types";
import { useMemo } from "react";

/** One spark slot on an ancestor. */
export interface SparkSlot {
  factorId: string;
  stars: number; // 1–3 (meaningful for blue)
}

export interface AncestorSparks {
  /** Character English name (optional). */
  uma: string;
  blue: SparkSlot;
  pink: SparkSlot; // red/pink aptitude
  white: SparkSlot; // skill
  green: SparkSlot; // scenario
  race: SparkSlot; // white race factor
}

export interface LegacyTree {
  parentA: AncestorSparks;
  gpA1: AncestorSparks;
  gpA2: AncestorSparks;
  parentB: AncestorSparks;
  gpB1: AncestorSparks;
  gpB2: AncestorSparks;
}

export function emptyAncestor(): AncestorSparks {
  return {
    uma: "",
    blue: { factorId: "", stars: 3 },
    pink: { factorId: "", stars: 3 },
    white: { factorId: "", stars: 1 },
    green: { factorId: "", stars: 1 },
    race: { factorId: "", stars: 1 },
  };
}

export function emptyLegacyTree(): LegacyTree {
  return {
    parentA: emptyAncestor(),
    gpA1: emptyAncestor(),
    gpA2: emptyAncestor(),
    parentB: emptyAncestor(),
    gpB1: emptyAncestor(),
    gpB2: emptyAncestor(),
  };
}

/** Flatten tree → engine `factor:id@stars` list (only filled sparks). */
export function flattenLegacyFactors(tree: LegacyTree): string[] {
  const out: string[] = [];
  for (const node of Object.values(tree)) {
    for (const slot of [node.blue, node.pink, node.white, node.green, node.race]) {
      if (!slot.factorId) continue;
      const stars = Math.max(1, Math.min(3, slot.stars || 1));
      out.push(`${slot.factorId}@${stars}`);
    }
  }
  return out;
}

export function legacyParentNames(tree: LegacyTree): string[] {
  return [tree.parentA.uma, tree.parentB.uma].filter(Boolean);
}

function factorsOf(
  factors: CatalogItem[],
  kind: string,
): CatalogItem[] {
  return factors.filter((f) => (f.kind ?? "").toLowerCase() === kind);
}

interface AncestorEditorProps {
  label: string;
  value: AncestorSparks;
  trainees: CatalogItem[];
  /** Names blocked for this uma picker (e.g. career trainee). */
  blockedUmaNames: string[];
  blues: CatalogItem[];
  pinks: CatalogItem[];
  whites: CatalogItem[];
  greens: CatalogItem[];
  races: CatalogItem[];
  busy: boolean;
  onChange: (next: AncestorSparks) => void;
}

function AncestorEditor({
  label,
  value,
  trainees,
  blockedUmaNames,
  blues,
  pinks,
  whites,
  greens,
  races,
  busy,
  onChange,
}: AncestorEditorProps) {
  const blocked = new Set(blockedUmaNames.map((n) => n.toLowerCase()));
  const umaOptions = trainees.filter((t) => !blocked.has(t.name.toLowerCase()));

  function setSpark(
    key: "blue" | "pink" | "white" | "green" | "race",
    patch: Partial<SparkSlot>,
  ) {
    onChange({ ...value, [key]: { ...value[key], ...patch } });
  }

  return (
    <div className="legacy-node">
      <div className="legacy-node-title">{label}</div>
      <div className="field">
        <label>Uma (optional)</label>
        <select
          disabled={busy}
          value={value.uma}
          onChange={(e) => onChange({ ...value, uma: e.target.value })}
        >
          <option value="">— none —</option>
          {umaOptions.map((t) => (
            <option key={t.id} value={t.name}>
              {t.name}
              {t.nameJa ? ` / ${t.nameJa}` : ""}
            </option>
          ))}
        </select>
      </div>
      <div className="legacy-sparks">
        <SparkPick
          label="Blue (stat)"
          kindClass="spark-blue"
          options={blues}
          slot={value.blue}
          busy={busy}
          showStars
          onChange={(s) => setSpark("blue", s)}
        />
        <SparkPick
          label="Pink/Red (aptitude)"
          kindClass="spark-pink"
          options={pinks}
          slot={value.pink}
          busy={busy}
          showStars
          onChange={(s) => setSpark("pink", s)}
        />
        <SparkPick
          label="White (skill)"
          kindClass="spark-white"
          options={whites}
          slot={value.white}
          busy={busy}
          showStars
          onChange={(s) => setSpark("white", s)}
        />
        <SparkPick
          label="Green (scenario)"
          kindClass="spark-green"
          options={greens}
          slot={value.green}
          busy={busy}
          showStars
          onChange={(s) => setSpark("green", s)}
        />
        <SparkPick
          label="Race (white)"
          kindClass="spark-race"
          options={races}
          slot={value.race}
          busy={busy}
          showStars
          onChange={(s) => setSpark("race", s)}
        />
      </div>
    </div>
  );
}

function SparkPick({
  label,
  kindClass,
  options,
  slot,
  busy,
  showStars,
  onChange,
}: {
  label: string;
  kindClass: string;
  options: CatalogItem[];
  slot: SparkSlot;
  busy: boolean;
  showStars?: boolean;
  onChange: (s: Partial<SparkSlot>) => void;
}) {
  return (
    <div className={`spark-row ${kindClass}`}>
      <span className="spark-label">{label}</span>
      <select
        disabled={busy}
        value={slot.factorId}
        onChange={(e) => onChange({ factorId: e.target.value })}
      >
        <option value="">—</option>
        {options.map((f) => (
          <option key={f.id} value={f.id}>
            {f.name}
          </option>
        ))}
      </select>
      {showStars && (
        <select
          disabled={busy || !slot.factorId}
          value={slot.stars}
          onChange={(e) => onChange({ stars: Number(e.target.value) })}
          title="Stars"
        >
          <option value={1}>★</option>
          <option value={2}>★★</option>
          <option value={3}>★★★</option>
        </select>
      )}
    </div>
  );
}

interface Props {
  enabled: boolean;
  tree: LegacyTree;
  trainees: CatalogItem[];
  factors: CatalogItem[];
  /** Career trainee — cannot be a direct parent. */
  traineeName: string;
  busy: boolean;
  onEnabled: (v: boolean) => void;
  onChange: (tree: LegacyTree) => void;
}

export function LegacyPanel({
  enabled,
  tree,
  trainees,
  factors,
  traineeName,
  busy,
  onEnabled,
  onChange,
}: Props) {
  const blues = useMemo(() => factorsOf(factors, "blue"), [factors]);
  const pinks = useMemo(() => factorsOf(factors, "pink"), [factors]);
  const whites = useMemo(() => factorsOf(factors, "skill"), [factors]);
  const greens = useMemo(() => factorsOf(factors, "scenario"), [factors]);
  const races = useMemo(() => factorsOf(factors, "race"), [factors]);
  const blockedParents = traineeName ? [traineeName] : [];

  function patch(key: keyof LegacyTree, next: AncestorSparks) {
    onChange({ ...tree, [key]: next });
  }

  return (
    <div className="card" style={{ marginTop: "0.85rem" }}>
      <label className="legacy-toggle">
        <input
          type="checkbox"
          checked={enabled}
          disabled={busy}
          onChange={(e) => onEnabled(e.target.checked)}
        />
        <span>
          <strong>Legacy / inheritance</strong>{" "}
          <span className="chip">optional</span>
        </span>
      </label>
      <p className="legacy-help">
        Off by default. Blue ★ add starting stats (5/12/21); pink/red ★ sum
        raises matching aptitudes (capped at A). Direct parents cannot match
        your trainee; parents <em>may</em> match support cards.
      </p>
      {enabled && (
        <div className="legacy-tree">
          <div className="legacy-branch">
            <h3>Parent A</h3>
            <AncestorEditor
              label="Direct parent A"
              value={tree.parentA}
              trainees={trainees}
              blockedUmaNames={blockedParents}
              blues={blues}
              pinks={pinks}
              whites={whites}
              greens={greens}
              races={races}
              busy={busy}
              onChange={(n) => patch("parentA", n)}
            />
            <AncestorEditor
              label="Grandparent A1"
              value={tree.gpA1}
              trainees={trainees}
              blockedUmaNames={[]}
              blues={blues}
              pinks={pinks}
              whites={whites}
              greens={greens}
              races={races}
              busy={busy}
              onChange={(n) => patch("gpA1", n)}
            />
            <AncestorEditor
              label="Grandparent A2"
              value={tree.gpA2}
              trainees={trainees}
              blockedUmaNames={[]}
              blues={blues}
              pinks={pinks}
              whites={whites}
              greens={greens}
              races={races}
              busy={busy}
              onChange={(n) => patch("gpA2", n)}
            />
          </div>
          <div className="legacy-branch">
            <h3>Parent B</h3>
            <AncestorEditor
              label="Direct parent B"
              value={tree.parentB}
              trainees={trainees}
              blockedUmaNames={blockedParents}
              blues={blues}
              pinks={pinks}
              whites={whites}
              greens={greens}
              races={races}
              busy={busy}
              onChange={(n) => patch("parentB", n)}
            />
            <AncestorEditor
              label="Grandparent B1"
              value={tree.gpB1}
              trainees={trainees}
              blockedUmaNames={[]}
              blues={blues}
              pinks={pinks}
              whites={whites}
              greens={greens}
              races={races}
              busy={busy}
              onChange={(n) => patch("gpB1", n)}
            />
            <AncestorEditor
              label="Grandparent B2"
              value={tree.gpB2}
              trainees={trainees}
              blockedUmaNames={[]}
              blues={blues}
              pinks={pinks}
              whites={whites}
              greens={greens}
              races={races}
              busy={busy}
              onChange={(n) => patch("gpB2", n)}
            />
          </div>
        </div>
      )}
    </div>
  );
}
