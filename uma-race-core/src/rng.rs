//! MIT-compatible port of the Prando xorshift(13,17,5) PRNG (zeh/prando, MIT),
//! plus the SeededRng wrapper shape used by the race oracle (int32 / random / uniform).

/// Deterministic PRNG matching npm `prando` 6.x bit-for-bit on numeric seeds.
#[derive(Clone, Debug)]
pub struct PrandoRng {
    seed: i32,
    value: i32,
}

impl PrandoRng {
    const MIN: i32 = i32::MIN;
    const MAX: i32 = i32::MAX;

    pub fn new(seed: u32) -> Self {
        let seed = Self::safe_seed(seed as i32);
        Self { seed, value: seed }
    }

    pub fn from_i32(seed: i32) -> Self {
        let seed = Self::safe_seed(seed);
        Self { seed, value: seed }
    }

    fn safe_seed(seed: i32) -> i32 {
        if seed == 0 {
            1
        } else {
            seed
        }
    }

    fn xorshift(mut value: i32) -> i32 {
        // JS bitwise ops are ToInt32; i32 wrapping matches.
        value ^= value.wrapping_shl(13);
        value ^= value.wrapping_shr(17); // arithmetic shift, like JS >>
        value ^= value.wrapping_shl(5);
        value
    }

    fn recalculate(&mut self) {
        self.value = Self::xorshift(self.value);
    }

    fn map(val: i32, min_from: i32, max_from: i32, min_to: f64, max_to: f64) -> f64 {
        let val = val as f64;
        let min_from = min_from as f64;
        let max_from = max_from as f64;
        ((val - min_from) / (max_from - min_from)) * (max_to - min_to) + min_to
    }

    /// `prando.next()` → [0, 1)
    pub fn random(&mut self) -> f64 {
        self.recalculate();
        Self::map(self.value, Self::MIN, Self::MAX, 0.0, 1.0)
    }

    /// `SeededRng.int32()` → floor(next() * 2^32)
    pub fn int32(&mut self) -> u32 {
        (self.random() * 4294967296.0).floor() as u32
    }

    /// `prando.nextInt(min, max)` inclusive
    pub fn next_int(&mut self, min: i32, max: i32) -> i32 {
        self.recalculate();
        Self::map(
            self.value,
            Self::MIN,
            Self::MAX,
            min as f64,
            (max as f64) + 1.0,
        )
        .floor() as i32
    }

    /// `SeededRng.uniform(upper)` → `nextInt(0, upper - 1)`.
    /// When `upper` is non-integer (umalator RandomPolicy passes float lengths),
    /// Prando maps into `[0, upper)` then floors — not `uniform(floor(upper))`.
    pub fn uniform(&mut self, upper: u32) -> u32 {
        if upper == 0 {
            return 0;
        }
        self.next_int(0, (upper as i32) - 1) as u32
    }

    /// Float-upper form of `SeededRng.uniform` (Prando `nextInt(0, upper - 1)`).
    pub fn uniform_f(&mut self, upper: f64) -> u32 {
        if upper <= 0.0 {
            return 0;
        }
        self.recalculate();
        Self::map(self.value, Self::MIN, Self::MAX, 0.0, upper).floor() as u32
    }

    pub fn reset(&mut self) {
        self.value = self.seed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

    fn vectors() -> Value {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../research/race_prando_vectors.json");
        let raw = fs::read_to_string(&path).expect("race_prando_vectors.json");
        serde_json::from_str(&raw).unwrap()
    }

    #[test]
    fn prando_readme_seed_12345678() {
        let v = vectors();
        let expected = v["prando_readme_check"].as_array().unwrap();
        let mut rng = PrandoRng::from_i32(12345678);
        assert!((rng.random() - expected[0].as_f64().unwrap()).abs() < 1e-15);
        assert!((rng.random() - expected[1].as_f64().unwrap()).abs() < 1e-15);
    }

    #[test]
    fn seeded_streams_match_oracle_dump() {
        let v = vectors();
        let seed = v["seed"].as_u64().unwrap() as u32;
        let n = v["random"].as_array().unwrap().len();

        let mut rng = PrandoRng::new(seed);
        for (i, exp) in v["random"].as_array().unwrap().iter().enumerate() {
            let got = rng.random();
            let e = exp.as_f64().unwrap();
            assert!(
                (got - e).abs() < 1e-15,
                "random[{i}]: got {got} expected {e}"
            );
        }

        let mut rng = PrandoRng::new(seed);
        for (i, exp) in v["int32"].as_array().unwrap().iter().enumerate() {
            let got = rng.int32();
            let e = exp.as_u64().unwrap() as u32;
            assert_eq!(got, e, "int32[{i}]");
        }

        let mut rng = PrandoRng::new(seed);
        for (i, exp) in v["uniform_100000"].as_array().unwrap().iter().enumerate() {
            let got = rng.uniform(100_000);
            let e = exp.as_u64().unwrap() as u32;
            assert_eq!(got, e, "uniform[{i}]");
        }

        assert_eq!(n, 16);
    }
}
