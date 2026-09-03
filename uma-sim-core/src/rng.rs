//! Deterministic RNG matching Kotlin 2.0.21 `kotlin.random.Random(seed)` / `SimRandom`.

/// Kotlin 2.0+ XorWowRandom — see `libraries/stdlib/src/kotlin/random/XorWowRandom.kt`.
struct XorWowRandom {
    x: i32,
    y: i32,
    z: i32,
    w: i32,
    v: i32,
    addend: i32,
}

impl XorWowRandom {
    fn from_seed(seed: i64) -> Self {
        let seed1 = seed as i32;
        let seed2 = (seed >> 32) as i32;
        let mut r = Self {
            x: seed1,
            y: seed2,
            z: 0,
            w: 0,
            v: !seed1,
            addend: (seed1 << 10) ^ ((seed2 as u32 >> 4) as i32),
        };
        for _ in 0..64 {
            r.next_int_raw();
        }
        r
    }

    fn next_int_raw(&mut self) -> i32 {
        let mut t = self.x;
        t ^= (self.x as u32 >> 2) as i32;
        self.x = self.y;
        self.y = self.z;
        self.z = self.w;
        let v0 = self.v;
        self.w = v0;
        t = (t ^ (t << 1)) ^ v0 ^ (v0 << 4);
        self.v = t;
        self.addend = self.addend.wrapping_add(362437);
        t.wrapping_add(self.addend)
    }

    fn take_upper_bits(value: i32, bit_count: i32) -> i32 {
        if bit_count <= 0 {
            return 0;
        }
        ((value as u32 >> (32 - bit_count)) as i32) & ((-bit_count).wrapping_shr(31))
    }

    fn next_bits(&mut self, bit_count: i32) -> i32 {
        Self::take_upper_bits(self.next_int_raw(), bit_count)
    }

    fn next_int_until(&mut self, until: i32) -> i32 {
        assert!(until > 0);
        let n = until;
        if n > 0 || n == i32::MIN {
            if n & -n == n {
                let bit_count = 31 - n.leading_zeros() as i32;
                return self.next_bits(bit_count) & (n - 1);
            }
            loop {
                let bits = (self.next_int_raw() as u32 >> 1) as i32;
                let v = bits % n;
                if bits.wrapping_sub(v).wrapping_add(n - 1) >= 0 {
                    return v;
                }
            }
        }
        loop {
            let rnd = self.next_int_raw();
            if rnd >= 0 && rnd < until {
                return rnd;
            }
        }
    }

    fn next_int_range(&mut self, from: i32, until: i32) -> i32 {
        from + self.next_int_until(until - from)
    }

    fn next_double(&mut self) -> f64 {
        let hi = self.next_bits(26) as i64;
        let lo = self.next_bits(27) as i64;
        ((hi << 27) + lo) as f64 / (1i64 << 53) as f64
    }

    fn next_long(&mut self) -> i64 {
        ((self.next_int_raw() as i64) << 32) + (self.next_int_raw() as u32 as i64)
    }
}

/// Deterministic RNG wrapper. All sim rolls go through here for reproducibility.
pub struct SimRandom {
    seed: i64,
    inner: XorWowRandom,
    calls: u32,
    trace: bool,
    trace_log: Option<Vec<String>>,
}

impl SimRandom {
    pub fn new(seed: i64) -> Self {
        Self::with_trace(seed, false)
    }

    pub fn with_trace(seed: i64, trace: bool) -> Self {
        Self {
            seed,
            inner: XorWowRandom::from_seed(seed),
            calls: 0,
            trace,
            trace_log: if trace { Some(Vec::new()) } else { None },
        }
    }

    fn traced(&mut self, name: &str, value: impl std::fmt::Display) {
        if self.trace {
            if let Some(log) = &mut self.trace_log {
                log.push(format!("rng#{} {name}={value}", self.calls));
            }
        }
    }

    pub fn next_double(&mut self) -> f64 {
        self.calls += 1;
        let v = self.inner.next_double();
        self.traced("nextDouble", v);
        v
    }

    pub fn next_int_until(&mut self, until: i32) -> i32 {
        self.calls += 1;
        let v = self.inner.next_int_until(until);
        self.traced(&format!("nextInt({until})"), v);
        v
    }

    pub fn next_int_range(&mut self, from: i32, until: i32) -> i32 {
        self.calls += 1;
        let v = self.inner.next_int_range(from, until);
        self.traced(&format!("nextInt({from},{until})"), v);
        v
    }

    pub fn next_boolean(&mut self, probability: f64) -> bool {
        self.next_double() < probability
    }

    pub fn call_count(&self) -> u32 {
        self.calls
    }

    pub fn seed(&self) -> i64 {
        self.seed
    }

    pub fn trace_log(&self) -> Vec<String> {
        self.trace_log.clone().unwrap_or_default()
    }

    pub fn restore(seed: i64, prior_calls: u32) -> Self {
        Self::restore_with_trace(seed, prior_calls, false)
    }

    pub fn restore_with_trace(seed: i64, prior_calls: u32, trace: bool) -> Self {
        let mut r = Self::with_trace(seed, trace);
        for _ in 0..prior_calls {
            r.inner.next_long();
        }
        r.calls = prior_calls;
        r
    }

    #[cfg(test)]
    fn next_int_raw(&mut self) -> i32 {
        self.inner.next_int_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RngFixture {
        seed: i64,
        #[serde(default)]
        raw_ints: Vec<i32>,
        doubles: Vec<f64>,
        ints_until100: Vec<i32>,
        call_count_after_ints: u32,
        double_after_restore: f64,
    }

    #[test]
    fn raw_ints_match_kotlin() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rng_seed_42.json");
        let f: RngFixture = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let mut r = SimRandom::new(f.seed);
        for (i, expected) in f.raw_ints.iter().enumerate() {
            assert_eq!(r.next_int_raw(), *expected, "raw int #{i}");
        }
    }

    #[test]
    fn matches_kotlin_seed_42_fixture() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rng_seed_42.json");
        let f: RngFixture = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let mut r = SimRandom::new(f.seed);
        for (i, expected) in f.doubles.iter().enumerate() {
            let actual = r.next_double();
            assert!(
                (actual - expected).abs() < 1e-15,
                "double #{i}: expected {expected}, got {actual}"
            );
        }
        for (i, expected) in f.ints_until100.iter().enumerate() {
            assert_eq!(r.next_int_until(100), *expected, "intUntil100 #{i}");
        }
        assert_eq!(r.call_count(), f.call_count_after_ints);
        let mut restored = SimRandom::restore(f.seed, f.call_count_after_ints);
        let after = restored.next_double();
        assert!((after - f.double_after_restore).abs() < 1e-15);
    }
}
