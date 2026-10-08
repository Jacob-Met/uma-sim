//! Assert GameTora/Condor Performance type labels on facility splits.

use uma_sim_core::{GrandLiveMechanics, TrainingFacility};

#[test]
fn facility_split_labels_match_gametora_condor() {
    let cases = [
        (TrainingFacility::Speed, ["Da", "Vi", "Pa"]),
        (TrainingFacility::Stamina, ["Pa", "Vo", "Vi"]),
        (TrainingFacility::Power, ["Vo", "Me", "Da"]),
        (TrainingFacility::Guts, ["Vi", "Da", "Pa"]),
        (TrainingFacility::Wit, ["Me", "Pa", "Vo"]),
    ];
    for (fac, expected) in cases {
        let split = GrandLiveMechanics::facility_split(fac);
        let codes: Vec<&str> = split.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(codes, expected, "{fac:?}");
        assert_eq!(
            split.iter().map(|(_, p)| *p).collect::<Vec<_>>(),
            vec![60, 30, 10]
        );
    }
}
