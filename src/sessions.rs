use crate::auth::generate_random_base64;
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// A sender handle for a WebSocket connection
pub type WsSender = mpsc::UnboundedSender<Message>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub token: String,
    pub client_name: String,
    pub client_key: String,
    pub created_at: u64,
    pub last_activity: u64,
}

pub struct Session {
    pub id: String,
    pub token: String,
    pub client_name: String,
    pub client_key: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub cli_socket: Option<WsSender>,
    pub web_sockets: Vec<WsSender>,
    pub messages: Vec<serde_json::Value>,
    pub status: String,
}

#[derive(Serialize)]
pub struct SessionListItem {
    pub id: String,
    #[serde(rename = "clientName")]
    pub client_name: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "lastActivity")]
    pub last_activity: u64,
    pub status: String,
    #[serde(rename = "webClients")]
    pub web_clients: usize,
    #[serde(rename = "hasCli")]
    pub has_cli: bool,
}

#[derive(Serialize)]
pub struct AdminSessionListItem {
    pub id: String,
    #[serde(rename = "clientName")]
    pub client_name: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "lastActivity")]
    pub last_activity: u64,
    pub status: String,
    #[serde(rename = "webClients")]
    pub web_clients: usize,
    #[serde(rename = "hasCli")]
    pub has_cli: bool,
    #[serde(rename = "messageCount")]
    pub message_count: usize,
}

pub struct SessionManager {
    pub sessions: HashMap<String, Session>,
    persist_path: Option<PathBuf>,
}

impl SessionManager {
    pub fn new(persist_path: Option<PathBuf>) -> Self {
        let mut mgr = SessionManager {
            sessions: HashMap::new(),
            persist_path,
        };
        mgr.load_from_disk();
        mgr
    }

    pub fn create(&mut self, client_name: &str, client_key: &str) -> SessionInfo {
        let id = format!("ses_{}", generate_random_base64(16));
        let token = format!("tok_{}", generate_random_base64(24));
        let now = now_ms();

        let session = Session {
            id: id.clone(),
            token: token.clone(),
            client_name: client_name.to_string(),
            client_key: client_key.to_string(),
            created_at: now,
            last_activity: now,
            cli_socket: None,
            web_sockets: Vec::new(),
            messages: Vec::new(),
            status: "waiting".to_string(),
        };

        let info = SessionInfo {
            id: id.clone(),
            token: token.clone(),
            client_name: client_name.to_string(),
            client_key: client_key.to_string(),
            created_at: now,
            last_activity: now,
        };

        self.sessions.insert(id, session);
        self.save_to_disk();
        info
    }

    pub fn get(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn list_by_client(&self, client_key: &str) -> Vec<SessionListItem> {
        self.sessions
            .values()
            .filter(|s| s.client_key == client_key)
            .map(|s| SessionListItem {
                id: s.id.clone(),
                client_name: s.client_name.clone(),
                created_at: s.created_at,
                last_activity: s.last_activity,
                status: s.status.clone(),
                web_clients: s.web_sockets.len(),
                has_cli: s.cli_socket.is_some(),
            })
            .collect()
    }

    pub fn list_all(&self) -> Vec<AdminSessionListItem> {
        self.sessions
            .values()
            .map(|s| AdminSessionListItem {
                id: s.id.clone(),
                client_name: s.client_name.clone(),
                created_at: s.created_at,
                last_activity: s.last_activity,
                status: s.status.clone(),
                web_clients: s.web_sockets.len(),
                has_cli: s.cli_socket.is_some(),
                message_count: s.messages.len(),
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    pub fn count_by_client(&self, client_key: &str) -> usize {
        self.sessions
            .values()
            .filter(|s| s.client_key == client_key)
            .count()
    }

    pub fn attach_cli(&mut self, session_id: &str, sender: WsSender) -> bool {
        let session = match self.sessions.get_mut(session_id) {
            Some(s) => s,
            None => return false,
        };

        // Close previous CLI if any
        if let Some(old) = session.cli_socket.take() {
            let _ = old.send(Message::Close(None));
        }

        session.cli_socket = Some(sender);
        session.status = "active".to_string();
        session.last_activity = now_ms();

        let msg = serde_json::json!({
            "type": "system",
            "content": "CLI client connected",
            "timestamp": now_ms(),
        });
        broadcast_to_web(session, &msg);
        true
    }

    pub fn detach_cli(&mut self, session_id: &str) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.cli_socket = None;
            session.status = "waiting".to_string();

            let msg = serde_json::json!({
                "type": "system",
                "content": "CLI client disconnected",
                "timestamp": now_ms(),
            });
            broadcast_to_web(session, &msg);
        }
    }

    pub fn attach_web(&mut self, session_id: &str, sender: WsSender) -> bool {
        let session = match self.sessions.get_mut(session_id) {
            Some(s) => s,
            None => return false,
        };

        session.web_sockets.push(sender.clone());
        session.last_activity = now_ms();

        // Send history
        if !session.messages.is_empty() {
            let history = serde_json::json!({
                "type": "history",
                "messages": session.messages,
                "timestamp": now_ms(),
            });
            let _ = sender.send(Message::Text(history.to_string().into()));
        }

        // Send session info
        let info = serde_json::json!({
            "type": "session_info",
            "sessionId": session.id,
            "status": session.status,
            "cliConnected": session.cli_socket.is_some(),
            "webClients": session.web_sockets.len(),
            "timestamp": now_ms(),
        });
        let _ = sender.send(Message::Text(info.to_string().into()));

        // Notify CLI
        if let Some(cli) = &session.cli_socket {
            let msg = serde_json::json!({
                "type": "system",
                "content": format!("Web client connected ({} total)", session.web_sockets.len()),
                "timestamp": now_ms(),
            });
            let _ = cli.send(Message::Text(msg.to_string().into()));
        }

        true
    }

    pub fn detach_web(&mut self, session_id: &str, sender: &WsSender) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.web_sockets.retain(|s| !s.same_channel(sender));

            if let Some(cli) = &session.cli_socket {
                let msg = serde_json::json!({
                    "type": "system",
                    "content": format!("Web client disconnected ({} remaining)", session.web_sockets.len()),
                    "timestamp": now_ms(),
                });
                let _ = cli.send(Message::Text(msg.to_string().into()));
            }
        }
    }

    pub fn relay_from_cli(&mut self, session_id: &str, message: serde_json::Value) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.last_activity = now_ms();
            session.messages.push(message.clone());
            if session.messages.len() > 500 {
                let len = session.messages.len();
                session.messages.drain(..len - 500);
            }
            broadcast_to_web(session, &message);
        }
    }

    pub fn relay_from_web(&mut self, session_id: &str, message: serde_json::Value) -> bool {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.last_activity = now_ms();
            session.messages.push(message.clone());
            if session.messages.len() > 500 {
                let len = session.messages.len();
                session.messages.drain(..len - 500);
            }
            if let Some(cli) = &session.cli_socket {
                let _ = cli.send(Message::Text(message.to_string().into()));
                return true;
            }
        }
        false
    }

    pub fn close(&mut self, session_id: &str) {
        if let Some(mut session) = self.sessions.remove(session_id) {
            session.status = "closed".to_string();
            let close_msg = serde_json::json!({
                "type": "system",
                "content": "Session closed",
                "timestamp": now_ms(),
            });
            let data = Message::Text(close_msg.to_string().into());

            if let Some(cli) = session.cli_socket.take() {
                let _ = cli.send(data.clone());
                let _ = cli.send(Message::Close(None));
            }
            for ws in &session.web_sockets {
                let _ = ws.send(data.clone());
                let _ = ws.send(Message::Close(None));
            }
        }
        self.save_to_disk();
    }

    pub fn cleanup(&mut self, max_age_ms: u64) {
        let now = now_ms();
        let to_close: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| now - s.last_activity > max_age_ms)
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_close {
            self.close(&id);
        }
    }

    pub fn save_to_disk(&self) {
        let path = match &self.persist_path {
            Some(p) => p,
            None => return,
        };
        let data: Vec<SessionInfo> = self
            .sessions
            .values()
            .map(|s| SessionInfo {
                id: s.id.clone(),
                token: s.token.clone(),
                client_name: s.client_name.clone(),
                client_key: s.client_key.clone(),
                created_at: s.created_at,
                last_activity: s.last_activity,
            })
            .collect();
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = fs::write(path, json);
        }
    }

    fn load_from_disk(&mut self) {
        let path = match &self.persist_path {
            Some(p) => p.clone(),
            None => return,
        };
        if !path.exists() {
            return;
        }
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let items: Vec<SessionInfo> = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(_) => return,
        };
        let now = now_ms();
        let day_ms = 24 * 60 * 60 * 1000;
        let mut count = 0;
        for info in items {
            if now - info.last_activity > day_ms {
                continue;
            }
            let session = Session {
                id: info.id.clone(),
                token: info.token,
                client_name: info.client_name,
                client_key: info.client_key,
                created_at: info.created_at,
                last_activity: info.last_activity,
                cli_socket: None,
                web_sockets: Vec::new(),
                messages: Vec::new(),
                status: "waiting".to_string(),
            };
            self.sessions.insert(info.id, session);
            count += 1;
        }
        if count > 0 {
            println!("  Restored {} sessions from disk", count);
        }
    }
}

fn broadcast_to_web(session: &Session, message: &serde_json::Value) {
    let data = Message::Text(message.to_string().into());
    for ws in &session.web_sockets {
        let _ = ws.send(data.clone());
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub type SharedSessions = Arc<RwLock<SessionManager>>;
