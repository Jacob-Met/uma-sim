//! Admit a batch's complete seed selection before any output is opened.

use std::ops::RangeInclusive;

pub struct SeedPlan {
    pub first: i64,
    pub count: i64,
    pub seeds: SeedValues,
}

pub enum SeedValues {
    Consecutive(RangeInclusive<i64>),
    Explicit(std::vec::IntoIter<i64>),
}

impl Iterator for SeedValues {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Consecutive(seeds) => seeds.next(),
            Self::Explicit(seeds) => seeds.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Consecutive(seeds) => seeds.size_hint(),
            Self::Explicit(seeds) => seeds.size_hint(),
        }
    }
}

#[derive(Debug)]
pub struct SeedError {
    pub code: i32,
    pub message: String,
}

pub fn parse(args: &[String], seed_start: i64) -> Result<SeedPlan, SeedError> {
    // Retain the first explicit list's existing precedence and parsing:
    // ordered duplicates survive, empty comma fields are ignored, and an
    // explicit list does not inspect the unused count.
    if let Some(value) = args.iter().find_map(|arg| arg.strip_prefix("--seeds=")) {
        let list: Result<Vec<i64>, _> = value
            .split(',')
            .filter(|seed| !seed.trim().is_empty())
            .map(|seed| seed.trim().parse::<i64>())
            .collect();
        return match list {
            Ok(seeds) if !seeds.is_empty() => Ok(SeedPlan {
                first: seeds[0],
                count: seeds.len() as i64,
                seeds: SeedValues::Explicit(seeds.into_iter()),
            }),
            _ => Err(SeedError {
                code: 1,
                message: format!(
                    "--seeds needs a comma-separated list of integer seeds, got: {value}"
                ),
            }),
        };
    }

    let count = match args.iter().find_map(|arg| arg.strip_prefix("--count=")) {
        Some(value) => match value.parse::<i64>() {
            Ok(count) if count > 0 => count,
            _ => {
                return Err(SeedError {
                    code: 2,
                    message: format!(
                        "Error: invalid batch count '{value}'; expected a positive signed 64-bit integer"
                    ),
                })
            }
        },
        None => 100,
    };
    let last = seed_start.checked_add(count - 1).ok_or_else(|| SeedError {
        code: 2,
        message: format!(
            "Error: batch seed range overflows signed 64-bit integers: start={seed_start}, count={count}"
        ),
    })?;

    Ok(SeedPlan {
        first: seed_start,
        count,
        // RangeInclusive handles an i64::MAX endpoint without incrementing
        // past it. Keep the admitted range lazy regardless of batch size.
        seeds: SeedValues::Consecutive(seed_start..=last),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(flags: &[&str], start: i64) -> Result<SeedPlan, SeedError> {
        parse(
            &flags
                .iter()
                .map(|flag| flag.to_string())
                .collect::<Vec<_>>(),
            start,
        )
    }

    #[test]
    fn default_cohort_keeps_100_consecutive_seeds() {
        let admitted = plan(&[], 42).unwrap();
        assert_eq!(admitted.first, 42);
        assert_eq!(admitted.count, 100);
        assert_eq!(
            admitted.seeds.collect::<Vec<_>>(),
            (42..142).collect::<Vec<_>>()
        );
    }

    #[test]
    fn maximum_count_is_admitted_lazily_without_a_batch_size_cap() {
        let mut admitted = plan(&["--count=9223372036854775807"], i64::MIN).unwrap();
        assert_eq!(admitted.count, i64::MAX);
        assert_eq!(admitted.seeds.next(), Some(i64::MIN));
        assert_eq!(admitted.seeds.next(), Some(i64::MIN + 1));
        assert_eq!(admitted.seeds.next(), Some(i64::MIN + 2));
    }

    #[test]
    fn range_including_maximum_seed_finishes_without_overflow() {
        let admitted = plan(&["--count=2"], i64::MAX - 1).unwrap();
        assert_eq!(
            admitted.seeds.collect::<Vec<_>>(),
            vec![i64::MAX - 1, i64::MAX]
        );
    }

    #[test]
    fn minimum_seed_and_signed_positive_count_are_valid() {
        let admitted = plan(&["--count=+1"], i64::MIN).unwrap();
        assert_eq!(admitted.seeds.collect::<Vec<_>>(), vec![i64::MIN]);
    }

    #[test]
    fn zero_negative_malformed_and_out_of_range_counts_are_refused() {
        for flag in [
            "--count=0",
            "--count=-1",
            "--count=",
            "--count=oops",
            "--count=1.5",
            "--count=9223372036854775808",
        ] {
            let error = plan(&[flag], 42).err().expect("invalid count refused");
            assert_eq!(error.code, 2, "{flag}");
            assert!(error.message.contains("invalid batch count"), "{flag}");
        }
    }

    #[test]
    fn explicit_and_default_overflow_are_refused() {
        for flags in [vec!["--count=2"], vec![]] {
            let error = plan(&flags, i64::MAX).err().expect("overflow refused");
            assert_eq!(error.code, 2);
            assert!(error.message.contains("seed range overflows"));
        }
    }

    #[test]
    fn first_count_wins_and_explicit_list_overrides_ignored_count() {
        let admitted = plan(&["--count=2", "--count=oops"], 7).unwrap();
        assert_eq!(admitted.seeds.collect::<Vec<_>>(), vec![7, 8]);
        assert!(plan(&["--count=oops", "--count=2"], 7).is_err());
        let admitted = plan(
            &["--count=oops", "--seeds=10, ,5,,10,", "--seeds=99"],
            i64::MAX,
        )
        .unwrap();
        assert_eq!(admitted.first, 10);
        assert_eq!(admitted.count, 3);
        assert_eq!(admitted.seeds.collect::<Vec<_>>(), vec![10, 5, 10]);
    }

    #[test]
    fn invalid_first_explicit_list_keeps_its_original_error() {
        for flags in [
            vec!["--seeds=", "--seeds=7"],
            vec!["--seeds=, ,", "--count=1"],
            vec!["--seeds=7,nope", "--count=1"],
        ] {
            let error = plan(&flags, 42).err().expect("invalid list refused");
            assert_eq!(error.code, 1);
            assert!(error
                .message
                .starts_with("--seeds needs a comma-separated list"));
        }
    }
}
