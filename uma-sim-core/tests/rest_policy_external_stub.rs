//! T6 — REST honors `policy=external` when a policy server is reachable.
//!
//! This test lives in its own integration-test binary (separate OS process)
//! on purpose: the external policy handle (`policy_external::EXTERNAL`) is a
//! process-global cache, so a stub spawned here must never interleave with
//! the loud-failure unit tests in `api.rs` (whose results would otherwise
//! depend on thread scheduling order).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn parse_http(buf: &str) -> (u16, String) {
    let status = buf
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = match buf.split_once("\r\n\r\n") {
        Some((_, b)) => b.to_string(),
        None => String::new(),
    };
    (status, body)
}

fn http_post(port: u16, path: &str, json_body: &str) -> (u16, String) {
    let mut stream =
        TcpStream::connect(("127.0.0.1", port)).expect("connect to test server");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.len()
    );
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    parse_http(&buf)
}

fn wait_ready(port: u16) {
    for _ in 0..50 {
        if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = s.write_all(
                b"GET /v1/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            );
            let mut buf = Vec::new();
            let _ = s.read_to_end(&mut buf);
            if !buf.is_empty() {
                return;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not become ready on port {port}");
}

/// Extract the step "text" from a step_response body.
fn step_text(body: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(body).expect("step body must be JSON");
    v.get("text")
        .and_then(|t| t.as_str())
        .expect("step body must have a string \"text\" field")
        .to_string()
}

/// The stub policy server (`tests/fixtures/policy_stub.sh`) answers
/// mandatory races with a race, event turns with option 0, and trains GUTS on
/// free turns. The built-in default heuristic never trains guts
/// (`policy.rs::default_auto_policy` trains speed), so a "GUTS" step proves
/// the REST layer honored the external policy instead of silently downgrading.
#[test]
fn policy_external_positive_path_with_stub() {
    // This binary is the only writer of UMA_POLICY_CMD in its process.
    let stub = format!(
        "{}/tests/fixtures/policy_stub.sh",
        env!("CARGO_MANIFEST_DIR")
    );
    std::env::set_var("UMA_POLICY_CMD", &stub);

    let port = free_port();
    thread::spawn(move || uma_sim_core::api::serve(port));
    wait_ready(port);
    let (status, _) = http_post(
        port,
        "/v1/run/start",
        r#"{"seed":7,"scenario":"ura","trainee":"Special Week","raceModel":"stub"}"#,
    );
    assert_eq!(status, 200);

    // Step the career until the stub gets a free training turn (turn 1 is a
    // mandatory race, which the stub answers with a race).
    let mut guts_text = None;
    for i in 0..12 {
        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"external"}"#);
        assert_eq!(
            status, 200,
            "stub-backed external policy must be honored with 200 (step {i}); got {status}: {body}"
        );
        assert!(
            !body.contains("external policy unavailable"),
            "must not fail loudly when the server is reachable (step {i}): {body}"
        );
        let text = step_text(&body);
        if text.contains("GUTS") {
            guts_text = Some(text);
            break;
        }
    }
    let text = guts_text.expect("stub should reach a free training turn within 12 steps");

    // "GUTS" in the step text proves the stub's train action ran (the
    // heuristic can never emit it); "SPEED" absent proves the heuristic did
    // not run instead.
    assert!(
        !text.contains("SPEED"),
        "heuristic must not have run instead of the stub; text was: {text}"
    );

    std::env::remove_var("UMA_POLICY_CMD");
}
