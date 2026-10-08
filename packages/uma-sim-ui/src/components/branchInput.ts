import type { ActionOverride } from "../api/types";

export interface BranchInputIssue {
  field: "overrides" | "maxActions";
  message: string;
  line?: number;
}

export type BranchDraftValidation =
  | { ok: true; maxActions: number; overrides: ActionOverride[] }
  | { ok: false; issues: BranchInputIssue[] };

/**
 * Validate the complete authored draft before submitting any branch request.
 * Turns fit the existing engine's i32 representation; legality of an action
 * at that turn still belongs to the simulator and its rejection trace.
 */
export function validateBranchDraft(
  overridesText: string,
  maxActionsText: string,
): BranchDraftValidation {
  const issues: BranchInputIssue[] = [];
  const overrides: ActionOverride[] = [];
  const firstLineByTurn = new Map<number, number>();
  const limitText = maxActionsText.trim();
  const maxActions = Number(limitText);
  if (!/^\d+$/.test(limitText) || maxActions < 1 || maxActions > 500) {
    issues.push({
      field: "maxActions",
      message: "Max actions must be a whole number from 1 through 500.",
    });
  }

  overridesText.split("\n").forEach((raw, index) => {
    const text = raw.trim();
    if (!text) return;
    const line = index + 1;
    const colon = text.indexOf(":");
    const reject = (message: string) => {
      issues.push({ field: "overrides", line, message: `Line ${line}: ${message}` });
    };
    if (colon < 0) {
      reject("use turn:actionId, for example 8:rest.");
      return;
    }

    const turnText = text.slice(0, colon).trim();
    const turn = Number(turnText);
    if (!/^\d+$/.test(turnText) || !Number.isSafeInteger(turn) || turn > 2147483647) {
      reject("turn must be a whole number from 0 through 2147483647.");
      return;
    }

    const actionId = text.slice(colon + 1).trim();
    if (!actionId) {
      reject("enter an action ID after the colon.");
      return;
    }

    const firstLine = firstLineByTurn.get(turn);
    if (firstLine !== undefined) {
      reject(`turn ${turn} is already set on line ${firstLine}; keep one override per turn.`);
      return;
    }
    firstLineByTurn.set(turn, line);
    overrides.push({ turn, actionId });
  });

  return issues.length > 0
    ? { ok: false, issues }
    : { ok: true, maxActions, overrides };
}
