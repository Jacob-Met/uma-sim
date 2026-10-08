//! Activation sample policies (KuromiAK / GameTora `*_random` conditions).

use crate::condition::region::{Region, RegionList};
use crate::rng::PrandoRng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplePolicy {
    Immediate,
    /// Uniform over total length, then a 10m trigger window (phase_random et al.).
    Random,
    /// Pick a region with equal weight, then a point on it (straight_random).
    StraightRandom,
    /// Place up to 4 forward triggers across corner arcs; return the earliest 10m window
    /// (umalator `AllCornerRandomPolicy`).
    AllCornerRandom,
    /// Uniform offset into total length; trigger lasts until that piece's end.
    DistUniform,
    /// Erlang(k, λ) offset into the region, trigger lasting until region end.
    Erlang {
        k: u8,
        lambda: u8,
    },
}

impl SamplePolicy {
    /// Dominance: AllCornerRandom > StraightRandom > Random > DistUniform/Erlang > Immediate.
    /// Equal-rank DistUniform/Erlang: keep `other` (umalator `reconcileDistributionRandom`
    /// returns the receiver = right side of `left.reconcile(right)`). Other equal ranks keep
    /// left (umalator's double-dispatch returns the argument = left).
    pub fn reconcile(self, other: Self) -> Self {
        use SamplePolicy::*;
        fn rank(p: SamplePolicy) -> u8 {
            match p {
                Immediate => 0,
                DistUniform | Erlang { .. } => 1,
                Random => 2,
                StraightRandom => 3,
                AllCornerRandom => 4,
            }
        }
        let (rs, ro) = (rank(self), rank(other));
        if rs > ro {
            self
        } else if ro > rs {
            other
        } else {
            match (self, other) {
                (DistUniform | Erlang { .. }, DistUniform | Erlang { .. }) => other,
                _ => self,
            }
        }
    }

    pub fn sample(&self, regions: &RegionList, rng: &mut PrandoRng) -> Option<Region> {
        if regions.is_empty() {
            return None;
        }
        match self {
            SamplePolicy::Immediate => {
                let r = regions.regions[0];
                Some(Region::new(r.start, r.end))
            }
            SamplePolicy::Random => {
                let mut acc = 0.0_f64;
                let weights: Vec<f64> = regions
                    .regions
                    .iter()
                    .map(|r| {
                        acc += r.len();
                        acc
                    })
                    .collect();
                let upper = acc.floor().max(1.0) as u32;
                let threshold = rng.uniform(upper) as f64;
                let region = regions
                    .regions
                    .iter()
                    .zip(weights.iter())
                    .find(|(_, w)| **w > threshold)
                    .map(|(r, _)| *r)
                    .unwrap_or(regions.regions[0]);
                let span = (region.end - region.start - 10.0).max(0.0);
                let offset = if span < 1.0 {
                    0
                } else {
                    rng.uniform(span.floor().max(1.0) as u32)
                };
                let start = region.start + offset as f64;
                Some(Region::new(start, start + 10.0))
            }
            SamplePolicy::StraightRandom => {
                let idx = rng.uniform(regions.regions.len() as u32) as usize;
                let region = regions.regions[idx];
                let span = (region.end - region.start - 10.0).max(0.0);
                let offset = if span < 1.0 {
                    0
                } else {
                    rng.uniform(span.floor().max(1.0) as u32)
                };
                let start = region.start + offset as f64;
                Some(Region::new(start, start + 10.0))
            }
            SamplePolicy::AllCornerRandom => sample_all_corner_random(regions, rng),
            SamplePolicy::DistUniform => {
                let range: f64 = regions.regions.iter().map(|r| r.len()).sum();
                if range <= 0.0 {
                    return None;
                }
                let upper = range.floor().max(1.0) as u32;
                let offset = rng.uniform(upper) as f64;
                place_dist_offset(regions, offset)
            }
            SamplePolicy::Erlang { k, lambda } => {
                let range: f64 = regions.regions.iter().map(|r| r.len()).sum();
                if range <= 0.0 {
                    return None;
                }
                let mut u = 1.0_f64;
                for _ in 0..*k {
                    u *= rng.random();
                }
                let n = -u.ln() / (*lambda as f64);
                let scale = 18.0;
                let offset = (range * (n / scale).min(1.0)).floor();
                place_dist_offset(regions, offset)
            }
        }
    }
}

/// Match umalator `AllCornerRandomPolicy.placeTriggers`: up to 4 forward corner picks,
/// return the earliest as a 10m activation window.
fn sample_all_corner_random(regions: &RegionList, rng: &mut PrandoRng) -> Option<Region> {
    let mut candidates: Vec<Region> = regions.regions.clone();
    candidates.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    let mut triggers: Vec<f64> = Vec::new();
    while triggers.len() < 4 && !candidates.is_empty() {
        let ci = rng.uniform(candidates.len() as u32) as usize;
        let c = candidates[ci];
        // Prando `uniform(upper)` → nextInt(0, upper-1); floor like JS when upper is float.
        let upper = (c.end - c.start - 10.0).floor().max(1.0) as u32;
        let start = c.start + rng.uniform(upper) as f64;
        if start + 20.0 <= c.end {
            candidates[ci] = Region::new(start + 10.0, c.end);
        } else {
            candidates.remove(ci);
        }
        // Drop every candidate before the chosen index (umalator `splice(0, ci)`).
        if ci > 0 {
            let end = ci.min(candidates.len());
            candidates.drain(0..end);
        }
        triggers.push(start);
    }
    let t0 = *triggers.first()?;
    Some(Region::new(t0, t0 + 10.0))
}

fn place_dist_offset(regions: &RegionList, mut pos: f64) -> Option<Region> {
    let mut rs = regions.regions.clone();
    rs.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    for r in &rs {
        pos += r.start;
        if pos > r.end {
            pos -= r.end;
        } else {
            return Some(Region::new(pos, r.end));
        }
    }
    let last = *rs.last()?;
    Some(Region::new(last.start, last.end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erlang_reconcile_keeps_right_k() {
        // Skill 10091: bashin_diff_behind (k=3) @ is_overtake (k=1) → k=1.
        let left = SamplePolicy::Erlang { k: 3, lambda: 2 };
        let right = SamplePolicy::Erlang { k: 1, lambda: 2 };
        assert_eq!(
            left.reconcile(right),
            SamplePolicy::Erlang { k: 1, lambda: 2 }
        );
    }
}
