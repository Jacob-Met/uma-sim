//! Minimal REST API matching Kotlin `SimApiServer` (Phase 7), extended for the web UI.

use crate::career_lab::{
    auto_name, compare_branches, new_session_id, render_json as render_compare_json,
    render_markdown, run_branch, validate_entry_name, ActionOverride, BranchConfig, BranchStore,
    CareerLibrary, LabError, LabResultSummary, SessionInfo,
};
use crate::catalog::event::{install_event_catalog, EventCatalog, FileEventCatalog};
use crate::catalog::factor::FactorCatalog;
use crate::catalog::support::SupportCatalog;
use crate::catalog::trainee::TraineeCatalog;
use crate::content::{ContentPackLoader, ContentPackRegistry};
use crate::deck::DeckPlacement;
use crate::engine::SimEngine;
use crate::factory::{detect_repo_root, init_from_detected_repo};
use crate::policy::default_auto_policy;
use crate::race::RaceModel;
use crate::render::TextRenderer;
use crate::session::parse_sim_action;
use crate::snapshot::{RunSnapshot, RunSnapshotCodec};
use crate::state::{DialogueMode, RunMeta, SimSettings};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One live career session. The server can hold several (the legacy
/// single-run behavior is the default session `""`, labeled "main"), so two
/// forked continuations can be played and resumed independently.
struct SessionSlot {
    id: String,
    label: String,
    engine: SimEngine,
}

struct ApiState {
    /// Defaults for newly started sessions.
    settings: SimSettings,
    /// Default policy name for `/auto` / `/fast` when the client omits it (`bot`|`default`|`external`).
    default_policy: String,
    sessions: HashMap<String, SessionSlot>,
    /// Id of the active session (`""` = the legacy default run).
    active: String,
    library: CareerLibrary,
    branches: BranchStore,
}

/// Start the REST server on `port` (blocking).
pub fn serve(port: u16) {
    serve_opts(port, false);
}

/// Start the REST (+ embedded UI) server; optionally open the default browser.
pub fn serve_opts(port: u16, open_browser: bool) {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).unwrap_or_else(|e| {
        eprintln!("Failed to bind {addr}: {e}");
        std::process::exit(1);
    });
    let url = format!("http://127.0.0.1:{port}/");
    println!("uma-sim REST API on http://127.0.0.1:{port}");
    println!("Web UI: {url}");
    if open_browser {
        open_url(&url);
    }

    // Warm catalogs once so `/v1/catalog/*` works before the first run.
    let _ = init_from_detected_repo(true);

    let state = Arc::new(Mutex::new(ApiState {
        settings: SimSettings::default(),
        default_policy: "bot".into(),
        sessions: HashMap::new(),
        active: String::new(),
        library: CareerLibrary::new(),
        branches: BranchStore::new(),
    }));

    for mut request in server.incoming_requests() {
        let (path, query) = parse_query(request.url());
        let method = request.method().clone();
        if method == Method::Options {
            let _ = request.respond(cors_preflight());
            continue;
        }
        let body = read_body(&mut request);
        let mut st = state.lock().unwrap();
        let response = with_cors(route(&mut st, &method, &path, &query, &body));
        drop(st);
        let _ = request.respond(response);
    }
}

fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

fn route(
    st: &mut ApiState,
    method: &Method,
    path: &str,
    query: &HashMap<String, String>,
    body: &str,
) -> Response<Cursor<Vec<u8>>> {
    match (method, path) {
        (Method::Get, "/v1/health") => handle_health(),
        (Method::Get, "/v1/catalog/scenarios") => handle_catalog_scenarios(),
        (Method::Get, "/v1/catalog/trainees") => handle_catalog_trainees(),
        (Method::Get, "/v1/catalog/supports") => handle_catalog_supports(),
        (Method::Get, "/v1/catalog/factors") => handle_catalog_factors(),
        (Method::Post, "/v1/run/start") => handle_start(st, body),
        (Method::Get, "/v1/run/state") => handle_state(st, query),
        (Method::Get, "/v1/run/text") => handle_text(st, query),
        (Method::Get, "/v1/run/choices") => handle_choices(st, query),
        (Method::Post, "/v1/run/action") => handle_action(st, body, query),
        (Method::Post, "/v1/run/auto") => handle_auto(st, body, query),
        (Method::Post, "/v1/run/fast") => handle_fast(st, body, query),
        (Method::Get, "/v1/run/telemetry") => handle_telemetry(st, query),
        (Method::Post, "/v1/run/load_content_pack") => handle_load_content_pack(st, body),
        (Method::Post, "/v1/run/deck/place") => handle_deck_place(st, body, query),
        (Method::Post, "/v1/run/style") => handle_set_style(st, body, query),
        // Career-lab: sessions (fork/resume), named checkpoint library, branch & compare.
        (Method::Get, "/v1/sessions") => handle_sessions(st),
        (Method::Post, "/v1/session/fork") => handle_session_fork(st, body),
        (Method::Post, "/v1/session/close") => handle_session_close(st, body),
        (Method::Post, "/v1/session/activate") => handle_session_activate(st, body),
        (Method::Get, "/v1/library") => handle_library_list(st),
        (Method::Post, "/v1/library/save") => handle_library_save(st, body),
        (Method::Post, "/v1/library/load") => handle_library_load(st, body),
        (Method::Post, "/v1/library/delete") => handle_library_delete(st, body),
        (Method::Post, "/v1/library/import") => handle_library_import(st, body),
        (Method::Get, "/v1/library/export") => handle_library_export(st, query),
        (Method::Post, "/v1/lab/branch") => handle_lab_branch(st, body),
        (Method::Get, "/v1/lab/branches") => handle_lab_branches(st),
        (Method::Get, "/v1/lab/branch") => handle_lab_branch_get(st, query),
        (Method::Post, "/v1/lab/branch/delete") => handle_lab_branch_delete(st, body),
        (Method::Post, "/v1/lab/compare") => handle_lab_compare(st, body),
        (Method::Get, "/v1/lab/report") => handle_lab_report(st, query),
        _ if is_known_path(path) => method_not_allowed(),
        (Method::Get, _) => serve_static(path),
        _ => json_response(404, json!({"error":"not found"})),
    }
}

fn is_known_path(path: &str) -> bool {
    matches!(
        path,
        "/v1/health"
            | "/v1/catalog/scenarios"
            | "/v1/catalog/trainees"
            | "/v1/catalog/supports"
            | "/v1/catalog/factors"
            | "/v1/run/start"
            | "/v1/run/state"
            | "/v1/run/text"
            | "/v1/run/choices"
            | "/v1/run/action"
            | "/v1/run/auto"
            | "/v1/run/fast"
            | "/v1/run/telemetry"
            | "/v1/run/load_content_pack"
            | "/v1/run/deck/place"
            | "/v1/run/style"
            | "/v1/sessions"
            | "/v1/session/fork"
            | "/v1/session/close"
            | "/v1/session/activate"
            | "/v1/library"
            | "/v1/library/save"
            | "/v1/library/load"
            | "/v1/library/delete"
            | "/v1/library/import"
            | "/v1/library/export"
            | "/v1/lab/branch"
            | "/v1/lab/branches"
            | "/v1/lab/branch/delete"
            | "/v1/lab/compare"
            | "/v1/lab/report"
    )
}

/// Split a request URL into its path and a decoded query map.
fn parse_query(url: &str) -> (String, HashMap<String, String>) {
    let mut parts = url.splitn(2, '?');
    let path = parts.next().unwrap_or("/").to_string();
    let mut query = HashMap::new();
    if let Some(qs) = parts.next() {
        for pair in qs.split('&') {
            let mut kv = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                query.insert(percent_decode(k), percent_decode(v));
            }
        }
    }
    (path, query)
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next().and_then(|c| c.to_digit(16));
            let lo = chars.next().and_then(|c| c.to_digit(16));
            match (hi, lo) {
                (Some(h), Some(l)) => out.push((h * 16 + l) as u8 as char),
                _ => {
                    out.push('%');
                    if let Some(h) = hi {
                        out.push(char::from_digit(h, 16).unwrap_or('?'));
                    }
                    if let Some(l) = lo {
                        out.push(char::from_digit(l, 16).unwrap_or('?'));
                    }
                }
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
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

fn cors_header() -> Header {
    Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap()
}

fn cors_preflight() -> Response<Cursor<Vec<u8>>> {
    Response::from_data(Vec::new())
        .with_status_code(StatusCode(204))
        .with_header(cors_header())
        .with_header(
            Header::from_bytes(
                &b"Access-Control-Allow-Methods"[..],
                &b"GET, POST, OPTIONS"[..],
            )
            .unwrap(),
        )
        .with_header(
            Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type"[..]).unwrap(),
        )
}

fn with_cors(response: Response<Cursor<Vec<u8>>>) -> Response<Cursor<Vec<u8>>> {
    response.with_header(cors_header())
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
    let id = st.active.clone();
    st.sessions
        .get(&id)
        .map(|s| RunSnapshotCodec::encode(&s.engine.export()))
        .unwrap_or_else(|| "{}".to_string())
}

/// Session id targeted by a request: explicit `session` field (POST body) or
/// `?session=` (GET), defaulting to the active session.
fn session_param(body: &Value, query: &HashMap<String, String>) -> String {
    body.get("session")
        .and_then(|v| v.as_str())
        .or_else(|| query.get("session").map(|s| s.as_str()))
        .unwrap_or("")
        .to_string()
}

/// Resolve the target session for a mutating run endpoint. Missing sessions
/// are a 404; a missing default session keeps the legacy "no active run".
fn resolve_session<'a>(
    st: &'a mut ApiState,
    body: &Value,
    query: &HashMap<String, String>,
) -> Result<&'a mut SessionSlot, Response<Cursor<Vec<u8>>>> {
    let explicit = session_param(body, query);
    let id = if explicit.is_empty() {
        st.active.clone()
    } else {
        explicit
    };
    let missing_default = id.is_empty() && !st.sessions.contains_key(&id);
    st.sessions.get_mut(&id).ok_or_else(|| {
        if missing_default {
            json_response(404, json!({"error":"no active run"}))
        } else {
            json_response(404, json!({"error": format!("no such session '{id}'")}))
        }
    })
}

/// Read-only session resolution.
fn resolve_session_ref<'a>(
    st: &'a ApiState,
    query: &HashMap<String, String>,
) -> Result<&'a SessionSlot, Response<Cursor<Vec<u8>>>> {
    let explicit = query.get("session").map(|s| s.as_str()).unwrap_or("");
    let id = if explicit.is_empty() {
        st.active.as_str()
    } else {
        explicit
    };
    let missing_default = id.is_empty() && !st.sessions.contains_key(id);
    st.sessions.get(id).ok_or_else(|| {
        if missing_default {
            json_response(404, json!({"error":"no active run"}))
        } else {
            json_response(404, json!({"error": format!("no such session '{id}'")}))
        }
    })
}

fn session_info(slot: &SessionSlot) -> SessionInfo {
    let s = slot.engine.state();
    SessionInfo {
        id: slot.id.clone(),
        label: slot.label.clone(),
        turn: s.turn,
        phase: s.phase.clone(),
        career_complete: s.career_complete,
        seed: s.meta.seed,
        scenario_id: s.meta.scenario_id.clone(),
        trainee_name: s.meta.trainee_name.clone(),
    }
}

fn lab_error_response(e: &LabError) -> Response<Cursor<Vec<u8>>> {
    let code = match e {
        LabError::NotFound(_) => 404,
        LabError::AlreadyExists(_) => 409,
        LabError::IncompatibleSnapshot(_) => 422,
        LabError::InvalidName(_) | LabError::InvalidSnapshot(_) => 400,
        LabError::Io(_) => 500,
    };
    json_response(code, json!({"error": e.to_string()}))
}

fn choices_json(eng: &SimEngine) -> Vec<Value> {
    eng.choices()
        .into_iter()
        .map(|c| json!({"id": c.id, "label": c.label}))
        .collect()
}

fn step_response(eng: &SimEngine, text: String, career_ended: bool) -> Response<Cursor<Vec<u8>>> {
    let state: Value = serde_json::from_str(&RunSnapshotCodec::encode(&eng.export()))
        .unwrap_or_else(|_| json!({}));
    json_response(
        200,
        json!({
            "text": text,
            "careerEnded": career_ended,
            "state": state,
            "choices": choices_json(eng),
        }),
    )
}

fn handle_health() -> Response<Cursor<Vec<u8>>> {
    let root = detect_repo_root();
    json_response(
        200,
        json!({
            "ok": true,
            "version": API_VERSION,
            "repoRoot": root.is_some(),
            "repoRootPath": root.map(|p| p.display().to_string()),
        }),
    )
}

fn handle_catalog_scenarios() -> Response<Cursor<Vec<u8>>> {
    let _ = init_from_detected_repo(true);
    json_response(
        200,
        json!({
            "items": [
                {"id": "ura", "name": "URA Finale"},
                {"id": "grand_concert", "name": "Grand Live"},
                {"id": "unity", "name": "Unity Cup"},
                {"id": "trackblazer", "name": "Trackblazer"},
            ]
        }),
    )
}

fn handle_catalog_trainees() -> Response<Cursor<Vec<u8>>> {
    let _ = init_from_detected_repo(true);
    let items: Vec<Value> = TraineeCatalog::list_all()
        .into_iter()
        .map(|t| {
            let char_id = t.char_id;
            let icon = char_id.map(|id| {
                format!("https://gametora.com/images/umamusume/characters/icons/chr_icon_{id}.png")
            });
            json!({
                "id": t.id,
                "name": t.name,
                "nameJa": t.name_ja,
                "charId": char_id,
                "iconUrl": icon,
                "playableEn": t.playable_en,
                "baseStats": t.base_stats,
                "aptitudes": crate::catalog::trainee::TraineeCatalog::aptitude_map(&t),
            })
        })
        .collect();
    json_response(200, json!({"items": items}))
}

fn handle_catalog_supports() -> Response<Cursor<Vec<u8>>> {
    let _ = init_from_detected_repo(true);
    let items: Vec<Value> = SupportCatalog::list_all()
        .into_iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "type": s.card_type,
                "rarity": s.rarity,
            })
        })
        .collect();
    json_response(200, json!({"items": items}))
}

fn handle_catalog_factors() -> Response<Cursor<Vec<u8>>> {
    let _ = init_from_detected_repo(true);
    let items: Vec<Value> = FactorCatalog::list_all()
        .into_iter()
        .map(|f| {
            json!({
                "id": f.id,
                "name": f.name,
                "kind": f.category,
                "pinkTag": f.pink_tag,
                "statKey": f.stat_key,
            })
        })
        .collect();
    json_response(200, json!({"items": items}))
}

fn handle_start(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let seed = body_string(&body, "seed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(42_i64);
    let scenario = body_string(&body, "scenario").unwrap_or_else(|| "ura".into());
    // Reject unknown scenarios with 400: the engine silently falls back to the
    // URA plugin while keeping the raw id, producing divergent, silently-wrong
    // runs (see issue #6 secondary item).
    if !crate::is_known_scenario(&scenario) {
        return json_response(
            400,
            json!({"error": format!("unknown scenario '{scenario}'; expected one of: {}", crate::KNOWN_SCENARIO_IDS.join(", "))}),
        );
    }
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
    let legacy_tree = body
        .get("legacyTree")
        .cloned()
        .and_then(|v| serde_json::from_value::<crate::state::LegacyTree>(v).ok())
        .filter(|t| t.is_populated());
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
    // Unknown raceModel values fail with 400 instead of silently flipping to
    // the legacy stub when the documented default is physics.
    let race_model = match body_string(&body, "raceModel") {
        Some(s) => match RaceModel::parse(&s) {
            Some(m) => m,
            None => {
                return json_response(
                    400,
                    json!({"error": format!("unknown raceModel '{s}'; expected one of: {}", RaceModel::KNOWN_RACE_MODELS.join(", "))}),
                )
            }
        },
        None => RaceModel::default(),
    };
    if let Some(policy) = body_string(&body, "policy") {
        let p = policy.to_lowercase();
        if p == "bot" || p == "default" || p == "external" {
            st.default_policy = p;
        }
    }

    st.settings = SimSettings {
        speed_multiplier: speed,
        trace_telemetry,
        trace_rng,
        dialogue_mode: dialogue,
        race_model,
        ..Default::default()
    };
    let mut engine = SimEngine::create(st.settings.clone());
    let mut meta = RunMeta::new(seed, scenario, trainee);
    meta.legacy_factors = legacy_factors;
    meta.legacy_tree = legacy_tree;
    meta.deck_supports = deck_supports;
    if let Some(c) = body_string(&body, "compatibilityScore").and_then(|s| s.parse().ok()) {
        meta.compatibility_score = c;
    } else if let Some(n) = body.get("compatibilityScore").and_then(|v| v.as_i64()) {
        meta.compatibility_score = n as i32;
    }
    if let Some(parents) = body_string(&body, "parentNames") {
        meta.parent_names = parents
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
    }
    engine.start(meta);
    // (Re)start the target session. An explicit `session` id starts an
    // independent career; the default session keeps the legacy behavior.
    let session_id = body_string(&body, "session").unwrap_or_default();
    if !session_id.is_empty() {
        if let Err(e) = validate_entry_name(&session_id) {
            return lab_error_response(&e);
        }
    }
    let label = body_string(&body, "label")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if session_id.is_empty() {
                "main".to_string()
            } else {
                session_id.clone()
            }
        });
    st.sessions.insert(
        session_id.clone(),
        SessionSlot {
            id: session_id.clone(),
            label,
            engine,
        },
    );
    st.active = session_id;
    json_response_str(200, &state_json(st))
}

fn handle_state(st: &ApiState, query: &HashMap<String, String>) -> Response<Cursor<Vec<u8>>> {
    let slot = match resolve_session_ref(st, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    json_response_str(200, &RunSnapshotCodec::encode(&slot.engine.export()))
}

fn handle_text(st: &ApiState, query: &HashMap<String, String>) -> Response<Cursor<Vec<u8>>> {
    let slot = match resolve_session_ref(st, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let eng = &slot.engine;
    let lines = TextRenderer::new(st.settings.clone()).render(eng.state(), &[]);
    json_response(200, json!({"text": lines.join("\n")}))
}

fn handle_choices(st: &ApiState, query: &HashMap<String, String>) -> Response<Cursor<Vec<u8>>> {
    let slot = match resolve_session_ref(st, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    json_response(200, json!({"choices": choices_json(&slot.engine)}))
}

fn handle_action(
    st: &mut ApiState,
    raw: &str,
    query: &HashMap<String, String>,
) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let slot = match resolve_session(st, &body, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let eng = &mut slot.engine;
    let action = body_string(&body, "action").unwrap_or_else(|| "rest".into());
    let result = eng.step(parse_sim_action(&action));
    step_response(eng, result.text_lines.join("\n"), result.career_ended)
}

fn handle_auto(
    st: &mut ApiState,
    raw: &str,
    query: &HashMap<String, String>,
) -> Response<Cursor<Vec<u8>>> {
    let policy_fallback = st.default_policy.clone();
    let body = parse_body(raw);
    let slot = match resolve_session(st, &body, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let eng = &mut slot.engine;
    let policy_name = body_string(&body, "policy").unwrap_or(policy_fallback);
    let result = if policy_name == "bot" {
        eng.auto_step_scoring()
    } else if policy_name.eq_ignore_ascii_case("external") {
        // The API advertises "external" in /v1/run/start, so honor it; when the
        // external policy server is unreachable this fails loudly (503) instead
        // of silently substituting the default heuristic.
        match eng.auto_step_external_checked() {
            Ok(r) => r,
            Err(e) => {
                return json_response(
                    503,
                    json!({"error": format!("external policy unavailable: {e}")}),
                )
            }
        }
    } else {
        eng.auto_step_with_policy(default_auto_policy)
    };
    step_response(eng, result.text_lines.join("\n"), result.career_ended)
}

fn handle_fast(
    st: &mut ApiState,
    raw: &str,
    query: &HashMap<String, String>,
) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let mult = body_string(&body, "multiplier")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(st.settings.speed_multiplier)
        .clamp(1, 100);
    let policy_name = body_string(&body, "policy").unwrap_or_else(|| st.default_policy.clone());

    // Nothing is committed until the run succeeds: neither the API-level
    // speed setting nor the engine. A 404 or 503 leaves both unchanged.
    // The session borrow covers all of `st`, so the response fields are
    // copied out of the block before the API-level setting is committed.
    let (career_ended, turn, fans) = {
        let slot = match resolve_session(st, &body, query) {
            Ok(s) => s,
            Err(r) => return r,
        };
        let eng = &mut slot.engine;
        let before = eng.export();
        let mut snap = before.clone();
        snap.settings.speed_multiplier = mult;
        eng.restore(snap);

        if policy_name == "bot" {
            eng.play_to_completion_scoring(500);
        } else if policy_name.eq_ignore_ascii_case("external") {
            // Honor the requested external policy; when the external policy
            // server is unreachable this fails loudly (503) instead of silently
            // substituting the default heuristic.
            if let Err(e) = eng.play_to_completion_external_checked(500) {
                // Roll the engine back to its pre-request snapshot: the speed
                // multiplier and any steps taken before the policy failed.
                eng.restore(before);
                return json_response(
                    503,
                    json!({"error": format!("external policy unavailable: {e}")}),
                );
            }
        } else {
            eng.play_to_completion(500);
        }
        let s = eng.state();
        (s.career_complete, s.turn, s.fans)
    };
    st.settings.speed_multiplier = mult;
    json_response(
        200,
        json!({
            "careerEnded": career_ended,
            "turn": turn,
            "fans": fans,
        }),
    )
}

fn handle_telemetry(st: &ApiState, query: &HashMap<String, String>) -> Response<Cursor<Vec<u8>>> {
    let slot = match resolve_session_ref(st, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    json_response_str(200, &slot.engine.export_telemetry_json())
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

    // Rebuild every live session against the new catalogs so continued
    // careers and restored checkpoints see the same content.
    for slot in st.sessions.values_mut() {
        let snap = slot.engine.export();
        let mut engine = SimEngine::create(st.settings.clone());
        engine.restore(snap);
        slot.engine = engine;
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

fn handle_deck_place(
    st: &mut ApiState,
    raw: &str,
    query: &HashMap<String, String>,
) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let slot = match resolve_session(st, &body, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let eng = &mut slot.engine;
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
    let snapshot = RunSnapshotCodec::encode(&eng.export());
    json_response_str(200, &snapshot)
}

fn handle_set_style(
    st: &mut ApiState,
    raw: &str,
    query: &HashMap<String, String>,
) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let slot = match resolve_session(st, &body, query) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let eng = &mut slot.engine;
    let Some(style) = body_string(&body, "style") else {
        return json_response(400, json!({"error":"style required (front|pace|late|end)"}));
    };
    let key = style.trim().to_ascii_lowercase();
    if !matches!(key.as_str(), "front" | "pace" | "late" | "end" | "") {
        return json_response(400, json!({"error":"style must be front|pace|late|end"}));
    }
    eng.set_preferred_running_style(if key.is_empty() { None } else { Some(key) });
    let snapshot = RunSnapshotCodec::encode(&eng.export());
    json_response_str(200, &snapshot)
}

// ---------------------------------------------------------------------------
// Career lab: sessions, library, branch & compare
// ---------------------------------------------------------------------------

fn handle_sessions(st: &ApiState) -> Response<Cursor<Vec<u8>>> {
    let mut sessions: Vec<SessionInfo> = st.sessions.values().map(session_info).collect();
    sessions.sort_by(|a, b| a.id.cmp(&b.id));
    json_response(200, json!({"sessions": sessions, "active": st.active}))
}

/// Fork a new session from a library checkpoint (or from another live
/// session). The new session is an independent engine: playing it can never
/// mutate the checkpoint or a sibling session.
fn handle_session_fork(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let mut advisories: Vec<String> = Vec::new();
    let source_label: String;
    let snapshot: RunSnapshot = if let Some(cp) = body_string(&body, "checkpoint") {
        match st.library.load(&cp) {
            Ok((snap, _, adv)) => {
                advisories = adv;
                source_label = format!("checkpoint '{cp}'");
                snap
            }
            Err(e) => return lab_error_response(&e),
        }
    } else {
        let src_id = body_string(&body, "session").unwrap_or_else(|| st.active.clone());
        match st.sessions.get(&src_id) {
            Some(slot) => {
                source_label = format!("session '{}'", slot.label);
                slot.engine.export()
            }
            None => {
                return json_response(404, json!({"error": format!("no such session '{src_id}'")}))
            }
        }
    };

    let new_id = body_string(&body, "id")
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| new_session_id(&source_label));
    if let Err(e) = validate_entry_name(&new_id) {
        return lab_error_response(&e);
    }
    if st.sessions.contains_key(&new_id) {
        return json_response(
            409,
            json!({"error": format!("session '{new_id}' already exists")}),
        );
    }
    let label = body_string(&body, "label")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| new_id.clone());
    let mut engine = SimEngine::create(snapshot.settings.clone());
    engine.restore(snapshot);
    st.sessions.insert(
        new_id.clone(),
        SessionSlot {
            id: new_id.clone(),
            label,
            engine,
        },
    );
    st.active = new_id.clone();
    let info = session_info(&st.sessions[&new_id]);
    json_response(
        200,
        json!({"session": info, "compatAdvisories": advisories}),
    )
}

fn handle_session_close(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let Some(id) = body_string(&body, "session").filter(|s| !s.is_empty()) else {
        return json_response(400, json!({"error":"session required"}));
    };
    if id == st.active {
        // Never strand the client without an active session: fall back to the
        // default session id (which may simply have no run yet).
        st.active = String::new();
    }
    if st.sessions.remove(&id).is_none() {
        return json_response(404, json!({"error": format!("no such session '{id}'")}));
    }
    json_response(200, json!({"closed": id, "active": st.active}))
}

fn handle_session_activate(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let Some(id) = body_string(&body, "session") else {
        return json_response(400, json!({"error":"session required"}));
    };
    if !st.sessions.contains_key(&id) {
        return json_response(404, json!({"error": format!("no such session '{id}'")}));
    }
    st.active = id.clone();
    let info = session_info(&st.sessions[&id]);
    json_response(200, json!({"session": info}))
}

fn handle_library_list(st: &ApiState) -> Response<Cursor<Vec<u8>>> {
    match st.library.list() {
        Ok(entries) => json_response(200, json!({"entries": entries})),
        Err(e) => lab_error_response(&e),
    }
}

/// Save the target session's current run as a named checkpoint. Atomic:
/// a crash mid-write leaves the previous library state intact.
fn handle_library_save(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let empty_query = HashMap::new();
    let snapshot = {
        let slot = match resolve_session(st, &body, &empty_query) {
            Ok(s) => s,
            Err(r) => return r,
        };
        slot.engine.export()
    };
    let name = body_string(&body, "name")
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| auto_name(&snapshot));
    let label = body_string(&body, "label").filter(|s| !s.trim().is_empty());
    let note = body_string(&body, "note").filter(|s| !s.trim().is_empty());
    let overwrite = body
        .get("overwrite")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    match st.library.save(
        &name,
        label.as_deref(),
        note.as_deref(),
        &snapshot,
        overwrite,
    ) {
        Ok(entry) => json_response(200, json!({"entry": entry})),
        Err(e) => lab_error_response(&e),
    }
}

/// Load a checkpoint into the target session (default: active). The library
/// entry is never modified; incompatible snapshots fail loudly (422) without
/// touching the live session.
fn handle_library_load(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let Some(name) = body_string(&body, "name").filter(|s| !s.trim().is_empty()) else {
        return json_response(400, json!({"error":"name required"}));
    };
    let (snapshot, entry, advisories) = match st.library.load(name.trim()) {
        Ok(t) => t,
        Err(e) => return lab_error_response(&e),
    };
    let target = body_string(&body, "session")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| st.active.clone());
    if !target.is_empty() {
        if let Err(e) = validate_entry_name(&target) {
            return lab_error_response(&e);
        }
    }
    let label = body_string(&body, "label")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("from '{}'", entry.name));
    let mut engine = SimEngine::create(snapshot.settings.clone());
    engine.restore(snapshot.clone());
    st.sessions.insert(
        target.clone(),
        SessionSlot {
            id: target.clone(),
            label,
            engine,
        },
    );
    st.active = target;
    let state: Value = serde_json::to_value(&snapshot).unwrap_or_else(|_| json!({}));
    json_response(
        200,
        json!({"state": state, "entry": entry, "compatAdvisories": advisories}),
    )
}

fn handle_library_delete(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let Some(name) = body_string(&body, "name").filter(|s| !s.trim().is_empty()) else {
        return json_response(400, json!({"error":"name required"}));
    };
    match st.library.delete(name.trim()) {
        Ok(()) => json_response(200, json!({"deleted": name.trim()})),
        Err(e) => lab_error_response(&e),
    }
}

/// Import an externally supplied snapshot as a new checkpoint. Validation
/// happens before any write, so a bad import preserves the last good
/// library state.
fn handle_library_import(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let Some(snapshot_value) = body.get("snapshot") else {
        return json_response(400, json!({"error":"snapshot required"}));
    };
    let snapshot_raw = snapshot_value.to_string();
    let name = body_string(&body, "name").filter(|s| !s.trim().is_empty());
    let overwrite = body
        .get("overwrite")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    match st.library.import(&snapshot_raw, name.as_deref(), overwrite) {
        Ok(entry) => json_response(200, json!({"entry": entry})),
        Err(e) => lab_error_response(&e),
    }
}

/// Download a checkpoint's raw snapshot JSON.
fn handle_library_export(
    st: &ApiState,
    query: &HashMap<String, String>,
) -> Response<Cursor<Vec<u8>>> {
    let Some(name) = query.get("name").filter(|s| !s.trim().is_empty()) else {
        return json_response(400, json!({"error":"name required"}));
    };
    match st.library.export_raw(name.trim()) {
        Ok(raw) => json_response_str(200, &raw),
        Err(e) => lab_error_response(&e),
    }
}

/// Resolve a branch source: a library checkpoint name, `session:<id>`, or
/// the default (active) session.
fn resolve_branch_source(
    st: &mut ApiState,
    checkpoint_ref: &str,
) -> Result<(RunSnapshot, String), Response<Cursor<Vec<u8>>>> {
    if let Some(sid) = checkpoint_ref.strip_prefix("session:") {
        let sid = if sid == "active" {
            st.active.clone()
        } else {
            sid.to_string()
        };
        match st.sessions.get(&sid) {
            Some(slot) => Ok((slot.engine.export(), format!("session:{sid}"))),
            None => Err(json_response(
                404,
                json!({"error": format!("no such session '{sid}'")}),
            )),
        }
    } else {
        match st.library.load(checkpoint_ref) {
            Ok((snap, _, _)) => Ok((snap, checkpoint_ref.to_string())),
            Err(e) => Err(lab_error_response(&e)),
        }
    }
}

/// Run one branch to completion from a checkpoint with a built-in policy.
/// Synchronous: the server is busy until the run finishes (a full career is
/// typically seconds with the stub race model).
fn handle_lab_branch(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let checkpoint_ref = body_string(&body, "checkpoint")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "session:active".to_string());
    let (snapshot, checkpoint_name) = match resolve_branch_source(st, checkpoint_ref.trim()) {
        Ok(t) => t,
        Err(r) => return r,
    };
    let policy = body_string(&body, "policy").unwrap_or_else(|| "bot".into());
    if policy != "bot" && policy != "default" {
        return json_response(
            400,
            json!({"error": format!("unknown policy '{policy}'; expected bot|default")}),
        );
    }
    let max_actions = body
        .get("maxActions")
        .and_then(|v| v.as_i64())
        .unwrap_or(500)
        .clamp(1, 500) as i32;
    let overrides: Vec<ActionOverride> = body
        .get("overrides")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|o| {
                    let turn = o.get("turn").and_then(|t| t.as_i64())? as i32;
                    let action_id = o.get("actionId").and_then(|a| a.as_str())?.to_string();
                    Some(ActionOverride { turn, action_id })
                })
                .collect()
        })
        .unwrap_or_default();
    let config = BranchConfig {
        policy: policy.clone(),
        max_actions,
        overrides,
    };
    let name = body_string(&body, "name")
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| format!("{checkpoint_name}-{policy}"));
    let result = run_branch(&snapshot, &name, &checkpoint_name, &config);
    if let Err(e) = st.branches.save(&result) {
        return lab_error_response(&e);
    }
    let summary = LabResultSummary::from(&result);
    json_response(200, json!({"branch": summary, "outcome": result.outcome}))
}

fn handle_lab_branches(st: &ApiState) -> Response<Cursor<Vec<u8>>> {
    match st.branches.list() {
        Ok(results) => {
            let summaries: Vec<LabResultSummary> =
                results.iter().map(LabResultSummary::from).collect();
            json_response(200, json!({"branches": summaries}))
        }
        Err(e) => lab_error_response(&e),
    }
}

fn handle_lab_branch_get(
    st: &ApiState,
    query: &HashMap<String, String>,
) -> Response<Cursor<Vec<u8>>> {
    let Some(id) = query.get("id").filter(|s| !s.trim().is_empty()) else {
        return json_response(400, json!({"error":"id required"}));
    };
    match st.branches.load(id.trim()) {
        Ok(result) => json_response(200, json!({"branch": result})),
        Err(e) => lab_error_response(&e),
    }
}

fn handle_lab_branch_delete(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let Some(id) = body_string(&body, "id").filter(|s| !s.trim().is_empty()) else {
        return json_response(400, json!({"error":"id required"}));
    };
    match st.branches.delete(id.trim()) {
        Ok(()) => json_response(200, json!({"deleted": id.trim()})),
        Err(e) => lab_error_response(&e),
    }
}

fn handle_lab_compare(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let (Some(a_id), Some(b_id)) = (
        body_string(&body, "a").filter(|s| !s.trim().is_empty()),
        body_string(&body, "b").filter(|s| !s.trim().is_empty()),
    ) else {
        return json_response(400, json!({"error":"a and b branch ids required"}));
    };
    let (a, b) = match (st.branches.load(a_id.trim()), st.branches.load(b_id.trim())) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return lab_error_response(&e),
    };
    let comparison = compare_branches(&a, &b);
    let payload = render_compare_json(&comparison);
    json_response_str(200, &payload)
}

/// Downloadable comparison report (`?format=markdown` or `?format=json`).
fn handle_lab_report(st: &ApiState, query: &HashMap<String, String>) -> Response<Cursor<Vec<u8>>> {
    let (Some(a_id), Some(b_id)) = (
        query.get("a").filter(|s| !s.trim().is_empty()),
        query.get("b").filter(|s| !s.trim().is_empty()),
    ) else {
        return json_response(400, json!({"error":"a and b branch ids required"}));
    };
    let (a, b) = match (st.branches.load(a_id.trim()), st.branches.load(b_id.trim())) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return lab_error_response(&e),
    };
    let comparison = compare_branches(&a, &b);
    let format = query
        .get("format")
        .map(|s| s.as_str())
        .unwrap_or("markdown");
    if format == "json" {
        let payload = render_compare_json(&comparison);
        attachment_response(
            200,
            "application/json",
            &format!("branch-compare-{}-vs-{}.json", a.id, b.id),
            payload.as_bytes(),
        )
    } else {
        let payload = render_markdown(&comparison);
        attachment_response(
            200,
            "text/markdown; charset=utf-8",
            &format!("branch-compare-{}-vs-{}.md", a.id, b.id),
            payload.as_bytes(),
        )
    }
}

fn attachment_response(
    code: u16,
    content_type: &str,
    filename: &str,
    bytes: &[u8],
) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(bytes.to_vec())
        .with_status_code(StatusCode(code))
        .with_header(Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap())
        .with_header(
            Header::from_bytes(
                &b"Content-Disposition"[..],
                format!("attachment; filename=\"{filename}\"").as_bytes(),
            )
            .unwrap(),
        )
}

#[cfg(feature = "embed-ui")]
fn content_type_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

fn html_response(code: u16, body: &str, cache_control: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(body.as_bytes().to_vec())
        .with_status_code(StatusCode(code))
        .with_header(
            Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
        )
        .with_header(Header::from_bytes(&b"Cache-Control"[..], cache_control.as_bytes()).unwrap())
}

#[cfg(feature = "embed-ui")]
fn bytes_response(code: u16, path: &str, bytes: Vec<u8>) -> Response<Cursor<Vec<u8>>> {
    let ct = content_type_for(path);
    let is_index = path == "index.html" || path.is_empty() || path == "/";
    let cache = if is_index {
        "no-cache"
    } else {
        "public, max-age=86400"
    };
    Response::from_data(bytes)
        .with_status_code(StatusCode(code))
        .with_header(Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap())
        .with_header(Header::from_bytes(&b"Cache-Control"[..], cache.as_bytes()).unwrap())
}

#[cfg(not(feature = "embed-ui"))]
const UI_PLACEHOLDER: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>uma-sim</title>
<style>body{font-family:system-ui,sans-serif;background:#0f1218;color:#e8eef8;padding:2rem;max-width:40rem;margin:auto}
code{background:#1e2533;padding:.15rem .35rem;border-radius:4px}</style></head>
<body>
<h1>uma-sim UI not embedded</h1>
<p>This binary was built without the <code>embed-ui</code> feature.</p>
<ul>
<li>Dev: <code>cd packages/uma-sim-ui &amp;&amp; npm run dev</code> (proxies to this API)</li>
<li>Release: build UI then <code>cargo build --release --features embed-ui -p uma-sim-core</code></li>
</ul>
<p>REST API is available under <code>/v1/*</code>.</p>
</body></html>"#;

#[cfg(feature = "embed-ui")]
#[derive(rust_embed::Embed)]
#[folder = "../packages/uma-sim-ui/dist/"]
struct UiAssets;

fn serve_static(path: &str) -> Response<Cursor<Vec<u8>>> {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };

    #[cfg(feature = "embed-ui")]
    {
        if let Some(file) = UiAssets::get(rel) {
            return bytes_response(200, rel, file.data.into_owned());
        }
        // SPA fallback for client-side routes
        if !rel.contains('.') {
            if let Some(index) = UiAssets::get("index.html") {
                return bytes_response(200, "index.html", index.data.into_owned());
            }
        }
        return html_response(
            404,
            "<!doctype html><title>404</title><h1>Not found</h1>",
            "no-cache",
        );
    }

    #[cfg(not(feature = "embed-ui"))]
    {
        let _ = rel;
        html_response(200, UI_PLACEHOLDER, "no-cache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    // All tests in this module share `policy_external::EXTERNAL_TEST_LOCK`
    // with the `policy_external` stub tests. Rust runs a test binary's tests
    // in threads of one process, and the REST server reads the process-global
    // `UMA_POLICY_CMD` per request — so any test that starts a server must
    // hold the *same* lock the stub tests use when they *set* the variable.
    // A private second lock here left a cross-module race: a request could
    // observe a stub's `UMA_POLICY_CMD` and then watch it vanish mid-flight,
    // panicking the server thread (500 instead of 400) or succeeding when a
    // 503 was expected. Hold the lock for the whole server interaction.
    // (The stub positive-path test lives in its own integration test binary
    // precisely so it cannot interleave with these.)
    use crate::policy_external::EXTERNAL_TEST_LOCK;

    /// Serializes the career-lab REST tests. Both spin up a server that stores
    /// checkpoints in the cwd-relative `.uma-sim/library/`, so they must not
    /// interleave with each other (file names are fixed per test).
    static LAB_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Start a run on the test server; `policy_field` is `""` or e.g.
    /// `",\"policy\":\"external\""`.
    fn start_run(port: u16, policy_field: &str) -> (u16, String) {
        let body = format!(
            "{{\"seed\":7,\"scenario\":\"ura\",\"trainee\":\"Special Week\",\"raceModel\":\"stub\"{policy_field}}}"
        );
        http_post(port, "/v1/run/start", &body)
    }

    /// Assert a response body is a JSON object with a string "error" field
    /// naming `needle`.
    fn assert_json_error(body: &str, needle: &str) {
        let v: serde_json::Value = serde_json::from_str(body).expect("error body must be JSON");
        let msg = v
            .get("error")
            .and_then(|e| e.as_str())
            .expect("error body must have a string \"error\" field");
        assert!(
            msg.contains(needle),
            "error message should name '{needle}'; got: {msg}"
        );
    }

    /// Raw HTTP round-trip returning the full response (status line + headers
    /// + body), for tests that need to inspect headers.
    fn http_raw(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to test server");
        stream.write_all(request.as_bytes()).unwrap();
        let mut buf = String::new();
        stream.read_to_string(&mut buf).unwrap();
        buf
    }

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
        let (headers, body) = match buf.split_once("\r\n\r\n") {
            Some(pair) => pair,
            None => return (status, String::new()),
        };
        let content_length = headers.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            if k.eq_ignore_ascii_case("content-length") {
                v.trim().parse::<usize>().ok()
            } else {
                None
            }
        });
        let body = if let Some(n) = content_length {
            body.as_bytes()
                .get(..n)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_else(|| body.to_string())
        } else if headers
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
        {
            decode_chunked(body)
        } else {
            body.to_string()
        };
        (status, body)
    }

    fn decode_chunked(raw: &str) -> String {
        let mut out = String::new();
        let mut rest = raw;
        while let Some((size_line, after)) = rest.split_once("\r\n") {
            let size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
            if size == 0 {
                break;
            }
            let chunk = &after[..size.min(after.len())];
            out.push_str(chunk);
            rest = after.get(size..).unwrap_or("");
            if rest.starts_with("\r\n") {
                rest = &rest[2..];
            }
        }
        out
    }

    /// Remove this test's checkpoint files from the cwd-relative library dir so
    /// repeated runs are idempotent (the REST server stores checkpoints
    /// cwd-relative, and the file names these tests use are fixed).
    fn scrub_checkpoints(names: &[&str]) {
        let dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".uma-sim")
            .join("library");
        for name in names {
            for ext in ["snapshot.json", "meta.json"] {
                let _ = std::fs::remove_file(dir.join(format!("{name}.{ext}")));
            }
        }
    }

    fn http_get(port: u16, path: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to test server");
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .unwrap();
        let mut buf = String::new();
        stream.read_to_string(&mut buf).unwrap();
        parse_http(&buf)
    }

    fn http_post(port: u16, path: &str, json_body: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to test server");
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

    #[test]
    fn health_and_catalogs_and_action_include_state() {
        // Server threads read UMA_POLICY_CMD per request: hold the shared
        // lock even though this test never sets it.
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);

        let (status, body) = http_get(port, "/v1/health");
        assert_eq!(status, 200);
        let health: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(health["ok"], true);
        assert!(health["version"].as_str().is_some());
        // When run from the repo (CI / local), repo root should be detected.
        assert_eq!(
            health["repoRoot"], true,
            "expected detect_repo_root to succeed from cargo test cwd"
        );

        for path in [
            "/v1/catalog/scenarios",
            "/v1/catalog/trainees",
            "/v1/catalog/supports",
            "/v1/catalog/factors",
        ] {
            let (status, body) = http_get(port, path);
            assert_eq!(status, 200, "{path}");
            let v: Value = serde_json::from_str(&body).unwrap();
            let items = v["items"].as_array().expect("items array");
            assert!(!items.is_empty(), "{path} should be non-empty");
        }

        let (status, _) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":42,"scenario":"ura","trainee":"Special Week","raceModel":"stub","policy":"default"}"#,
        );
        assert_eq!(status, 200);

        let (status, body) = http_get(port, "/v1/run/choices");
        assert_eq!(status, 200);
        let choices: Value = serde_json::from_str(&body).unwrap();
        let first = choices["choices"][0]["id"].as_str().unwrap_or("rest");

        let (status, body) = http_post(
            port,
            "/v1/run/action",
            &format!(r#"{{"action":"{first}"}}"#),
        );
        assert_eq!(status, 200);
        let step: Value = serde_json::from_str(&body).unwrap();
        assert!(step.get("state").is_some(), "action must return state");
        assert!(
            step["choices"].as_array().is_some(),
            "action must return choices"
        );
        assert!(step.get("text").is_some());
        assert!(step.get("careerEnded").is_some());
    }

    #[test]
    fn start_rejects_unknown_scenario_and_race_model() {
        // Server threads read UMA_POLICY_CMD per request: hold the shared
        // lock even though this test never sets it.
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);

        // Unknown scenario: 400 naming the problem, never a silent URA fallback.
        let (status, body) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":42,"scenario":"foo","trainee":"Special Week"}"#,
        );
        assert_eq!(
            status, 400,
            "unknown scenario must be rejected, got: {body}"
        );
        let v: Value = serde_json::from_str(&body).unwrap();
        assert!(
            v["error"]
                .as_str()
                .unwrap_or("")
                .contains("unknown scenario"),
            "error names the problem, got: {body}"
        );

        // Unknown raceModel: 400, never a silent flip to the legacy stub.
        let (status, body) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":42,"scenario":"ura","trainee":"Special Week","raceModel":"foo"}"#,
        );
        assert_eq!(
            status, 400,
            "unknown raceModel must be rejected, got: {body}"
        );
        let v: Value = serde_json::from_str(&body).unwrap();
        assert!(
            v["error"]
                .as_str()
                .unwrap_or("")
                .contains("unknown raceModel"),
            "error names the problem, got: {body}"
        );

        // Valid aliases and canonical values keep working.
        for payload in [
            r#"{"seed":42,"scenario":"tb"}"#,
            r#"{"seed":42,"scenario":"Grand Concert"}"#,
            r#"{"seed":42,"scenario":"ura","raceModel":"stub"}"#,
            r#"{"seed":42,"scenario":"ura","raceModel":"physics"}"#,
            r#"{"seed":42}"#,
        ] {
            let (status, body) = http_post(port, "/v1/run/start", payload);
            assert_eq!(status, 200, "valid payload rejected: {payload} -> {body}");
        }
    }

    /// Regression test for uma-sim issue #6: with no external policy server
    /// configured (`UMA_POLICY_CMD` unset), requesting `policy=external`
    /// through REST must fail loudly instead of silently downgrading to the
    /// default heuristic with 200 OK. This matches the CLI, which panics
    /// naming the missing env var.
    #[test]
    fn policy_external_without_policy_cmd_fails_loudly_not_silently() {
        // Serialize with the policy_external stub tests: they set
        // UMA_POLICY_CMD and populate the global EXTERNAL slot. Hold the lock
        // for the whole interaction: the stub positive-path tests set
        // UMA_POLICY_CMD, and the per-request reads must not see a value set
        // by another test. (A single guard: std Mutex is not reentrant, so a
        // second lock() of the same mutex in this test would self-deadlock.)
        let _lock = crate::policy_external::EXTERNAL_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::policy_external::reset_external_for_tests();
        // Precondition: no external policy server is configured.
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);

        // Session accepts "external" as the stored default policy.
        let (status, _) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":7,"scenario":"ura","trainee":"Special Week","raceModel":"stub","policy":"external"}"#,
        );
        assert_eq!(status, 200);

        // /v1/run/fast with policy=external: must NOT be 200 with the default
        // heuristic's output; it must be a loud 503 naming the misconfiguration.
        let (status, body) = http_post(port, "/v1/run/fast", r#"{"policy":"external"}"#);
        assert_eq!(
            status, 503,
            "REST /v1/run/fast with policy=external and no policy server must not 200; got {status}: {body}"
        );
        assert!(
            body.contains("UMA_POLICY_CMD"),
            "error should name the missing config; got: {body}"
        );

        // The session default ("external") is inherited by /v1/run/auto when
        // the request omits "policy": also loud, not a silent downgrade.
        let (status, body) = http_post(port, "/v1/run/auto", r#"{}"#);
        assert_eq!(
            status, 503,
            "REST /v1/run/auto inheriting policy=external must not 200; got {status}: {body}"
        );
        assert!(body.contains("UMA_POLICY_CMD"), "got: {body}");

        // Explicit policy=external on /v1/run/auto as well.
        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"external"}"#);
        assert_eq!(
            status, 503,
            "REST /v1/run/auto with policy=external must not 200; got {status}: {body}"
        );
        assert!(body.contains("UMA_POLICY_CMD"), "got: {body}");
    }

    /// T1 — unknown policy values on /v1/run/auto and /v1/run/fast.
    ///
    /// CURRENT BEHAVIOR (pinned, not blessed): `"foo"` silently falls through
    /// to the default heuristic with 200 OK — the same silent-downgrade class
    /// as issue #6, one match arm over.
    /// TODO (needs Jacob's decision (a)): amend the residual `else` arms in
    /// `handle_auto`/`handle_fast` to 400 on unknown policies; when that
    /// lands, flip these assertions to 400 + a JSON error naming the value.
    #[test]
    fn policy_unknown_on_auto_and_fast() {
        // Server threads read UMA_POLICY_CMD per request: hold the shared
        // lock even though this test never sets it.
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);

        let (status, _) = start_run(port, "");
        assert_eq!(status, 200);

        // /v1/run/auto with an unknown policy: currently 200 + default heuristic.
        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"foo"}"#);
        assert_eq!(
            status, 200,
            "current behavior: unknown policy 200s; got {status}: {body}"
        );
        assert!(
            body.contains("\"careerEnded\""),
            "expected a normal step body, not an error; got: {body}"
        );

        // /v1/run/fast with an unknown policy: currently 200 after a full
        // default-heuristic career.
        let (status, body) = http_post(port, "/v1/run/fast", r#"{"policy":"foo"}"#);
        assert_eq!(
            status, 200,
            "current behavior: unknown policy 200s; got {status}: {body}"
        );
        assert!(
            body.contains("\"turn\""),
            "expected a fast-run body, not an error; got: {body}"
        );
    }

    /// T2 — case variants of policy names on /v1/run/auto.
    ///
    /// `"external"` matching is case-insensitive (decided behavior on
    /// b3cd202): `"External"`/`"EXTERNAL"` take the external arm, so with no
    /// policy server configured they 503 loudly. `"bot"`/`"default"` matching
    /// is exact-case, so `"BOT"` silently falls through to the default
    /// heuristic.
    /// TODO (needs Jacob's decision (b)): normalize `bot`/`default` matching
    /// to case-insensitive for consistency; when that lands, `"BOT"` should
    /// behave exactly like `"bot"`.
    #[test]
    fn policy_case_variants() {
        // Hold for the whole test, not per-iteration: the second half spawns
        // servers too, and must not interleave with the stub tests.
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        // "External"/"EXTERNAL" hit the external arm (case-insensitive).
        for variant in ["External", "EXTERNAL"] {
            std::env::remove_var("UMA_POLICY_CMD");

            let port = free_port();
            thread::spawn(move || serve(port));
            wait_ready(port);
            let (status, _) = start_run(port, "");
            assert_eq!(status, 200);

            let (status, body) = http_post(
                port,
                "/v1/run/auto",
                &format!(r#"{{"policy":"{variant}"}}"#),
            );
            assert_eq!(
                status, 503,
                "policy={variant}: expected loud 503; got {status}: {body}"
            );
            assert!(body.contains("UMA_POLICY_CMD"), "got: {body}");
        }

        // "BOT" does not match the exact-case "bot" arm: currently 200 with
        // the default heuristic. Pinned with a flag (see TODO above).
        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, _) = start_run(port, "");
        assert_eq!(status, 200);

        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"BOT"}"#);
        assert_eq!(
            status, 200,
            "current behavior: \"BOT\" 200s via default heuristic; got {status}: {body}"
        );
        assert!(body.contains("\"careerEnded\""), "got: {body}");

        // "Default" likewise falls to the default arm (which is the default).
        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"Default"}"#);
        assert_eq!(status, 200, "got {status}: {body}");
        assert!(body.contains("\"careerEnded\""), "got: {body}");
    }

    /// T3 — /v1/run/start with an unknown policy.
    ///
    /// CURRENT BEHAVIOR (pinned, not blessed): 200, and the junk value is
    /// silently ignored — the previous default is kept (forward-compat by
    /// accident, not by contract).
    /// TODO (needs Jacob's decision (a)): reject unknown policies at session
    /// creation with 400; when that lands, flip this assertion to 400.
    #[test]
    fn start_rejects_unknown_policy() {
        // Server threads read UMA_POLICY_CMD per request: hold the shared
        // lock even though this test never sets it.
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);

        let (status, _) = start_run(port, r#","policy":"foo""#);
        assert_eq!(
            status, 200,
            "current behavior: unknown start policy 200s; got {status}"
        );

        // The session is still usable on the default path afterwards.
        let (status, body) = http_post(port, "/v1/run/auto", r#"{}"#);
        assert_eq!(status, 200, "got {status}: {body}");
        assert!(body.contains("\"careerEnded\""), "got: {body}");
    }

    /// T4 — error-shape contract for loud policy failures.
    ///
    /// Clients (e.g. packages/uma-sim-ui) need a stable shape to surface
    /// loud failures as error states: a JSON body with a string "error" field
    /// and a JSON Content-Type. (The 400 half of this contract arrives with
    /// the unknown→400 amendment; this test pins the 503 half now.)
    #[test]
    fn policy_error_shape_contract() {
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, _) = start_run(port, "");
        assert_eq!(status, 200);

        // Raw request so the response headers (not just the body) can be
        // inspected.
        let req_body = r#"{"policy":"external"}"#;
        let raw = http_raw(
            port,
            &format!(
                "POST /v1/run/auto HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{req_body}",
                req_body.len()
            ),
        );
        assert!(
            raw.starts_with("HTTP/1.1 503"),
            "expected a 503 status line; got: {}",
            raw.lines().next().unwrap_or("")
        );
        let (headers, body) = raw
            .split_once("\r\n\r\n")
            .expect("response must have a header/body split");
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("content-type: application/json"),
            "loud policy errors must be JSON; headers were: {headers}"
        );
        assert_json_error(body, "UMA_POLICY_CMD");
    }

    /// T5 — the session stays usable after a loud policy failure.
    ///
    /// Guards the validation-before-mutation ordering: a 503 from the
    /// external arm must not poison the engine, so a subsequent default-policy
    /// step returns a normal 200 with a valid step body.
    #[test]
    fn session_usable_after_policy_503() {
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, _) = start_run(port, "");
        assert_eq!(status, 200);

        // Loud failure on /v1/run/fast with policy=external and no server.
        // (Fails immediately inside ensure_external; no 500-iteration run.)
        let (status, body) = http_post(port, "/v1/run/fast", r#"{"policy":"external"}"#);
        assert_eq!(status, 503, "got {status}: {body}");
        assert!(body.contains("UMA_POLICY_CMD"), "got: {body}");

        // The session is uncorrupted: a default-policy step works normally.
        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"default"}"#);
        assert_eq!(
            status, 200,
            "session was poisoned by the 503 path; got {status}: {body}"
        );
        assert!(body.contains("\"text\""), "got: {body}");
        assert!(body.contains("\"careerEnded\""), "got: {body}");
    }

    /// T9 — unrelated endpoints are unaffected by policy errors.
    ///
    /// Regression net: after a loud policy failure, /v1/run/choices and
    /// /v1/health still answer 200 with valid shapes.
    #[test]
    fn run_endpoints_unaffected_by_policy_errors() {
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, _) = start_run(port, "");
        assert_eq!(status, 200);

        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"external"}"#);
        assert_eq!(status, 503, "got {status}: {body}");

        let (status, body) = http_get(port, "/v1/run/choices");
        assert_eq!(status, 200, "got {status}: {body}");
        assert!(body.contains("\"choices\""), "got: {body}");

        let (status, body) = http_get(port, "/v1/health");
        assert_eq!(status, 200, "got {status}: {body}");
        assert!(body.contains("\"ok\":true"), "got: {body}");
    }

    /// T100 — career library + session fork + branch & compare over HTTP.
    ///
    /// Exercises the full laboratory flow against a live test server: start a
    /// run, save a named checkpoint, fork two independent sessions, verify
    /// they evolve independently, run two lab branches with different
    /// overrides, and compare them. Also pins the error shapes for duplicate
    /// saves, bad names, and missing checkpoints.
    #[test]
    fn lab_library_fork_branch_compare_flow() {
        let _guard = LAB_LOCK.lock().unwrap();
        // Server threads read UMA_POLICY_CMD per request: hold the shared
        // lock for the whole server interaction (see EXTERNAL_TEST_LOCK).
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        scrub_checkpoints(&["lab-e2e"]);
        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);

        let (status, _) = start_run(port, "");
        assert_eq!(status, 200);

        // Play a few scoring steps so the checkpoint is mid-career.
        for _ in 0..4 {
            let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"bot"}"#);
            assert_eq!(status, 200, "got {status}: {body}");
        }

        // Save a named checkpoint; the entry shows seed/scenario/turn.
        let (status, body) = http_post(
            port,
            "/v1/library/save",
            r#"{"name":"lab-e2e","label":"E2E checkpoint"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["entry"]["name"], "lab-e2e");
        assert_eq!(v["entry"]["seed"], 7);
        assert_eq!(v["entry"]["scenarioId"], "ura");
        let cp_turn = v["entry"]["turn"].as_i64().unwrap();

        // Duplicate save without overwrite: 409. Bad name: 400.
        let (status, body) = http_post(port, "/v1/library/save", r#"{"name":"lab-e2e"}"#);
        assert_eq!(status, 409, "got {status}: {body}");
        assert_json_error(&body, "already exists");
        let (status, body) = http_post(port, "/v1/library/save", r#"{"name":"bad name"}"#);
        assert_eq!(status, 400, "got {status}: {body}");

        // Library lists the checkpoint.
        let (status, body) = http_get(port, "/v1/library");
        assert_eq!(status, 200, "got {status}: {body}");
        assert!(body.contains("\"lab-e2e\""), "got: {body}");

        // Fork two sessions from the checkpoint; both start at the checkpoint turn.
        for sid in ["s1", "s2"] {
            let (status, body) = http_post(
                port,
                "/v1/session/fork",
                &format!(r#"{{"checkpoint":"lab-e2e","id":"{sid}","label":"{sid}"}}"#),
            );
            assert_eq!(status, 200, "fork {sid}: got {status}: {body}");
            let v: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(v["session"]["turn"], cp_turn);
        }

        // Play the sessions differently: rest on s1, auto on s2. They must
        // evolve independently of each other and of the checkpoint.
        let (status, _) = http_post(
            port,
            "/v1/run/action",
            r#"{"session":"s1","action":"rest"}"#,
        );
        assert_eq!(status, 200);
        let (status, _) = http_post(port, "/v1/run/auto", r#"{"session":"s2","policy":"bot"}"#);
        assert_eq!(status, 200);
        let (status, a_body) = http_get(port, "/v1/run/state?session=s1");
        assert_eq!(status, 200);
        let (status, b_body) = http_get(port, "/v1/run/state?session=s2");
        assert_eq!(status, 200);
        let (a_v, b_v): (Value, Value) = (
            serde_json::from_str(&a_body).unwrap(),
            serde_json::from_str(&b_body).unwrap(),
        );
        assert_ne!(
            a_v["state"]["energy"], b_v["state"]["energy"],
            "forked sessions must evolve independently"
        );

        // The checkpoint still loads at its original turn: sessions never
        // mutated it.
        let (status, body) = http_post(
            port,
            "/v1/library/load",
            r#"{"name":"lab-e2e","session":"s3"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["state"]["state"]["turn"], cp_turn);

        // Unknown session / checkpoint: 404 with a JSON error.
        let (status, body) = http_get(port, "/v1/run/state?session=nope");
        assert_eq!(status, 404, "got {status}: {body}");
        assert_json_error(&body, "no such session");
        let (status, body) = http_post(
            port,
            "/v1/lab/branch",
            r#"{"checkpoint":"nope","policy":"bot"}"#,
        );
        assert_eq!(status, 404, "got {status}: {body}");

        // Two lab branches with different overrides on the checkpoint turn.
        let branch = |name: &str, action: &str| {
            http_post(
                port,
                "/v1/lab/branch",
                &format!(
                    r#"{{"checkpoint":"lab-e2e","name":"{name}","policy":"bot","overrides":[{{"turn":{cp_turn},"actionId":"{action}"}}]}}"#,
                ),
            )
        };
        let (status, body) = branch("e2e-rest", "rest");
        assert_eq!(status, 200, "got {status}: {body}");
        let a_id = serde_json::from_str::<Value>(&body).unwrap()["branch"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let (status, body) = branch("e2e-train", "train_speed");
        assert_eq!(status, 200, "got {status}: {body}");
        let b_id = serde_json::from_str::<Value>(&body).unwrap()["branch"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Branches are listed, and the comparison finds the decision
        // divergence on the checkpoint turn.
        let (status, body) = http_get(port, "/v1/lab/branches");
        assert_eq!(status, 200, "got {status}: {body}");
        assert!(body.contains(&a_id) && body.contains(&b_id), "got: {body}");

        let (status, body) = http_post(
            port,
            "/v1/lab/compare",
            &format!(r#"{{"a":"{a_id}","b":"{b_id}"}}"#),
        );
        assert_eq!(status, 200, "got {status}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["firstDivergence"]["kind"], "decision");
        assert_eq!(v["firstDivergence"]["turn"], cp_turn);
        assert_eq!(v["sameCheckpoint"], true);
        assert!(v["caveats"].as_array().unwrap().len() >= 3);

        // The downloadable report renders with the checkpoint turn fixed.
        let raw = http_raw(
            port,
            &format!("GET /v1/lab/report?a={a_id}&b={b_id}&format=markdown HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
        );
        assert!(
            raw.starts_with("HTTP/1.1 200"),
            "got: {}",
            raw.lines().next().unwrap_or("")
        );
        assert!(
            raw.to_ascii_lowercase()
                .contains("content-type: text/markdown"),
            "report must be markdown"
        );
        assert!(raw.contains("# Branch comparison: e2e-rest vs e2e-train"));
        assert!(raw.contains(&format!("turn {cp_turn}")));
        scrub_checkpoints(&["lab-e2e"]);
    }

    /// T100 — import validation happens before any write (M6).
    #[test]
    fn lab_library_import_failure_is_nondestructive() {
        let _guard = LAB_LOCK.lock().unwrap();
        // Server threads read UMA_POLICY_CMD per request: hold the shared
        // lock for the whole server interaction (see EXTERNAL_TEST_LOCK).
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        scrub_checkpoints(&["keep"]);
        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);

        let (status, _) = start_run(port, "");
        assert_eq!(status, 200);
        let (status, body) = http_post(port, "/v1/library/save", r#"{"name":"keep"}"#);
        assert_eq!(status, 200, "got {status}: {body}");

        // Wrong-shape snapshot: 400, and the good entry survives.
        let (status, body) = http_post(
            port,
            "/v1/library/import",
            r#"{"name":"bad","snapshot":{"nope":true}}"#,
        );
        assert_eq!(status, 400, "got {status}: {body}");
        assert_json_error(&body, "not imported");

        let (status, body) = http_get(port, "/v1/library");
        assert_eq!(status, 200, "got {status}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        let names: Vec<&str> = v["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["keep"]);
        scrub_checkpoints(&["keep"]);
    }

    /// Read `settings.speedMultiplier` and `state.turn` from /v1/run/state.
    fn speed_and_turn(port: u16) -> (i64, i64) {
        let (status, body) = http_get(port, "/v1/run/state");
        assert_eq!(status, 200, "got {status}: {body}");
        let v: serde_json::Value = serde_json::from_str(&body).expect("state must be JSON");
        let speed = v["settings"]["speedMultiplier"]
            .as_i64()
            .unwrap_or_else(|| panic!("no settings.speedMultiplier in: {body}"));
        let turn = v["state"]["turn"]
            .as_i64()
            .unwrap_or_else(|| panic!("no state.turn in: {body}"));
        (speed, turn)
    }

    /// A 503 from /v1/run/fast must not change the speed setting (neither the
    /// engine's nor the server default used when `multiplier` is omitted),
    /// while the success path still applies the requested multiplier.
    /// Regression test for the PR #42 merge silently reverting the PR #38
    /// 503 speed-rollback fix (restored here with #45 session plumbing).
    #[test]
    fn fast_503_leaves_speed_unchanged_success_sets_it() {
        // Server threads read UMA_POLICY_CMD per request: hold the shared
        // lock for the whole server interaction (see EXTERNAL_TEST_LOCK).
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, body) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":7,"scenario":"ura","trainee":"Special Week","raceModel":"stub","speed":"3"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        let (speed0, turn0) = speed_and_turn(port);
        assert_eq!(speed0, 3);

        // Failure: external policy with no server configured -> 503.
        let (status, body) = http_post(
            port,
            "/v1/run/fast",
            r#"{"policy":"external","multiplier":"50"}"#,
        );
        assert_eq!(status, 503, "got {status}: {body}");
        assert_eq!(
            speed_and_turn(port),
            (speed0, turn0),
            "503 from /v1/run/fast must leave speed and turn unchanged"
        );

        // Server-level default is also unchanged: omitting `multiplier`
        // falls back to it, so a successful run keeps speed 3, not 50.
        let (status, body) = http_post(port, "/v1/run/fast", r#"{"policy":"default"}"#);
        assert_eq!(status, 200, "got {status}: {body}");
        assert_eq!(speed_and_turn(port).0, 3);

        // Success path still sets the requested multiplier.
        let (status, body) = http_post(
            port,
            "/v1/run/fast",
            r#"{"policy":"default","multiplier":"7"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        assert_eq!(speed_and_turn(port).0, 7);
    }

    /// Write a stub external-policy shell script that answers the ping,
    /// answers `answers` choose requests with a rest action, then dies
    /// (simulating a policy server that fails mid-run, not just at config).
    /// Caller must hold EXTERNAL_TEST_LOCK.
    fn write_midrun_kill_stub(answers: usize) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("umasim-midrun-stub-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = format!(
            "#!/bin/sh\nn=0\nwhile IFS= read -r line; do\n  case \"$line\" in\n    *ping*) echo '{{\"ok\":true}}' ;;\n    *quit*) exit 0 ;;\n    *) n=$((n+1)); if [ \"$n\" -gt {answers} ]; then exit 0; fi; echo '{{\"kind\":\"rest\",\"payload\":null}}' ;;\n  esac\ndone\n"
        );
        let path = dir.join("midkill.sh");
        std::fs::write(&path, script).unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        path
    }

    /// A 503 that arrives MID-RUN (policy server dies after N successful
    /// steps, not at config time) must roll the engine back to its
    /// pre-request snapshot: partial steps are undone and the speed
    /// multiplier is restored. The existing 503 tests only pin the
    /// immediate-failure (no-config) case; this covers the
    /// `play_to_completion_external_checked` fallible-step loop.
    #[test]
    fn fast_503_midrun_failure_rolls_back_engine() {
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        let old_cmd = std::env::var("UMA_POLICY_CMD").ok();
        crate::policy_external::reset_external_for_tests();

        let stub = write_midrun_kill_stub(2);
        std::env::set_var("UMA_POLICY_CMD", stub.to_string_lossy().into_owned());

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, body) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":7,"scenario":"ura","trainee":"Special Week","raceModel":"stub","speed":"3"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        let (speed0, turn0) = speed_and_turn(port);
        assert_eq!(speed0, 3);

        // The stub dies after 2 successful chooses: the request must fail
        // loudly, and the 2 partial steps must be rolled back.
        let (status, body) = http_post(
            port,
            "/v1/run/fast",
            r#"{"policy":"external","multiplier":"50"}"#,
        );
        assert_eq!(
            status, 503,
            "mid-run policy death must 503, not silently complete; got {status}: {body}"
        );
        assert!(
            body.contains("external policy"),
            "error should name the external-policy layer; got: {body}"
        );
        assert_eq!(
            speed_and_turn(port),
            (speed0, turn0),
            "mid-run 503 must undo partial steps AND restore the speed multiplier"
        );

        // Server-level default untouched, and the session is still usable.
        let (status, body) = http_post(port, "/v1/run/fast", r#"{"policy":"default"}"#);
        assert_eq!(status, 200, "got {status}: {body}");
        assert_eq!(speed_and_turn(port).0, 3);
        let (status, body) = http_post(port, "/v1/run/auto", r#"{"policy":"default"}"#);
        assert_eq!(status, 200, "got {status}: {body}");
        assert!(body.contains("\"careerEnded\""), "got: {body}");

        std::env::set_var("UMA_POLICY_CMD", old_cmd.as_deref().unwrap_or(""));
        if old_cmd.is_none() {
            std::env::remove_var("UMA_POLICY_CMD");
        }
        crate::policy_external::reset_external_for_tests();
    }

    /// /v1/run/fast with an omitted `policy` inherits the session's default
    /// policy. When that default is "external" and no policy server is
    /// configured, the request must fail loudly (503), not silently run a
    /// full default-heuristic career — the fast-side analog of the /auto
    /// inheritance test.
    #[test]
    fn fast_inherits_session_default_external_policy() {
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, body) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":7,"scenario":"ura","trainee":"Special Week","raceModel":"stub","policy":"external"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        let before = speed_and_turn(port);

        // No `policy` field: the session default ("external") applies.
        let (status, body) = http_post(port, "/v1/run/fast", r#"{}"#);
        assert_eq!(
            status, 503,
            "fast inheriting policy=external must not silently 200; got {status}: {body}"
        );
        assert!(body.contains("UMA_POLICY_CMD"), "got: {body}");
        assert_eq!(
            speed_and_turn(port),
            before,
            "inherited-policy 503 must leave speed and turn unchanged"
        );
    }

    /// /v1/run/fast for an unknown session id is a 404 before any mutation:
    /// neither the engine nor the API-level speed setting may change, and the
    /// error body must name the missing session.
    #[test]
    fn fast_404_unknown_session_leaves_speed_unchanged() {
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, body) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":7,"scenario":"ura","trainee":"Special Week","raceModel":"stub","speed":"3"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        let before = speed_and_turn(port);

        let (status, body) = http_post(
            port,
            "/v1/run/fast",
            r#"{"session":"no-such-session","policy":"default","multiplier":"50"}"#,
        );
        assert_eq!(status, 404, "got {status}: {body}");
        assert_json_error(&body, "no-such-session");
        assert_eq!(
            speed_and_turn(port),
            before,
            "404 fast must leave speed and turn unchanged"
        );
    }

    /// Multiplier parsing on the success path: out-of-range values clamp to
    /// [1, 100] (not an error), and a non-numeric value falls back to the
    /// current server default. Pins the parsing surface adjacent to the
    /// 503 speed-rollback regression.
    #[test]
    fn fast_multiplier_clamp_and_fallback() {
        let _env_guard = EXTERNAL_TEST_LOCK.lock().unwrap();
        std::env::remove_var("UMA_POLICY_CMD");

        let port = free_port();
        thread::spawn(move || serve(port));
        wait_ready(port);
        let (status, body) = http_post(
            port,
            "/v1/run/start",
            r#"{"seed":7,"scenario":"ura","trainee":"Special Week","raceModel":"stub","speed":"3"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");

        // 500 clamps to 100.
        let (status, body) = http_post(
            port,
            "/v1/run/fast",
            r#"{"policy":"default","multiplier":"500"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        assert_eq!(speed_and_turn(port).0, 100, "multiplier must clamp to 100");

        // -5 clamps to 1.
        let (status, body) = http_post(
            port,
            "/v1/run/fast",
            r#"{"policy":"default","multiplier":"-5"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        assert_eq!(speed_and_turn(port).0, 1, "multiplier must clamp to 1");

        // Non-numeric falls back to the current server default (1 here).
        let (status, body) = http_post(
            port,
            "/v1/run/fast",
            r#"{"policy":"default","multiplier":"abc"}"#,
        );
        assert_eq!(status, 200, "got {status}: {body}");
        assert_eq!(
            speed_and_turn(port).0,
            1,
            "non-numeric multiplier must fall back to the server default"
        );
    }
}
