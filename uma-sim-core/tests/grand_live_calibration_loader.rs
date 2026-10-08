//! Port of JVM `GrandLiveCalibrationLoaderTest.kt`.

use std::sync::Mutex;

use uma_sim_core::{
    detect_repo_root, GrandLiveCalibrationLoader, GrandLiveMechanics, TrainingFacility,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn loads_calibration_from_repo() {
    let _g = TEST_LOCK.lock().unwrap();
    let root = detect_repo_root().expect("repo root required");
    GrandLiveCalibrationLoader::init_from_repo(Some(&root));
    assert!(GrandLiveCalibrationLoader::loaded());
    let gains = GrandLiveMechanics::training_token_gain(TrainingFacility::Speed, 1, 0, 0, None);
    assert_eq!(gains.get("Da").copied(), Some(6));
    assert_eq!(gains.get("Vi").copied(), Some(3));
    assert_eq!(gains.get("Pa").copied(), Some(1));
}

#[test]
fn deck_size_specific_row_preferred() {
    let _g = TEST_LOCK.lock().unwrap();
    let root = detect_repo_root().expect("repo root required");
    GrandLiveCalibrationLoader::init_from_repo(Some(&root));
    let gains = GrandLiveMechanics::training_token_gain(TrainingFacility::Speed, 1, 2, 0, None);
    assert_eq!(gains.get("Da").copied(), Some(8));
}

#[test]
fn load_from_path() {
    let _g = TEST_LOCK.lock().unwrap();
    let root = detect_repo_root().expect("repo root required");
    let path = root.join("research/grand_concert_calibration.json");
    GrandLiveMechanics::install_calibration(None);
    GrandLiveCalibrationLoader::load_path(&path);
    assert!(!GrandLiveCalibrationLoader::confidence_tier().is_empty());
}

#[test]
fn calibration_row_totals_documented() {
    let _g = TEST_LOCK.lock().unwrap();
    let root = detect_repo_root().expect("repo root required");
    GrandLiveCalibrationLoader::init_from_repo(Some(&root));
    let gains = GrandLiveMechanics::training_token_gain(TrainingFacility::Speed, 1, 0, 0, None);
    assert_eq!(gains.values().sum::<i32>(), 10);
}
