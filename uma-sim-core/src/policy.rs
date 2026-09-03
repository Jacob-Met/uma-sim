use crate::state::{SimAction, SimActionKind, SimChoice};

pub fn default_auto_policy(choices: &[SimChoice]) -> SimAction {
    if choices.iter().any(|c| c.id == "race" && c.label.to_lowercase().contains("mandatory")) {
        return SimAction {
            kind: SimActionKind::Race,
            payload: None,
        };
    }
    if choices.iter().any(|c| c.id.starts_with("event_")) {
        let id = choices
            .iter()
            .find(|c| c.id.starts_with("event_"))
            .map(|c| c.id.clone())
            .unwrap_or_else(|| "event_0".to_string());
        return SimAction {
            kind: SimActionKind::Choose,
            payload: Some(id.trim_start_matches("event_").to_string()),
        };
    }
    if choices.iter().any(|c| c.id == "race") && choices.len() == 1 {
        return SimAction {
            kind: SimActionKind::Race,
            payload: None,
        };
    }
    if choices.iter().any(|c| c.id.starts_with("train_")) {
        return SimAction {
            kind: SimActionKind::Train,
            payload: Some("speed".to_string()),
        };
    }
    if choices.iter().any(|c| c.id == "rest") {
        return SimAction {
            kind: SimActionKind::Rest,
            payload: None,
        };
    }
    SimAction {
        kind: SimActionKind::Rest,
        payload: None,
    }
}
