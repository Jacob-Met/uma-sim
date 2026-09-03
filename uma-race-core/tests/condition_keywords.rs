use uma_race_core::condition::regions::{reduce_condition_str, HorseCtx};
use uma_race_core::get_course;
use uma_race_core::{Aptitude, Strategy};

fn ctx() -> HorseCtx {
    HorseCtx {
        strategy: Strategy::Oikomi,
        distance_apt: Aptitude::A,
        surface_apt: Aptitude::A,
        ground: uma_race_core::GroundCondition::Good,
        mood: 0,
            speed: 1000.0,
            stamina: 1000.0,
            power: 1000.0,
            guts: 1000.0,
            wisdom: 1000.0,
            weather: 1,
                        season: 1,
                        time: 2,
                        grade: 100,
    }
}

#[test]
fn distance_rate_after_random_50_has_regions() {
    let c = get_course(10910).unwrap();
    let r = reduce_condition_str("distance_rate_after_random==50", c, ctx()).unwrap();
    assert!(!r.regions.is_empty());
    assert!(r.regions.regions[0].start >= c.distance * 0.5 - 1e-6);
}

#[test]
fn corner_3_uses_oval_corner_indices() {
    // Oval corner==3 → indices len+3-5, len+3-9, … (not corners[2]).
    let c = get_course(11404).unwrap();
    assert!(c.corners.len() >= 5);
    let r = reduce_condition_str("corner==3", c, ctx()).unwrap();
    assert!(!r.regions.is_empty());
    let mut expected = Vec::new();
    let mut idx = c.corners.len() as i64 + 3 - 5;
    while idx >= 0 {
        let corner = &c.corners[idx as usize];
        expected.push((corner.start, corner.start + corner.length));
        idx -= 4;
    }
    expected.reverse();
    assert_eq!(r.regions.regions.len(), expected.len());
    for (reg, (start, end)) in r.regions.regions.iter().zip(expected.iter()) {
        assert!((reg.start - start).abs() < 1e-6);
        assert!((reg.end - end).abs() < 1e-6);
    }
}

#[test]
fn is_basis_distance_filters_non_core_course() {
    let c = get_course(10311).unwrap(); // 1800m — not divisible by 400
    assert_eq!((c.distance as i64) % 400, 1800 % 400);
    let yes = reduce_condition_str("is_basis_distance==1", c, ctx()).unwrap();
    assert!(yes.regions.is_empty());
    let no = reduce_condition_str("is_basis_distance==0", c, ctx()).unwrap();
    assert!(!no.regions.is_empty());
}
