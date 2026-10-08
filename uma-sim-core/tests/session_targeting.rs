//! Explicit main-session requests must stay on main after a fork is activated.
//! Each test owns a disposable API process; no shared service or policy state.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

struct TestApi {
    process: Child,
    port: u16,
}

impl TestApi {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let process = Command::new(env!("CARGO_BIN_EXE_uma-sim-api"))
            .arg(port.to_string())
            .env_remove("UMA_POLICY_CMD")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let mut api = Self { process, port };
        for _ in 0..100 {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return api;
            }
            assert!(
                api.process.try_wait().unwrap().is_none(),
                "API process exited"
            );
            thread::sleep(Duration::from_millis(20));
        }
        panic!("API process did not become ready");
    }

    fn request(&self, method: &str, path: &str, body: Value) -> (u16, Value) {
        let raw = if method == "GET" {
            String::new()
        } else {
            body.to_string()
        };
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(stream,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{raw}", raw.len()
        ).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
        (
            status,
            serde_json::from_str(body).expect("API must return JSON"),
        )
    }

    fn get(&self, path: &str) -> Value {
        let (status, body) = self.request("GET", path, Value::Null);
        assert_eq!(status, 200, "{path}: {body}");
        body
    }

    fn post(&self, path: &str, body: Value) -> Value {
        let (status, result) = self.request("POST", path, body);
        assert_eq!(status, 200, "{path}: {result}");
        result
    }

    fn start(&self, session: &str, seed: i64) {
        self.post(
            "/v1/run/start",
            json!({
                "session": session, "seed": seed, "scenario": "ura",
                "policy": "default", "traceTelemetry": true,
            }),
        );
    }
}

impl Drop for TestApi {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[test]
fn explicit_main_reads_do_not_follow_an_active_fork() {
    let api = TestApi::new();
    api.start("", 10);
    let paths = ["state", "text", "choices", "telemetry"];
    let original: Vec<Value> = paths
        .iter()
        .map(|path| api.get(&format!("/v1/run/{path}")))
        .collect();
    api.start("branch", 20);
    assert_eq!(api.get("/v1/run/state")["meta"]["seed"], 20);
    for (path, expected) in paths.iter().zip(original) {
        assert_eq!(
            api.get(&format!("/v1/run/{path}?session=")),
            expected,
            "{path}"
        );
    }
    assert_eq!(api.get("/v1/sessions")["active"], "branch");
}

#[test]
fn explicit_main_action_changes_only_main() {
    let api = TestApi::new();
    api.start("", 10);
    let before = api.get("/v1/run/state");
    let action = api.get("/v1/run/choices")["choices"][0]["id"].clone();
    api.start("branch", 20);
    let branch = api.get("/v1/run/state");
    let step = api.post("/v1/run/action", json!({"action": action, "session": ""}));
    assert_eq!(step["state"]["meta"]["seed"], 10);
    assert_ne!(
        step["state"], before,
        "a real offered action must advance main"
    );
    assert_eq!(api.get("/v1/run/state?session=branch"), branch);
}

#[test]
fn explicit_main_fast_forward_keeps_active_fork_unchanged() {
    let api = TestApi::new();
    api.start("", 10);
    api.start("branch", 20);
    let branch = api.get("/v1/run/state");
    api.post(
        "/v1/run/fast",
        json!({"policy": "default", "session": "", "multiplier": 100}),
    );
    assert_eq!(
        api.get("/v1/run/state?session=")["state"]["careerComplete"],
        true
    );
    assert_eq!(api.get("/v1/run/state?session=branch"), branch);
}

#[test]
fn empty_body_session_precedes_query_and_empty_query_is_explicit() {
    let api = TestApi::new();
    api.start("", 10);
    api.start("branch", 20);
    let selected = api.post(
        "/v1/run/style?session=branch",
        json!({"session": "", "style": "front"}),
    );
    assert_eq!(selected["meta"]["seed"], 10);
    let selected = api.post("/v1/run/style?session=", json!({"style": "late"}));
    assert_eq!(selected["meta"]["seed"], 10);
    assert_eq!(selected["state"]["preferredRunningStyle"], "late");
    let selected = api.post("/v1/run/style", json!({"style": "pace"}));
    assert_eq!(
        selected["meta"]["seed"], 20,
        "omission keeps active-session behavior"
    );
    assert_eq!(
        api.get("/v1/run/state?session=")["state"]["preferredRunningStyle"],
        "late"
    );
}

#[test]
fn absent_explicit_main_never_falls_back_to_the_active_fork() {
    let api = TestApi::new();
    api.start("branch", 20);
    let branch = api.get("/v1/run/state");
    assert_eq!(
        api.request("GET", "/v1/run/state?session=", Value::Null).0,
        404
    );
    assert_eq!(
        api.request(
            "POST",
            "/v1/run/action",
            json!({"session": "", "action": "rest"})
        )
        .0,
        404
    );
    assert_eq!(api.get("/v1/run/state?session=branch"), branch);
    assert_eq!(
        api.request("GET", "/v1/run/state?session=missing", Value::Null)
            .0,
        404
    );
}
