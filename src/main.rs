mod admin_ui;
mod auth;
mod pairing;
mod sessions;
mod web_ui;

use auth::{authenticate_client, generate_key, generate_random_base64, load_config, save_config};
use pairing::PairingManager;
use sessions::{SessionManager, SharedSessions};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, Method, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

#[derive(Parser)]
#[command(name = "freecc-relay")]
struct Args {
    #[arg(long, default_value = "8081")]
    port: u16,
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long)]
    tls_cert: Option<String>,
    #[arg(long)]
    tls_key: Option<String>,
    #[arg(long)]
    generate_key: Option<String>,
}

#[derive(Clone)]
struct AppState {
    sessions: SharedSessions,
    config: Arc<RwLock<auth::Config>>,
    config_path: Arc<PathBuf>,
    pairing: Arc<RwLock<PairingManager>>,
    admin_password: Arc<String>,
    host: String,
    port: u16,
    is_tls: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let config_dir = exe_dir.join("config");
    let config_path = config_dir.join("server.json");
    let data_dir = exe_dir.join("data");
    let sessions_path = data_dir.join("sessions.json");

    // Handle --generate-key
    if let Some(name) = &args.generate_key {
        std::fs::create_dir_all(&config_dir).ok();
        let mut config = load_config(&config_path);
        let key = generate_key("ck");
        config.clients.push(auth::Client {
            name: name.clone(),
            key: key.clone(),
        });
        save_config(&config, &config_path);
        println!("Generated client key for \"{}\": {}", name, key);
        return;
    }

    // Ensure dirs
    std::fs::create_dir_all(&config_dir).ok();
    std::fs::create_dir_all(&data_dir).ok();

    let config = load_config(&config_path);
    let is_tls = args.tls_cert.is_some() && args.tls_key.is_some();
    let admin_password = generate_random_base64(12);

    let session_mgr = SessionManager::new(Some(sessions_path));
    let shared_sessions: SharedSessions = Arc::new(RwLock::new(session_mgr));

    // Periodic cleanup every 10 minutes
    {
        let sessions = shared_sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                sessions.write().await.cleanup(24 * 60 * 60 * 1000);
            }
        });
    }

    // Periodic save every 30 seconds
    {
        let sessions = shared_sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                sessions.read().await.save_to_disk();
            }
        });
    }

    let pairing_mgr = PairingManager::new();
    let shared_pairing = Arc::new(RwLock::new(pairing_mgr));

    // Periodic pairing cleanup every 60 seconds
    {
        let pairing = shared_pairing.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                pairing.write().await.cleanup_expired();
            }
        });
    }

    let state = AppState {
        sessions: shared_sessions,
        config: Arc::new(RwLock::new(config.clone())),
        config_path: Arc::new(config_path.clone()),
        pairing: shared_pairing,
        admin_password: Arc::new(admin_password.clone()),
        host: args.host.clone(),
        port: args.port,
        is_tls,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            "X-Client-Key".parse().unwrap(),
            "X-Admin-Token".parse().unwrap(),
        ]);

    let app = Router::new()
        .route("/", get(handle_index))
        .route("/api/health", get(handle_health))
        .route("/api/pair", post(handle_pair_request))
        .route("/api/pair/{id}", get(handle_pair_check))
        .route(
            "/api/sessions",
            get(handle_list_sessions).post(handle_create_session),
        )
        .route("/api/sessions/{id}", delete(handle_delete_session))
        .route("/admin", get(handle_admin))
        .route("/api/admin/sessions", get(handle_admin_list))
        .route("/api/admin/sessions/kill", post(handle_admin_kill))
        .route("/api/admin/sessions/kill-all", post(handle_admin_kill_all))
        .route("/api/admin/sessions/cleanup", post(handle_admin_cleanup))
        .route("/api/admin/pairings", get(handle_admin_pairings))
        .route(
            "/api/admin/pairings/{id}/approve",
            post(handle_admin_approve_pairing),
        )
        .route(
            "/api/admin/pairings/{id}/reject",
            post(handle_admin_reject_pairing),
        )
        .route("/s/{id}", get(handle_web_ui))
        .route("/ws/cli/{id}", get(handle_ws_cli))
        .route("/ws/web/{id}", get(handle_ws_web))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse().unwrap();
    let proto = if is_tls { "https" } else { "http" };

    println!();
    println!("  ╔══════════════════════════════════════════════╗");
    println!("  ║     Free CC Relay Server                       ║");
    println!("  ╚══════════════════════════════════════════════╝");
    println!();
    println!(
        "  Server running on {}://{}:{}",
        proto, args.host, args.port
    );
    println!(
        "  Admin dashboard: {}://{}:{}/admin",
        proto, args.host, args.port
    );
    println!("  Admin password:  {}", admin_password);
    if is_tls {
        println!("  TLS: enabled");
    }
    println!();
    println!("  Client keys:");
    for client in &config.clients {
        println!("    {}: {}", client.name, client.key);
    }
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    println!("\nShutting down...");
}

// --- Handlers ---

async fn handle_index(State(state): State<AppState>) -> Html<String> {
    let count = state.sessions.read().await.count();
    Html(format!(
        r#"<!DOCTYPE html><html><head><title>Free CC Relay</title>
<style>body{{background:#1a1b26;color:#c0caf5;font-family:monospace;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;}}
.c{{text-align:center;}}h1{{color:#7aa2f7;}}p{{color:#565f89;}}a{{color:#7aa2f7;}}</style></head>
<body><div class="c"><h1>Free CC Relay Server</h1><p>Server is running.</p>
<p style="margin-top:20px;font-size:12px;">Active sessions: <span id="n">{}</span></p>
<p style="margin-top:10px;"><a href="/admin">Admin Dashboard</a></p>
<script>fetch('/api/health').then(r=>r.json()).then(d=>{{document.getElementById('n').textContent=d.sessions}})</script>
</div></body></html>"#,
        count
    ))
}

async fn handle_health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let count = state.sessions.read().await.count();
    let pending_pairings = state
        .pairing
        .read()
        .await
        .list_all()
        .iter()
        .filter(|p| p.status == "pending")
        .count();
    Json(serde_json::json!({"status": "ok", "sessions": count, "pendingPairings": pending_pairings}))
}

async fn handle_create_session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let client_key = headers
        .get("x-client-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let config = state.config.read().await;
    let client_name = match authenticate_client(&config, client_key) {
        Some(name) => name,
        None => return json_response(401, serde_json::json!({"error": "Invalid client key"})),
    };

    let mut sessions = state.sessions.write().await;
    let existing = sessions.count_by_client(client_key);
    if existing >= config.auth.max_sessions_per_key {
        return json_response(
            429,
            serde_json::json!({"error": "Max sessions reached", "max": config.auth.max_sessions_per_key}),
        );
    }

    let info = sessions.create(&client_name, client_key);

    let forwarded_proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok());
    let proto = forwarded_proto.unwrap_or(if state.is_tls { "https" } else { "http" });
    let ws_proto = if proto == "https" { "wss" } else { "ws" };
    let default_host = format!("{}:{}", state.host, state.port);
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&default_host);
    let base = format!("{}://{}", proto, host);
    let ws_base = format!("{}://{}", ws_proto, host);

    json_response(
        201,
        serde_json::json!({
            "sessionId": info.id,
            "token": info.token,
            "url": format!("{}/s/{}?token={}", base, info.id, info.token),
            "wsCliUrl": format!("{}/ws/cli/{}?key={}", ws_base, info.id, client_key),
            "wsWebUrl": format!("{}/ws/web/{}?token={}", ws_base, info.id, info.token),
        }),
    )
}

async fn handle_list_sessions(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let client_key = headers
        .get("x-client-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let config = state.config.read().await;
    if authenticate_client(&config, client_key).is_none() {
        return json_response(401, serde_json::json!({"error": "Invalid client key"}));
    }

    let sessions = state.sessions.read().await;
    let list = sessions.list_by_client(client_key);
    json_response(200, serde_json::json!({"sessions": list}))
}

async fn handle_delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let client_key = headers
        .get("x-client-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let config = state.config.read().await;
    if authenticate_client(&config, client_key).is_none() {
        return json_response(401, serde_json::json!({"error": "Invalid client key"}));
    }

    let mut sessions = state.sessions.write().await;
    let owns = sessions
        .get(&id)
        .map(|s| s.client_key == client_key)
        .unwrap_or(false);

    if !owns {
        return json_response(404, serde_json::json!({"error": "Session not found"}));
    }

    sessions.close(&id);
    json_response(200, serde_json::json!({"ok": true}))
}

// --- Admin handlers ---

fn check_admin_token(headers: &HeaderMap, admin_password: &str) -> bool {
    if let Some(token) = headers.get("x-admin-token").and_then(|v| v.to_str().ok()) {
        if token == admin_password {
            return true;
        }
    }
    if let Some(cookie) = headers.get("cookie").and_then(|v| v.to_str().ok()) {
        for part in cookie.split(';') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("admin_token=") {
                if val == admin_password {
                    return true;
                }
            }
        }
    }
    false
}

async fn handle_admin(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let token_param = params.get("token").map(|s| s.as_str());
    let cookie_header = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let cookie_token = cookie_header
        .split(';')
        .find_map(|p| p.trim().strip_prefix("admin_token="));

    let token = token_param.or(cookie_token);

    if token != Some(state.admin_password.as_str()) {
        let html = admin_ui::render_admin_login(token_param.is_some());
        return Html(html).into_response();
    }

    let mut response = Html(admin_ui::render_admin_ui(&state.admin_password)).into_response();
    if token_param.is_some() && cookie_token.is_none() {
        let secure = if state.is_tls { "; Secure" } else { "" };
        let cookie = format!(
            "admin_token={}; Path=/admin; HttpOnly; SameSite=Strict; Max-Age=86400{}",
            state.admin_password, secure
        );
        response
            .headers_mut()
            .insert("Set-Cookie", cookie.parse().unwrap());
    }
    response
}

async fn handle_admin_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_admin_token(&headers, &state.admin_password) {
        return json_response(401, serde_json::json!({"error": "Unauthorized"}));
    }
    let sessions = state.sessions.read().await;
    let list = sessions.list_all();
    let config = state.config.read().await;
    let clients: Vec<serde_json::Value> = config
        .clients
        .iter()
        .map(|c| serde_json::json!({"name": c.name}))
        .collect();
    json_response(
        200,
        serde_json::json!({"sessions": list, "clients": clients}),
    )
}

#[derive(Deserialize)]
struct KillBody {
    #[serde(rename = "sessionIds", default)]
    session_ids: Vec<String>,
}

async fn handle_admin_kill(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<KillBody>,
) -> Response {
    if !check_admin_token(&headers, &state.admin_password) {
        return json_response(401, serde_json::json!({"error": "Unauthorized"}));
    }
    let mut sessions = state.sessions.write().await;
    let mut closed = 0;
    for id in &body.session_ids {
        if sessions.get(id).is_some() {
            sessions.close(id);
            closed += 1;
        }
    }
    json_response(200, serde_json::json!({"closed": closed}))
}

async fn handle_admin_kill_all(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_admin_token(&headers, &state.admin_password) {
        return json_response(401, serde_json::json!({"error": "Unauthorized"}));
    }
    let mut sessions = state.sessions.write().await;
    let ids: Vec<String> = sessions.sessions.keys().cloned().collect();
    let count = ids.len();
    for id in ids {
        sessions.close(&id);
    }
    json_response(200, serde_json::json!({"closed": count}))
}

#[derive(Deserialize)]
struct CleanupBody {
    #[serde(rename = "maxAgeHours", default = "default_cleanup_hours")]
    max_age_hours: u64,
}

fn default_cleanup_hours() -> u64 {
    24
}

async fn handle_admin_cleanup(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<CleanupBody>,
) -> Response {
    if !check_admin_token(&headers, &state.admin_password) {
        return json_response(401, serde_json::json!({"error": "Unauthorized"}));
    }
    let mut sessions = state.sessions.write().await;
    let before = sessions.count();
    sessions.cleanup(body.max_age_hours * 60 * 60 * 1000);
    let after = sessions.count();
    json_response(200, serde_json::json!({"removed": before - after}))
}

// --- Pairing handlers ---

#[derive(Deserialize)]
struct PairRequestBody {
    #[serde(default = "default_hostname")]
    hostname: String,
}

fn default_hostname() -> String {
    "unknown".to_string()
}

async fn handle_pair_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<PairRequestBody>,
) -> Response {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut pairing = state.pairing.write().await;
    let request = pairing.create(&body.hostname, &ip);
    println!(
        "[pairing] NEW request from {} ({}) id={}",
        body.hostname, ip, request.id
    );
    json_response(
        201,
        serde_json::json!({"pairingId": request.id, "status": "pending"}),
    )
}

async fn handle_pair_check(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let pairing = state.pairing.read().await;
    match pairing.get(&id) {
        Some(request) => {
            let mut result =
                serde_json::json!({"pairingId": request.id, "status": request.status});
            if request.status == "approved" {
                result["clientKey"] = serde_json::json!(request.client_key);
                result["clientName"] = serde_json::json!(request.client_name);
            }
            json_response(200, result)
        }
        None => json_response(
            404,
            serde_json::json!({"error": "Pairing request not found or expired"}),
        ),
    }
}

async fn handle_admin_pairings(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !check_admin_token(&headers, &state.admin_password) {
        return json_response(401, serde_json::json!({"error": "Unauthorized"}));
    }
    let pairing = state.pairing.read().await;
    json_response(200, serde_json::json!({"pairings": pairing.list_all()}))
}

#[derive(Deserialize)]
struct ApproveBody {
    #[serde(default)]
    name: Option<String>,
}

async fn handle_admin_approve_pairing(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<ApproveBody>,
) -> Response {
    if !check_admin_token(&headers, &state.admin_password) {
        return json_response(401, serde_json::json!({"error": "Unauthorized"}));
    }
    let mut pairing = state.pairing.write().await;
    let mut config = state.config.write().await;
    match pairing.approve(&id, body.name.as_deref(), &mut config, &state.config_path) {
        Some(result) => {
            println!(
                "[pairing] APPROVED {} → {} ({}...)",
                id,
                result.client_name.as_deref().unwrap_or(""),
                &result.client_key.as_deref().unwrap_or("")[..8.min(
                    result.client_key.as_deref().unwrap_or("").len()
                )]
            );
            json_response(
                200,
                serde_json::json!({"ok": true, "clientName": result.client_name}),
            )
        }
        None => json_response(
            404,
            serde_json::json!({"error": "Pairing not found or already processed"}),
        ),
    }
}

async fn handle_admin_reject_pairing(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !check_admin_token(&headers, &state.admin_password) {
        return json_response(401, serde_json::json!({"error": "Unauthorized"}));
    }
    let mut pairing = state.pairing.write().await;
    pairing.reject(&id);
    println!("[pairing] REJECTED {}", id);
    json_response(200, serde_json::json!({"ok": true}))
}

// --- Web UI ---

async fn handle_web_ui(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let token = params.get("token").map(|s| s.as_str()).unwrap_or("");
    let sessions = state.sessions.read().await;
    let session = match sessions.get(&id) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h1>Session not found</h1>".to_string()),
            )
                .into_response()
        }
    };

    let config = state.config.read().await;
    if config.auth.web_access_policy == "token" && session.token != token {
        return (
            StatusCode::FORBIDDEN,
            Html("<h1>Access denied</h1>".to_string()),
        )
            .into_response();
    }

    let forwarded_proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok());
    let default_host = format!("{}:{}", state.host, state.port);
    let host_header = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&default_host);

    let ws_proto = if forwarded_proto == Some("https") || state.is_tls {
        "wss"
    } else {
        "ws"
    };
    let ws_url = format!("{ws_proto}://{host_header}/ws/web/{id}?token={token}");

    Html(web_ui::render_web_ui(&id, token, &ws_url)).into_response()
}

// --- WebSocket handlers ---

#[derive(Deserialize)]
struct WsCliQuery {
    key: Option<String>,
}

async fn handle_ws_cli(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<WsCliQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let client_key = query.key.unwrap_or_default();

    let config = state.config.read().await;
    let client_name = match authenticate_client(&config, &client_key) {
        Some(name) => name,
        None => return (StatusCode::UNAUTHORIZED, "Invalid client key").into_response(),
    };
    drop(config);

    {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(s) if s.client_key == client_key => {}
            _ => return (StatusCode::NOT_FOUND, "Session not found").into_response(),
        }
    }

    ws.on_upgrade(move |socket| async move {
        handle_cli_socket(socket, state, session_id, client_name).await;
    })
}

async fn handle_cli_socket(
    socket: WebSocket,
    state: AppState,
    session_id: String,
    client_name: String,
) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Register CLI
    {
        let mut sessions = state.sessions.write().await;
        sessions.attach_cli(&session_id, tx.clone());
        if let Some(session) = sessions.get(&session_id) {
            let info = serde_json::json!({
                "type": "session_info",
                "sessionId": session_id,
                "status": "active",
                "webClients": session.web_sockets.len(),
                "timestamp": now_ms(),
            });
            let _ = tx.send(Message::Text(info.to_string().into()));
        }
    }
    println!(
        "[ws:cli] CONNECTED client=\"{}\" session={}",
        client_name, session_id
    );

    // Forward channel to sink
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Read from CLI
    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Text(text) => {
                let text: &str = &text;
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    state
                        .sessions
                        .write()
                        .await
                        .relay_from_cli(&session_id, parsed);
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    println!("[ws:cli] DISCONNECTED session={}", session_id);
    state.sessions.write().await.detach_cli(&session_id);
    send_task.abort();
}

#[derive(Deserialize)]
struct WsWebQuery {
    token: Option<String>,
}

async fn handle_ws_web(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(query): Query<WsWebQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let token = query.token.unwrap_or_default();

    {
        let config = state.config.read().await;
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(s) => {
                if config.auth.web_access_policy == "token" && s.token != token {
                    return (StatusCode::FORBIDDEN, "Invalid token").into_response();
                }
            }
            None => return (StatusCode::NOT_FOUND, "Session not found").into_response(),
        }
    }

    ws.on_upgrade(move |socket| async move {
        handle_web_socket(socket, state, session_id).await;
    })
}

async fn handle_web_socket(socket: WebSocket, state: AppState, session_id: String) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    {
        let mut sessions = state.sessions.write().await;
        sessions.attach_web(&session_id, tx.clone());
    }
    println!("[ws:web] CONNECTED session={}", session_id);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Text(text) => {
                let text: &str = &text;
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                    state
                        .sessions
                        .write()
                        .await
                        .relay_from_web(&session_id, parsed);
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    println!("[ws:web] DISCONNECTED session={}", session_id);
    state.sessions.write().await.detach_web(&session_id, &tx);
    send_task.abort();
}

// --- Helpers ---

fn json_response(status: u16, body: serde_json::Value) -> Response {
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(body)).into_response()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
