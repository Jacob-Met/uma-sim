//! Minimal REST API matching Kotlin `SimApiServer` (Phase 7).

use crate::catalog::event::{install_event_catalog, EventCatalog, FileEventCatalog};
use crate::content::{ContentPackLoader, ContentPackRegistry};
use crate::deck::DeckPlacement;
use crate::engine::SimEngine;
use crate::factory::detect_repo_root;
use crate::policy::default_auto_policy;
use crate::render::TextRenderer;
use crate::session::parse_sim_action;
use crate::snapshot::RunSnapshotCodec;
use crate::state::{DialogueMode, RunMeta, SimSettings};
use serde_json::{json, Value};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

struct ApiState {
    engine: Option<SimEngine>,
    settings: SimSettings,
}

/// Start the REST server on `port` (blocking).
pub fn serve(port: u16) {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).unwrap_or_else(|e| {
        eprintln!("Failed to bind {addr}: {e}");
        std::process::exit(1);
    });
    println!("uma-sim REST API on http://127.0.0.1:{port}");

    let state = Arc::new(Mutex::new(ApiState {
        engine: None,
        settings: SimSettings::default(),
    }));

    for mut request in server.incoming_requests() {
        let path = request.url().split('?').next().unwrap_or("/").to_string();
        let method = request.method().clone();
        let body = read_body(&mut request);
        let mut st = state.lock().unwrap();
        let response = route(&mut st, &method, &path, &body);
        drop(st);
        let _ = request.respond(response);
    }
}

fn route(
    st: &mut ApiState,
    method: &Method,
    path: &str,
    body: &str,
) -> Response<Cursor<Vec<u8>>> {
    match (method, path) {
        (Method::Post, "/v1/run/start") => handle_start(st, body),
        (Method::Get, "/v1/run/state") => handle_state(st),
        (Method::Get, "/v1/run/text") => handle_text(st),
        (Method::Get, "/v1/run/choices") => handle_choices(st),
        (Method::Post, "/v1/run/action") => handle_action(st, body),
        (Method::Post, "/v1/run/auto") => handle_auto(st, body),
        (Method::Post, "/v1/run/fast") => handle_fast(st, body),
        (Method::Get, "/v1/run/telemetry") => handle_telemetry(st),
        (Method::Post, "/v1/run/load_content_pack") => handle_load_content_pack(st, body),
        (Method::Post, "/v1/run/deck/place") => handle_deck_place(st, body),
        _ if is_known_path(path) => method_not_allowed(),
        _ => json_response(404, json!({"error":"not found"})),
    }
}

fn is_known_path(path: &str) -> bool {
    matches!(
        path,
        "/v1/run/start"
            | "/v1/run/state"
            | "/v1/run/text"
            | "/v1/run/choices"
            | "/v1/run/action"
            | "/v1/run/auto"
            | "/v1/run/fast"
            | "/v1/run/telemetry"
            | "/v1/run/load_content_pack"
            | "/v1/run/deck/place"
    )
}

fn read_body(request: &mut Request) -> String {
    let mut buf = String::new();
    let _ = std::io::Read::read_to_string(&mut request.as_reader(), &mut buf);
    buf
}

fn parse_body(raw: &str) -> Value {
    if raw.trim().is_empty() {
        return json!({});
    }
    serde_json::from_str(raw).unwrap_or_else(|_| json!({}))
}

fn body_string(body: &Value, key: &str) -> Option<String> {
    match body.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn json_response(code: u16, body: Value) -> Response<Cursor<Vec<u8>>> {
    let bytes = body.to_string().into_bytes();
    Response::from_data(bytes)
        .with_status_code(StatusCode(code))
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
}

fn json_response_str(code: u16, body: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(body.as_bytes().to_vec())
        .with_status_code(StatusCode(code))
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
}

fn method_not_allowed() -> Response<Cursor<Vec<u8>>> {
    Response::from_data(Vec::new()).with_status_code(StatusCode(405))
}

fn state_json(st: &ApiState) -> String {
    st.engine
        .as_ref()
        .map(|e| RunSnapshotCodec::encode(&e.export()))
        .unwrap_or_else(|| "{}".to_string())
}

fn handle_start(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let seed = body_string(&body, "seed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(42_i64);
    let scenario = body_string(&body, "scenario").unwrap_or_else(|| "ura".into());
    let trainee = body_string(&body, "trainee").unwrap_or_else(|| "Special Week".into());
    let speed = body_string(&body, "speed")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1)
        .clamp(1, 100);
    let legacy_factors = body_string(&body, "legacyFactors")
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let deck_supports = body_string(&body, "deckSupports")
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let trace_telemetry = body_string(&body, "traceTelemetry")
        .and_then(|s| s.parse().ok())
        .unwrap_or(false);
    let trace_rng = body_string(&body, "traceRng")
        .and_then(|s| s.parse().ok())
        .unwrap_or(false);
    let dialogue = match body_string(&body, "dialogue")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "off" => DialogueMode::Off,
        "full" => DialogueMode::Full,
        _ => DialogueMode::ChoicesOnly,
    };

    st.settings = SimSettings {
        speed_multiplier: speed,
        trace_telemetry,
        trace_rng,
        dialogue_mode: dialogue,
        ..Default::default()
    };
    let mut engine = SimEngine::create(st.settings.clone());
    let mut meta = RunMeta::new(seed, scenario, trainee);
    meta.legacy_factors = legacy_factors;
    meta.deck_supports = deck_supports;
    engine.start(meta);
    st.engine = Some(engine);
    json_response_str(200, &state_json(st))
}

fn handle_state(st: &ApiState) -> Response<Cursor<Vec<u8>>> {
    if st.engine.is_none() {
        return json_response(404, json!({"error":"no active run"}));
    }
    json_response_str(200, &state_json(st))
}

fn handle_text(st: &ApiState) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_ref() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let lines = TextRenderer::new(st.settings.clone()).render(eng.state(), &[]);
    json_response(200, json!({"text": lines.join("\n")}))
}

fn handle_choices(st: &ApiState) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_ref() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let choices: Vec<Value> = eng
        .choices()
        .into_iter()
        .map(|c| json!({"id": c.id, "label": c.label}))
        .collect();
    json_response(200, json!({"choices": choices}))
}

fn handle_action(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_mut() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let body = parse_body(raw);
    let action = body_string(&body, "action").unwrap_or_else(|| "rest".into());
    let result = eng.step(parse_sim_action(&action));
    json_response(
        200,
        json!({
            "text": result.text_lines.join("\n"),
            "careerEnded": result.career_ended,
        }),
    )
}

fn handle_auto(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_mut() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let body = parse_body(raw);
    let policy_name = body_string(&body, "policy").unwrap_or_else(|| "bot".into());
    let result = if policy_name == "bot" {
        eng.auto_step_scoring()
    } else {
        eng.auto_step_with_policy(default_auto_policy)
    };
    json_response(
        200,
        json!({
            "text": result.text_lines.join("\n"),
            "careerEnded": result.career_ended,
        }),
    )
}

fn handle_fast(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let mult = body_string(&body, "multiplier")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(st.settings.speed_multiplier)
        .clamp(1, 100);
    let policy_name = body_string(&body, "policy").unwrap_or_else(|| "default".into());
    st.settings.speed_multiplier = mult;

    let Some(eng) = st.engine.as_mut() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let mut snap = eng.export();
    snap.settings.speed_multiplier = mult;
    eng.restore(snap);

    if policy_name == "bot" {
        eng.play_to_completion_scoring(500);
    } else {
        eng.play_to_completion(500);
    }
    let s = eng.state();
    json_response(
        200,
        json!({
            "careerEnded": s.career_complete,
            "turn": s.turn,
            "fans": s.fans,
        }),
    )
}

fn handle_telemetry(st: &ApiState) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_ref() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    json_response_str(200, &eng.export_telemetry_json())
}

fn handle_load_content_pack(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let Some(path_str) = body_string(&body, "path") else {
        return json_response(400, json!({"error":"path required"}));
    };
    let root = detect_repo_root().unwrap_or_else(|| std::path::PathBuf::from("."));
    let pack_path = root.join(&path_str);
    let events = ContentPackLoader::load_pack_file(&pack_path);
    if events.is_empty() {
        return json_response(404, json!({"error":"no events in pack"}));
    }
    let loaded = events.len();
    ContentPackRegistry::register(events);
    reinstall_event_catalog(&root);

    if let Some(snap) = st.engine.as_ref().map(|e| e.export()) {
        let mut engine = SimEngine::create(st.settings.clone());
        engine.restore(snap);
        st.engine = Some(engine);
    }

    json_response(
        200,
        json!({
            "loaded": loaded,
            "totalRegistered": ContentPackRegistry::all().len(),
        }),
    )
}

fn reinstall_event_catalog(root: &std::path::Path) {
    let path = FileEventCatalog::default_path(root);
    let base = FileEventCatalog::load(&path);
    let mut pack_events = ContentPackLoader::load_events(root);
    pack_events.extend(ContentPackRegistry::all());
    let merged = base.merge(pack_events);
    if merged.event_count() > 0 {
        install_event_catalog(Arc::new(merged));
    }
}

fn handle_deck_place(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_mut() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let body = parse_body(raw);
    let Some(support_id) = body_string(&body, "supportId") else {
        return json_response(400, json!({"error":"supportId required"}));
    };
    let Some(facility_raw) = body_string(&body, "facility") else {
        return json_response(400, json!({"error":"facility required"}));
    };
    let Some(facility) = DeckPlacement::parse_facility_name(&facility_raw) else {
        return json_response(400, json!({"error":"unknown facility"}));
    };
    if !eng.assign_deck_slot(&support_id, facility) {
        return json_response(409, json!({"error":"cannot place card"}));
    }
    json_response_str(200, &state_json(st))
}
