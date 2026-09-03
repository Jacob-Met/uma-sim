//! Run snapshot encode/decode for save/resume.

use crate::state::{CareerState, RunMeta, SimSettings};
use serde::{Deserialize, Serialize};

/// Serializable run checkpoint matching Kotlin `RunSnapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSnapshot {
    pub meta: RunMeta,
    pub settings: SimSettings,
    pub state: CareerState,
    #[serde(rename = "rngSeed")]
    pub rng_seed: i64,
    #[serde(rename = "rngCalls")]
    pub rng_calls: u32,
}

pub struct RunSnapshotCodec;

impl RunSnapshotCodec {
    pub fn encode(snapshot: &RunSnapshot) -> String {
        serde_json::to_string_pretty(snapshot).unwrap_or_default()
    }

    pub fn decode(raw: &str) -> Result<RunSnapshot, serde_json::Error> {
        serde_json::from_str(raw)
    }
}
