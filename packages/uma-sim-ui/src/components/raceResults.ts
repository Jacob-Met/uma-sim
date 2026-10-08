// SPDX-License-Identifier: GPL-3.0-only

interface RetainedRace {
  /** Position in the supplied career log, including non-race entries. */
  logIndex: number;
  summary: string;
}

export interface PhysicsRaceResult extends RetainedRace {
  kind: "physics";
  raceId: string;
  place: number;
  placeLabel: string;
  fieldSize: number;
  /** Keep the precision present in the record. */
  finishTime: string;
  fanGain: number;
  courseId: string;
  seed: string;
  marginToWinner: string;
  marginAhead: string;
}

export interface FanOnlyRaceResult extends RetainedRace {
  kind: "fan-only";
  raceId: string;
  fanGain: number;
}

export interface UnknownRaceResult extends RetainedRace {
  kind: "unknown";
}

export type RaceResult = PhysicsRaceResult | FanOnlyRaceResult | UnknownRaceResult;

// These are the retained engine::do_race records, not the action narration.
const PHYSICS = /^Race (\S+) (\d+)(st|nd|rd|th) \+(\d+) fans \[physics t=(\S+)s course=(\d+) seed=(\d+) field=(\d+) margin_win=(\S+)s margin_ahead=(\S+)s\]$/;
const FAN_ONLY = /^Race (\S+) \+(\d+) fans$/;
const DECIMAL = /^\d+(?:\.\d+)?$/;

function nonnegativeInteger(value: string): number | null {
  const number = Number(value);
  return Number.isSafeInteger(number) && number >= 0 ? number : null;
}

function nonnegativeDecimal(value: string): number | null {
  const number = Number(value);
  return DECIMAL.test(value) && Number.isFinite(number) && number >= 0 ? number : null;
}

function ordinalSuffix(place: number): string {
  if (place % 100 >= 11 && place % 100 <= 13) return "th";
  return ({ 1: "st", 2: "nd", 3: "rd" } as Record<number, string>)[place % 10] ?? "th";
}

function parseRecord(summary: string, logIndex: number): RaceResult | null {
  if (!/^Race(?:\s|$)/u.test(summary)) return null;
  const unknown: UnknownRaceResult = { kind: "unknown", logIndex, summary };
  if (/[\r\n]/.test(summary)) return unknown;

  const physics = PHYSICS.exec(summary);
  if (physics) {
    const [, raceId, placeText, suffix, fansText, finishTime, courseId, seed,
      fieldText, marginToWinner, marginAhead] = physics;
    const place = nonnegativeInteger(placeText);
    const fieldSize = nonnegativeInteger(fieldText);
    const fanGain = nonnegativeInteger(fansText);
    const finish = nonnegativeDecimal(finishTime);
    const winnerGap = nonnegativeDecimal(marginToWinner);
    const aheadGap = nonnegativeDecimal(marginAhead);
    const courseNumber = nonnegativeInteger(courseId);
    const seedNumber = nonnegativeInteger(seed);
    if (place === null || place < 1 || fieldSize === null || fieldSize < place ||
        fanGain === null || finish === null || finish <= 0 || winnerGap === null ||
        aheadGap === null || courseNumber === null || courseNumber > 0xffffffff ||
        seedNumber === null || seedNumber > 0xffffffff || suffix !== ordinalSuffix(place)) {
      return unknown;
    }
    return {
      kind: "physics", logIndex, summary, raceId, place,
      placeLabel: `${placeText}${suffix}`, fieldSize, finishTime, fanGain,
      courseId, seed, marginToWinner, marginAhead,
    };
  }

  const fanOnly = FAN_ONLY.exec(summary);
  if (fanOnly) {
    const fanGain = nonnegativeInteger(fanOnly[2]);
    if (fanGain !== null) return { kind: "fan-only", logIndex, summary, raceId: fanOnly[1], fanGain };
  }
  return unknown;
}

/** Retained occurrences in source order; duplicate race IDs are separate records. */
export function parseRaceResults(lines: readonly string[]): RaceResult[] {
  const records: RaceResult[] = [];
  lines.forEach((line, index) => {
    const record = parseRecord(line, index);
    if (record) records.push(record);
  });
  return records;
}

/** An unrecognized newest race remains newest instead of exposing an older result. */
export function parseRaceResult(lines: readonly string[]): RaceResult | null {
  for (let index = lines.length - 1; index >= 0; index--) {
    const record = parseRecord(lines[index], index);
    if (record) return record;
  }
  return null;
}
