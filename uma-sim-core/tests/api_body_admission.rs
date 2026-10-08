//! Request admission must finish before any career or library mutation.
//! Each test launches the actual API with its own process and storage directory.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT_SERVER: AtomicUsize = AtomicUsize::new(0);

struct Reply {
    status: u16,
    headers: String,
    body: String,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).expect("API JSON response")
    }

    fn assert_rejected(&self, context: &str) {
        assert_eq!(self.status, 400, "{context}: {}", self.body);
        assert!(
            self.json()["error"].as_str().is_some_and(|s| !s.is_empty()),
            "{context}: error must be a nonempty JSON string"
        );
        assert!(self.headers.contains("content-type: application/json"));
        assert!(self.headers.contains("access-control-allow-origin: *"));
    }
}

struct Api {
    process: Child,
    port: u16,
    storage: PathBuf,
}

impl Api {
    fn start() -> Self {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let storage = std::env::temp_dir().join(format!(
            "uma-api-body-{}-{nonce}-{}",
            std::process::id(),
            NEXT_SERVER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&storage).unwrap();
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let process = Command::new(env!("CARGO_BIN_EXE_uma-sim-api"))
            .arg(port.to_string())
            .current_dir(&storage)
            .env("UMA_REPO_ROOT", source_root)
            .env_remove("UMA_POLICY_CMD")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start isolated API");
        let mut api = Self {
            process,
            port,
            storage,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(reply) = api.try_request("GET", "/v1/health", b"") {
                if reply.status == 200 {
                    break;
                }
            }
            assert!(api.process.try_wait().unwrap().is_none(), "API exited");
            assert!(Instant::now() < deadline, "API did not become ready");
            thread::sleep(Duration::from_millis(20));
        }
        api
    }

    fn try_request(&self, method: &str, path: &str, body: &[u8]) -> std::io::Result<Reply> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        // HTTP/1.0 keeps these small test responses out of chunked encoding.
        write!(
            stream,
            "{method} {path} HTTP/1.0\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )?;
        stream.write_all(body)?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        let (headers, body) = response.split_once("\r\n\r\n").expect("HTTP response");
        let status = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .expect("HTTP status");
        Ok(Reply {
            status,
            headers: headers.to_ascii_lowercase(),
            body: body.to_string(),
        })
    }

    fn request(&self, method: &str, path: &str, body: &[u8]) -> Reply {
        self.try_request(method, path, body).expect("HTTP request")
    }

    fn post(&self, path: &str, body: Value) -> Value {
        let reply = self.request("POST", path, body.to_string().as_bytes());
        assert_eq!(reply.status, 200, "{path}: {}", reply.body);
        reply.json()
    }

    fn get(&self, path: &str) -> Value {
        let reply = self.request("GET", path, b"");
        assert_eq!(reply.status, 200, "{path}: {}", reply.body);
        reply.json()
    }

    fn prepare(&self) {
        self.post(
            "/v1/run/start",
            json!({
                "seed": 7301, "scenario": "ura", "raceModel": "stub",
                "session": "kept", "label": "Kept career"
            }),
        );
        for _ in 0..5 {
            self.post("/v1/run/auto", json!({"session":"kept"}));
        }
        self.post(
            "/v1/library/save",
            json!({"session":"kept","name":"preserved-checkpoint"}),
        );
    }

    fn observations(&self) -> Value {
        json!({
            "active": self.get("/v1/run/state"),
            "kept": self.get("/v1/run/state?session=kept"),
            "sessions": self.get("/v1/sessions"),
            "library": self.get("/v1/library"),
            "checkpoint": self.get("/v1/library/export?name=preserved-checkpoint"),
            "branches": self.get("/v1/lab/branches"),
            "telemetry": self.request("GET", "/v1/run/telemetry?session=kept", b"").body
        })
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        let _ = std::fs::remove_dir_all(&self.storage);
    }
}

#[test]
fn malformed_json_is_rejected_before_every_post_handler() {
    let api = Api::start();
    api.prepare();
    let before = api.observations();
    for path in [
        "/v1/run/start",
        "/v1/run/action",
        "/v1/run/auto",
        "/v1/run/fast",
        "/v1/run/load_content_pack",
        "/v1/run/deck/place",
        "/v1/run/style",
        "/v1/session/fork",
        "/v1/session/close",
        "/v1/session/activate",
        "/v1/library/save",
        "/v1/library/load",
        "/v1/library/delete",
        "/v1/library/import",
        "/v1/lab/branch",
        "/v1/lab/branch/delete",
        "/v1/lab/compare",
    ] {
        api.request("POST", path, br#"{"session":"kept","broken":"#)
            .assert_rejected(path);
        assert_eq!(api.observations(), before, "{path} changed career state");
    }
}

#[test]
fn nonobject_json_is_not_an_empty_command() {
    let api = Api::start();
    api.prepare();
    let before = api.observations();
    for body in ["null", "[]", "42", "true", r#""kept""#] {
        api.request("POST", "/v1/run/auto", body.as_bytes())
            .assert_rejected(body);
        assert_eq!(api.observations(), before, "{body} advanced the career");
    }
}

#[test]
fn invalid_utf8_is_rejected_before_start_and_auto() {
    let api = Api::start();
    api.prepare();
    let before = api.observations();
    for path in ["/v1/run/start", "/v1/run/auto"] {
        for body in [b"\xff".as_slice(), b"{}\xff".as_slice()] {
            api.request("POST", path, body).assert_rejected(path);
            assert_eq!(api.observations(), before, "unreadable body mutated state");
        }
    }
}

#[test]
fn empty_bodies_and_valid_objects_keep_existing_defaults() {
    let api = Api::start();
    let mut reference = None;
    for body in [b"{}".as_slice(), b"".as_slice(), b" \t\r\n".as_slice()] {
        let start = api.request("POST", "/v1/run/start", body);
        assert_eq!(start.status, 200, "{}", start.body);
        let step = api.request("POST", "/v1/run/auto", body);
        assert_eq!(step.status, 200, "{}", step.body);
        let state = api.get("/v1/run/state");
        if let Some(expected) = &reference {
            assert_eq!(&state, expected, "blank-body default changed");
        } else {
            reference = Some(state);
        }
    }

    // Body value coercion belongs to existing endpoint contracts, not admission.
    for seed in [json!(7301), json!("7301")] {
        api.post(
            "/v1/run/start",
            json!({
                "session":"unicode", "label":"練習 café 🐎",
                "seed":seed, "scenario":"ura", "raceModel":"stub",
                "futureClientField":{"nested":[1,true,null]}
            }),
        );
        let state = api.get("/v1/run/state?session=unicode");
        assert_eq!(state["state"]["meta"]["seed"], json!(7301));
        let sessions = api.get("/v1/sessions");
        assert!(sessions.to_string().contains("練習 café 🐎"));
    }
}

#[test]
fn unsupported_routes_and_preflight_keep_their_status() {
    let api = Api::start();
    for body in [b"{".as_slice(), b"\xff".as_slice()] {
        for (method, path, expected) in [
            ("POST", "/v1/health", 405),
            ("GET", "/v1/run/start", 405),
            ("PUT", "/v1/run/start", 405),
            ("POST", "/v1/not-a-route", 404),
            ("GET", "/v1/health", 200),
            ("OPTIONS", "/v1/run/start", 204),
        ] {
            let reply = api.request(method, path, body);
            assert_eq!(reply.status, expected, "{method} {path}: {}", reply.body);
            assert!(reply.headers.contains("access-control-allow-origin: *"));
        }
    }
}
