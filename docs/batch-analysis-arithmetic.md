# Skill-shop gains across the accepted score range

`uma-sim analyze` reports the average of each saved record's `score` minus
`score_before_shop`. Both fields accept signed 32-bit integers. The subtraction
now widens each operand to the existing signed 64-bit accumulator before taking
the difference, so the full accepted difference range is retained.

This corrects saved-input arithmetic. It does not change career scoring or claim
that a normal simulated career reaches the endpoint values used by the tests.
The existing omitted-field defaults and skipped malformed-record behavior still
apply.

## Reproduction

Save this single synthetic record as `extreme.jsonl`:

```json
{"seed":1,"scenario":"ura","grade":"F","score":2147483647,"score_before_shop":-2147483648}
```

Run the existing analyzer:

```sh
uma-sim analyze --input=extreme.jsonl --format=json
```

The expected `summary.shop_gain_mean` is `4294967295.0`. The original source
panicked with overflow checks enabled and reported `-1.0` with them disabled.
Reversing the two scores should report `-4294967295.0`.

## Focused verification

The original is main `cd852b3be106855524e789c42670d38f419d3bd5`, module blob
`8169e3f115c4ee3a7cc2464190ced43fceb770dc`. The original module's six existing
tests passed before the new cases were added. Native Rust 1.93.1 execution of
that exact module, its existing tests, and the new public-API integration cases
produced:

| Source and configuration | Passed | Failed | Result |
| --- | ---: | ---: | --- |
| Original, overflow checks enabled | 8 | 3 | Endpoint differences panic |
| Original, optimized with overflow checks disabled | 9 | 2 | Endpoint differences wrap |
| Corrected, overflow checks enabled | 11 | 0 | Full differences retained |
| Corrected, optimized with overflow checks disabled | 11 | 0 | Full differences retained |

The new cases also exercise mixed positive/negative/ordinary gains, equal
endpoints, text and JSON rendering, an omitted pre-shop score, and malformed or
out-of-range records. The source module was compiled directly with real
`serde`/`serde_json` dependencies because the shared filesystem was full; this
is scoped native module verification, not a full-workspace Cargo test result.

To run the same maintained tests from a complete checkout with its normal Rust
environment:

```sh
cargo test --locked -p uma-sim-core --lib batch_analysis::tests
cargo test --locked -p uma-sim-core --test batch_analysis_numeric
cargo test --locked --release -p uma-sim-core --test batch_analysis_numeric
```

Source ownership and receiving updates are tracked in
[issue #158](https://github.com/Jacob-Met/uma-sim/issues/158).
