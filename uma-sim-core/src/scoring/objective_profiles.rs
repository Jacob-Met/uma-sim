#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveWeights {
    pub spark_quality: f64,
    pub pvp_raceability: f64,
    pub career_score: f64,
    pub stat_targets: f64,
    pub scenario_completion: f64,
}

impl Default for ObjectiveWeights {
    fn default() -> Self {
        Self {
            spark_quality: 0.0,
            pvp_raceability: 0.0,
            career_score: 0.0,
            stat_targets: 0.0,
            scenario_completion: 0.0,
        }
    }
}

impl ObjectiveWeights {
    pub fn normalized(self) -> ObjectiveWeights {
        let sum = self.spark_quality
            + self.pvp_raceability
            + self.career_score
            + self.stat_targets
            + self.scenario_completion;
        if sum <= 1e-9 {
            return ObjectiveWeights {
                stat_targets: 1.0,
                ..Default::default()
            };
        }
        ObjectiveWeights {
            spark_quality: self.spark_quality / sum,
            pvp_raceability: self.pvp_raceability / sum,
            career_score: self.career_score / sum,
            stat_targets: self.stat_targets / sum,
            scenario_completion: self.scenario_completion / sum,
        }
    }

    pub fn blend(self, other: ObjectiveWeights, t: f64) -> ObjectiveWeights {
        let u = t.clamp(0.0, 1.0);
        ObjectiveWeights {
            spark_quality: self.spark_quality * (1.0 - u) + other.spark_quality * u,
            pvp_raceability: self.pvp_raceability * (1.0 - u) + other.pvp_raceability * u,
            career_score: self.career_score * (1.0 - u) + other.career_score * u,
            stat_targets: self.stat_targets * (1.0 - u) + other.stat_targets * u,
            scenario_completion: self.scenario_completion * (1.0 - u)
                + other.scenario_completion * u,
        }
        .normalized()
    }
}

pub struct ObjectiveProfiles;

impl ObjectiveProfiles {
    pub const SPARK_FARMING: ObjectiveWeights = ObjectiveWeights {
        spark_quality: 0.55,
        scenario_completion: 0.25,
        stat_targets: 0.15,
        career_score: 0.05,
        pvp_raceability: 0.0,
    };

    pub const PVP_ACE: ObjectiveWeights = ObjectiveWeights {
        pvp_raceability: 0.5,
        stat_targets: 0.3,
        career_score: 0.15,
        scenario_completion: 0.05,
        spark_quality: 0.0,
    };

    pub const CAREER_SCORE: ObjectiveWeights = ObjectiveWeights {
        career_score: 0.55,
        stat_targets: 0.25,
        pvp_raceability: 0.1,
        scenario_completion: 0.1,
        spark_quality: 0.0,
    };

    pub const STAT_TOTAL: ObjectiveWeights = ObjectiveWeights {
        stat_targets: 0.7,
        career_score: 0.15,
        pvp_raceability: 0.1,
        scenario_completion: 0.05,
        spark_quality: 0.0,
    };

    pub const SCENARIO_CLEAR_GRAND_CONCERT: ObjectiveWeights = ObjectiveWeights {
        scenario_completion: 0.5,
        stat_targets: 0.25,
        spark_quality: 0.1,
        career_score: 0.1,
        pvp_raceability: 0.05,
    };

    pub const DEFAULT: ObjectiveWeights = Self::STAT_TOTAL;

    pub fn by_name(name: &str) -> ObjectiveWeights {
        match name.trim().to_lowercase().replace(' ', "_").as_str() {
            "spark_farming" | "spark" | "sparks" => Self::SPARK_FARMING,
            "pvp_ace" | "pvp" | "cm" => Self::PVP_ACE,
            "career_score" | "rank" | "score" => Self::CAREER_SCORE,
            "stat_total" | "stats" | "stat_targets" | "default" => Self::STAT_TOTAL,
            "scenario_clear" | "grand_concert" | "grand_live" | "scenario" => {
                Self::SCENARIO_CLEAR_GRAND_CONCERT
            }
            _ => Self::DEFAULT,
        }
    }
}

pub fn combine_objective_score(
    base_training_score: f64,
    scenario_action_score: f64,
    weights: ObjectiveWeights,
) -> f64 {
    let w = weights.normalized();
    let training_part = base_training_score
        * (w.stat_targets + w.career_score + w.pvp_raceability + 0.5 * w.spark_quality);
    let scenario_part = scenario_action_score * (w.scenario_completion + 0.5 * w.spark_quality);
    training_part + scenario_part
}
