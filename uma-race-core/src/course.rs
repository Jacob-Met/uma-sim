//! Course geometry loaded from `research/race_course_data.json`.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::hp::Surface;

#[derive(Clone, Debug, Deserialize)]
pub struct Slope {
    pub start: f64,
    pub length: f64,
    pub slope: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Corner {
    pub start: f64,
    pub length: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Straight {
    pub start: f64,
    pub end: f64,
    #[serde(rename = "frontType")]
    pub front_type: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Course {
    #[serde(rename = "raceTrackId")]
    pub race_track_id: u32,
    pub distance: f64,
    #[serde(rename = "distanceType")]
    pub distance_type: u8,
    pub surface: u8,
    pub turn: u8,
    #[serde(rename = "courseSetStatus", default)]
    pub course_set_status: Vec<u8>,
    /// Basis-point lane capacity from fork `course_data` (`laneMax`).
    #[serde(rename = "laneMax", default = "default_lane_max")]
    pub lane_max: u32,
    #[serde(default)]
    pub corners: Vec<Corner>,
    #[serde(default)]
    pub straights: Vec<Straight>,
    #[serde(default)]
    pub slopes: Vec<Slope>,
}

fn default_lane_max() -> u32 {
    10_000
}

impl Course {
    pub fn surface_enum(&self) -> Surface {
        match self.surface {
            2 => Surface::Dirt,
            _ => Surface::Turf,
        }
    }

    /// Slope value at `pos` (game units; 10000 ≈ 1.0 grade). 0 if flat.
    pub fn slope_at(&self, pos: f64) -> f64 {
        for s in &self.slopes {
            if pos >= s.start && pos < s.start + s.length {
                return s.slope;
            }
        }
        0.0
    }

    /// True when |grade| > 1m per 100m (KuromiAK).
    pub fn is_uphill(&self, pos: f64) -> bool {
        self.slope_at(pos) >= 10_000.0
    }

    pub fn is_downhill(&self, pos: f64) -> bool {
        self.slope_at(pos) <= -10_000.0
    }

    pub fn course_width(&self) -> f64 {
        11.25
    }

    pub fn horse_lane(&self) -> f64 {
        self.course_width() / 18.0
    }

    pub fn max_lane_distance(&self) -> f64 {
        self.course_width() * self.lane_max as f64 / 10_000.0
    }

    pub fn move_lane_point(&self) -> f64 {
        self.corners.first().map(|c| c.start).unwrap_or(30.0)
    }

    pub fn lane_change_accel_per_frame(&self) -> f64 {
        (0.02 * 1.5) / 15.0
    }
}

#[derive(Deserialize)]
struct FileRoot {
    courses: HashMap<String, Course>,
}

fn catalog() -> &'static HashMap<String, Course> {
    static CAT: OnceLock<HashMap<String, Course>> = OnceLock::new();
    CAT.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../research/race_course_data.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let root: FileRoot = serde_json::from_str(&raw).expect("parse race_course_data.json");
        root.courses
    })
}

pub fn get_course(course_id: u32) -> Option<&'static Course> {
    catalog().get(&course_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_all_89_career_course_ids() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../research/race_course_data.json");
        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(raw["career_course_ids_required"], 89);
        assert_eq!(raw["career_course_ids_covered"], 89);
        assert!(raw["missing_course_ids"].as_array().unwrap().is_empty());
        for id in [10601u32, 10205, 10611, 11203, 11302, 11612] {
            assert!(get_course(id).is_some(), "missing course {id}");
        }
    }

    #[test]
    fn course_10601_round_trips_slope_geometry() {
        let c = get_course(10601).unwrap();
        assert_eq!(c.slopes.len(), 3);
        assert!((c.slopes[0].start - 125.0).abs() < 1e-9);
        assert!((c.slopes[0].length - 75.0).abs() < 1e-9);
        assert!((c.slopes[0].slope - 20000.0).abs() < 1e-9);
        assert!(c.lane_max > 0);
        assert!(c.max_lane_distance() > 0.0);
    }
}
