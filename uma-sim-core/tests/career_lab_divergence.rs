//! Regression tests for a branch that ends before its comparison partner.
//! Authored public results keep their common prefix identical, so the first
//! missing recorded step is the cause of divergence rather than a game rule.

use serde_json::{json, Value};
use uma_sim_core::career_lab::{compare_branches, render_json, LabResult};

fn branch(id: &str, steps: usize) -> LabResult {
    let stats = json!({"speed": 100, "stamina": 110, "power": 120, "guts": 130, "wit": 140});
    let timeline: Vec<Value> = (0..steps)
        .map(|index| {
            json!({
                "stepIndex": index, "turn": 20 + index,
                "dateLabel": format!("recorded turn {}", 20 + index), "phase": "FREE",
                "actionId": "rest", "actionLabel": "Take a rest",
                "overrideApplied": false, "overrideRejected": false,
                "rngCallsBefore": index * 4, "rngCallsAfter": (index + 1) * 4,
                "energy": 40 + index, "mood": "NORMAL", "fans": 100,
                "skillPoints": 25, "stats": stats,
                "newRaces": [], "totalRaces": 0, "totalSkills": 0
            })
        })
        .collect();
    serde_json::from_value(json!({
        "id": id, "name": format!("Branch {id}"), "checkpointName": "shared-checkpoint",
        "checkpointTurn": 20, "seed": 17, "scenarioId": "ura", "traineeName": "Fixture",
        "config": {"policy": "bot", "maxActions": 100, "overrides": []},
        "startedAt": "2026-10-08T10:00:00Z", "timeline": timeline,
        "outcome": {
            "steps": steps, "finalTurn": 20 + steps, "careerComplete": false,
            "completedRaces": [], "stats": stats, "energy": 40 + steps,
            "mood": "NORMAL", "fans": 100, "skillPoints": 25, "learnedSkills": [],
            "sparks": [], "scenarioResources": {}, "totalRngCalls": steps * 4,
            "telemetryRecords": steps
        }
    }))
    .expect("complete public branch fixture")
}

fn assert_ended(a: &LabResult, b: &LabResult, side: &str, recorded_steps: usize) {
    let comparison = compare_branches(a, b);
    let divergence = comparison.first_divergence.as_ref().unwrap();
    let expected_note =
        format!("Branch {side} ended after {recorded_steps} steps while the other continued.");
    assert_eq!(divergence.note, expected_note);
    assert_eq!(divergence.step_index, recorded_steps);
    assert_eq!(divergence.kind, "outcome");
    assert_eq!(
        comparison.aligned.len(),
        a.timeline.len().max(b.timeline.len())
    );
    assert!(comparison.aligned[..recorded_steps]
        .iter()
        .all(|row| row.same_action && row.same_outcome));

    let row = &comparison.aligned[recorded_steps];
    let continuing = if side == "A" {
        assert!(row.turn_a.is_none());
        assert!(row.action_a.is_none());
        assert!(row.label_a.is_none());
        assert_eq!(divergence.action_a, "");
        assert_eq!(divergence.label_a, "");
        assert_eq!(divergence.rng_calls_a_after, 0);
        assert_eq!(row.action_b.as_deref(), Some("rest"));
        &b.timeline[recorded_steps]
    } else {
        assert!(row.turn_b.is_none());
        assert!(row.action_b.is_none());
        assert!(row.label_b.is_none());
        assert_eq!(divergence.action_b, "");
        assert_eq!(divergence.label_b, "");
        assert_eq!(divergence.rng_calls_b_after, 0);
        assert_eq!(row.action_a.as_deref(), Some("rest"));
        &a.timeline[recorded_steps]
    };
    assert_eq!(divergence.turn, continuing.turn);
    assert_eq!(divergence.date_label, continuing.date_label);
    assert_eq!(divergence.phase, continuing.phase);
    assert!(!row.same_action && !row.same_outcome);

    let json: Value = serde_json::from_str(&render_json(&comparison)).unwrap();
    assert_eq!(json["firstDivergence"]["note"], expected_note);
    assert_eq!(json["firstDivergence"]["stepIndex"], recorded_steps);
    assert_eq!(json["outcomeA"]["steps"], a.outcome.steps);
    assert_eq!(json["outcomeB"]["steps"], b.outcome.steps);
}

#[test]
fn a_ended_note_counts_a_recorded_steps_when_b_continues() {
    // Changing only the continuing length must not change the ended count;
    // changing the shorter length must change the report.
    for (a_steps, b_steps) in [(2, 3), (2, 7), (5, 8)] {
        assert_ended(&branch("a", a_steps), &branch("b", b_steps), "A", a_steps);
    }
}

#[test]
fn b_ended_note_counts_b_recorded_steps_when_a_continues() {
    for (a_steps, b_steps) in [(3, 2), (7, 2), (8, 5)] {
        assert_ended(&branch("a", a_steps), &branch("b", b_steps), "B", b_steps);
    }
}

#[test]
fn an_empty_branch_ended_after_zero_recorded_steps_in_either_position() {
    assert_ended(&branch("a", 0), &branch("b", 3), "A", 0);
    assert_ended(&branch("a", 3), &branch("b", 0), "B", 0);
}

#[test]
fn ending_uses_the_recorded_timeline_even_if_summary_counts_are_stale() {
    for (a_steps, b_steps, side) in [(2, 4, "A"), (4, 2, "B")] {
        let mut a = branch("a", a_steps);
        let mut b = branch("b", b_steps);
        // Persisted public results may contain old summary metadata. Alignment
        // and the ending explanation must describe the actual recorded steps.
        a.outcome.steps = 901;
        b.outcome.steps = 902;
        assert_ended(&a, &b, side, 2);
    }
}

#[test]
fn equal_length_timelines_keep_existing_divergence_behavior() {
    for steps in [0, 1, 4] {
        let comparison = compare_branches(&branch("a", steps), &branch("b", steps));
        assert!(comparison.first_divergence.is_none());
        assert_eq!(comparison.aligned.len(), steps);
        assert!(comparison
            .aligned
            .iter()
            .all(|row| row.same_action && row.same_outcome));
    }

    for kind in ["decision", "outcome"] {
        let a = branch("a", 3);
        let mut b = branch("b", 3);
        if kind == "decision" {
            b.timeline[1].action_id = "train_speed".into();
            b.timeline[1].action_label = "Train speed".into();
        } else {
            b.timeline[1].energy += 1;
        }
        let comparison = compare_branches(&a, &b);
        let divergence = comparison.first_divergence.unwrap();
        assert_eq!(divergence.step_index, 1);
        assert_eq!(divergence.kind, kind);
        assert!(!divergence.note.contains("ended after"));
    }
}

#[test]
fn earlier_decision_or_outcome_remains_first_when_a_branch_later_ends() {
    for (a_steps, b_steps) in [(2, 4), (4, 2)] {
        for kind in ["decision", "outcome"] {
            let a = branch("a", a_steps);
            let mut b = branch("b", b_steps);
            if kind == "decision" {
                b.timeline[0].action_id = "train_speed".into();
                b.timeline[0].action_label = "Train speed".into();
            } else {
                b.timeline[0].energy += 1;
            }
            let comparison = compare_branches(&a, &b);
            assert_eq!(comparison.aligned.len(), 4);
            let divergence = comparison.first_divergence.unwrap();
            assert_eq!(divergence.step_index, 0);
            assert_eq!(divergence.kind, kind);
            assert!(!divergence.note.contains("ended after"));
        }
    }
}
