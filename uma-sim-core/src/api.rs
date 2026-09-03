//! Minimal REST API matching Kotlin `SimApiServer` (Phase 7), extended for the web UI.

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
use crate::snapshot::RunSnapshotCodec;
use crate::state::{DialogueMode, RunMeta, SimSettings};
use serde_json::{json, Value};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const API_VERSION: &str = env!("CARGO_PKG_VERSION");

struct ApiState {
    engine: Option<SimEngine>,
    settings: SimSettings,
    /// Default policy name for `/auto` / `/fast` when the client omits it (`bot`|`default`).
    default_policy: String,
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
        engine: None,
        settings: SimSettings::default(),
        default_policy: "bot".into(),
    }));

    for mut request in server.incoming_requests() {
        let path = request.url().split('?').next().unwrap_or("/").to_string();
        let method = request.method().clone();
        if method == Method::Options {
            let _ = request.respond(cors_preflight());
            continue;
        }
        let body = read_body(&mut request);
        let mut st = state.lock().unwrap();
        let response = with_cors(route(&mut st, &method, &path, &body));
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

fn route(st: &mut ApiState, method: &Method, path: &str, body: &str) -> Response<Cursor<Vec<u8>>> {
    match (method, path) {
        (Method::Get, "/v1/health") => handle_health(),
        (Method::Get, "/v1/catalog/scenarios") => handle_catalog_scenarios(),
        (Method::Get, "/v1/catalog/trainees") => handle_catalog_trainees(),
        (Method::Get, "/v1/catalog/supports") => handle_catalog_supports(),
        (Method::Get, "/v1/catalog/factors") => handle_catalog_factors(),
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
        (Method::Post, "/v1/run/style") => handle_set_style(st, body),
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
    st.engine
        .as_ref()
        .map(|e| RunSnapshotCodec::encode(&e.export()))
        .unwrap_or_else(|| "{}".to_string())
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
    let race_model = body_string(&body, "raceModel")
        .map(|s| RaceModel::parse(&s))
        .unwrap_or_default();
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
    json_response(200, json!({"choices": choices_json(eng)}))
}

fn handle_action(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_mut() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let body = parse_body(raw);
    let action = body_string(&body, "action").unwrap_or_else(|| "rest".into());
    let result = eng.step(parse_sim_action(&action));
    step_response(eng, result.text_lines.join("\n"), result.career_ended)
}

fn handle_auto(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let policy_fallback = st.default_policy.clone();
    let Some(eng) = st.engine.as_mut() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let body = parse_body(raw);
    let policy_name = body_string(&body, "policy").unwrap_or(policy_fallback);
    let result = if policy_name == "bot" {
        eng.auto_step_scoring()
    } else {
        eng.auto_step_with_policy(default_auto_policy)
    };
    step_response(eng, result.text_lines.join("\n"), result.career_ended)
}

fn handle_fast(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let body = parse_body(raw);
    let mult = body_string(&body, "multiplier")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(st.settings.speed_multiplier)
        .clamp(1, 100);
    let policy_name = body_string(&body, "policy").unwrap_or_else(|| st.default_policy.clone());
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

fn handle_set_style(st: &mut ApiState, raw: &str) -> Response<Cursor<Vec<u8>>> {
    let Some(eng) = st.engine.as_mut() else {
        return json_response(404, json!({"error":"no active run"}));
    };
    let body = parse_body(raw);
    let Some(style) = body_string(&body, "style") else {
        return json_response(400, json!({"error":"style required (front|pace|late|end)"}));
    };
    let key = style.trim().to_ascii_lowercase();
    if !matches!(key.as_str(), "front" | "pace" | "late" | "end" | "") {
        return json_response(400, json!({"error":"style must be front|pace|late|end"}));
    }
    eng.set_preferred_running_style(if key.is_empty() { None } else { Some(key) });
    json_response_str(200, &state_json(st))
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
}
