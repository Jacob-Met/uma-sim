//! Approximate Markov models for other-uma lane conditions (`blocked_side`, `overtake`).
//!
//! Clean-room of umalator `ApproximateConditions.ts` + `SpecialConditions.ts`.
//! Used by lateral lane movement; skill-condition sampling for the same keywords
//! remains Erlang (`noopErlang`) to match Virtual/default fixtures.

use crate::hp::Strategy;
use crate::physics::Phase;
use crate::rng::PrandoRng;

#[derive(Clone, Copy, Debug)]
struct StartContinue {
    start_rate: f64,
    continuation_rate: f64,
}

impl StartContinue {
    fn update(self, rng: &mut PrandoRng, current: u8) -> u8 {
        let rate = if current == 0 {
            self.start_rate
        } else {
            self.continuation_rate
        };
        if rng.random() < rate {
            1
        } else {
            0
        }
    }
}

#[derive(Clone, Debug)]
struct MultiEntry {
    rates: StartContinue,
    /// `None` = fallback when no earlier predicate matches.
    kind: MultiKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MultiKind {
    BlockedOuterLane,
    BlockedEarly,
    BlockedMid,
    Fallback,
    OvertakeNige,
    OvertakeSenkou,
}

#[derive(Clone, Debug)]
pub struct ApproximateMulti {
    entries: Vec<MultiEntry>,
    pub value: u8,
}

impl ApproximateMulti {
    pub fn blocked_side() -> Self {
        Self {
            // umalator createBlockedSideCondition valueOnStart = 1
            value: 1,
            entries: vec![
                MultiEntry {
                    rates: StartContinue {
                        start_rate: 0.0,
                        continuation_rate: 0.0,
                    },
                    kind: MultiKind::BlockedOuterLane,
                },
                MultiEntry {
                    rates: StartContinue {
                        start_rate: 0.1,
                        continuation_rate: 0.85,
                    },
                    kind: MultiKind::BlockedEarly,
                },
                MultiEntry {
                    rates: StartContinue {
                        start_rate: 0.08,
                        continuation_rate: 0.75,
                    },
                    kind: MultiKind::BlockedMid,
                },
                MultiEntry {
                    rates: StartContinue {
                        start_rate: 0.07,
                        continuation_rate: 0.50,
                    },
                    kind: MultiKind::Fallback,
                },
            ],
        }
    }

    pub fn overtake() -> Self {
        Self {
            value: 0,
            entries: vec![
                MultiEntry {
                    rates: StartContinue {
                        start_rate: 0.05,
                        continuation_rate: 0.50,
                    },
                    kind: MultiKind::OvertakeNige,
                },
                MultiEntry {
                    rates: StartContinue {
                        start_rate: 0.15,
                        continuation_rate: 0.55,
                    },
                    kind: MultiKind::OvertakeSenkou,
                },
                MultiEntry {
                    rates: StartContinue {
                        start_rate: 0.20,
                        continuation_rate: 0.60,
                    },
                    kind: MultiKind::Fallback,
                },
            ],
        }
    }

    fn select_rates(
        &self,
        phase: Phase,
        strategy: Strategy,
        pos: f64,
        section_len: f64,
        current_lane: f64,
        horse_lane: f64,
    ) -> StartContinue {
        let section = if section_len > 0.0 {
            (pos / section_len).floor() as i32
        } else {
            0
        };
        let mut fallback: Option<StartContinue> = None;
        for e in &self.entries {
            let matches = match e.kind {
                MultiKind::BlockedOuterLane => {
                    (1..=3).contains(&section) && current_lane > 3.0 * horse_lane
                }
                MultiKind::BlockedEarly => matches!(phase, Phase::Opening),
                MultiKind::BlockedMid => matches!(phase, Phase::Middle),
                MultiKind::OvertakeNige => matches!(strategy, Strategy::Nige),
                MultiKind::OvertakeSenkou => matches!(strategy, Strategy::Senkou),
                MultiKind::Fallback => {
                    fallback = Some(e.rates);
                    false
                }
            };
            if matches {
                return e.rates;
            }
        }
        fallback.unwrap_or(StartContinue {
            start_rate: 0.0,
            continuation_rate: 0.0,
        })
    }

    pub fn tick(
        &mut self,
        rng: &mut PrandoRng,
        phase: Phase,
        strategy: Strategy,
        pos: f64,
        section_len: f64,
        current_lane: f64,
        horse_lane: f64,
    ) {
        let rates = self.select_rates(phase, strategy, pos, section_len, current_lane, horse_lane);
        self.value = rates.update(rng, self.value);
    }

    pub fn active(&self) -> bool {
        self.value == 1
    }
}

/// Pair of Approximate conditions + 1 Hz tick timer (umalator `conditionTimer`).
#[derive(Clone, Debug)]
pub struct SpecialConditions {
    pub blocked_side: ApproximateMulti,
    pub overtake: ApproximateMulti,
    /// Starts at -1.0; when >= 0 after frame dt accumulation, tick then reset to -1.
    pub condition_timer: f64,
    pub rng: PrandoRng,
}

impl SpecialConditions {
    pub fn new(rng: PrandoRng) -> Self {
        Self {
            blocked_side: ApproximateMulti::blocked_side(),
            overtake: ApproximateMulti::overtake(),
            condition_timer: -1.0,
            rng,
        }
    }

    /// Call once per frame with the same `dt` as the solver step (before lane movement).
    pub fn on_frame(
        &mut self,
        dt: f64,
        phase: Phase,
        strategy: Strategy,
        pos: f64,
        section_len: f64,
        current_lane: f64,
        horse_lane: f64,
    ) {
        self.condition_timer += dt;
        if self.condition_timer < 0.0 {
            return;
        }
        self.blocked_side.tick(
            &mut self.rng,
            phase,
            strategy,
            pos,
            section_len,
            current_lane,
            horse_lane,
        );
        self.overtake.tick(
            &mut self.rng,
            phase,
            strategy,
            pos,
            section_len,
            current_lane,
            horse_lane,
        );
        self.condition_timer = -1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_side_starts_active() {
        assert!(ApproximateMulti::blocked_side().active());
        assert!(!ApproximateMulti::overtake().active());
    }

    #[test]
    fn outer_lane_forces_off_on_tick() {
        let mut m = ApproximateMulti::blocked_side();
        let mut rng = PrandoRng::new(1);
        // section 2, wide lane → outer rates (0,0) → must go inactive
        m.tick(
            &mut rng,
            Phase::Opening,
            Strategy::Nige,
            2.5 * 100.0, // section ≈ 2 for section_len=100
            100.0,
            4.0,
            1.0,
        );
        assert!(!m.active());
    }

    #[test]
    fn condition_timer_ticks_once_per_second() {
        let mut sc = SpecialConditions::new(PrandoRng::new(42));
        let dt = 1.0 / 15.0;
        let mut tick_frames = Vec::new();
        for frame in 1..=45 {
            let t_before = sc.condition_timer;
            sc.on_frame(dt, Phase::Opening, Strategy::Sasi, 10.0, 100.0, 0.5, 1.0);
            // After a tick, timer is forced back to -1 (umalator conditionTimer reset).
            if t_before + dt >= 0.0 && (sc.condition_timer + 1.0).abs() < 1e-12 {
                tick_frames.push(frame);
            }
        }
        // IEEE: 15*(1/15) undershoots 1.0, so ticks every 16 frames (matches JS umalator).
        assert_eq!(tick_frames.first().copied(), Some(16));
        assert!(tick_frames.len() >= 2);
        assert_eq!(tick_frames[1] - tick_frames[0], 16);
    }
}
