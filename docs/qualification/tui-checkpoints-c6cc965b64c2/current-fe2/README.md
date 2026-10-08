# Current native API receiving

This stage receives the frozen terminal workflow on current main fe2e6fdcdf7625cb5beac0e0062ace0360288a02. The local composed source is 92db1618cc014924e43a69b2dfe4d5891680b176. The original b8e3 authored and independent records remain in their original directories.

The current API was built from the unmodified fe2 tree using installed native Cargo/Rust, offline and locked with one job. The composed source does not alter Rust or Cargo inputs. The API now rejects unreadable, malformed, and non-object JSON POST bodies before endpoint mutation. Existing valid-object endpoint behavior remains compatible with the terminal.

The unchanged authored native package entry point passes all 11 groups without skips. Its actual restart restores the full turn-7/RNG-34 snapshot, preserves checkpoint bytes and unrelated careers, and reproduces three advancing bot states through turn 10/RNG 49. Actual 404/409/422/500 responses, lost post-commit outcomes, EOF and SIGINT retain their earlier no-replay classification.

Independent current receiving is retained verbatim in ../independent-current-fe2/: the original 16 groups pass unchanged, and three separate native request-admission groups pass. The old/current full state comparison ignores only JSON object-key order while preserving every numeric source token. Each run independently proves unchanged checkpoint bytes across its own restart; metadata wall-clock times are not asserted equal between different runs.

source-composition.json proves the 76-path pre-receiving overlay: 75 frozen non-CI paths exact, all 558 unaffected fe2 leaves unchanged, and current CI preserved with only the 13-line terminal native hook. Subsequent commits add receiving documentation only. This source/receiving result is separate from actual hosted CI, repository integration, and runtime deployment.

build-provenance.json pins the executable and its input files. native/ retains all 12 raw JSON captures; native.log and native-build.log retain the unedited package/build output. manifest.json records all files appended in this stage. No scanner allowlist or default rule is changed by this receiving step.
