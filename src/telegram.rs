use crate::auth::save_config;
use crate::AppState;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://api.telegram.org/bot";

#[derive(Debug, Deserialize)]
struct TgResponse<T> {
    ok: bool,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
    callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Deserialize)]
struct Message {
    message_id: i64,
    chat: Chat,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    id: String,
    message: Option<Message>,
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
}

#[derive(Debug, Serialize)]
struct SendMessage {
    chat_id: i64,
    text: String,
    parse_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_markup: Option<InlineKeyboardMarkup>,
}

#[derive(Debug, Serialize)]
struct EditMessage {
    chat_id: i64,
    message_id: i64,
    text: String,
    parse_mode: String,
}

#[derive(Debug, Serialize)]
struct AnswerCallback {
    callback_query_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct InlineKeyboardMarkup {
    inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Serialize, Clone)]
struct InlineKeyboardButton {
    text: String,
    callback_data: String,
}

pub struct Bot {
    token: String,
    http: Client,
    state: AppState,
    allowed_chat: Option<i64>,
}

impl Bot {
    fn new(token: String, state: AppState, admin_chat_id: Option<i64>) -> Self {
        Bot {
            token,
            http: Client::new(),
            state,
            allowed_chat: admin_chat_id,
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}{}/{}", API_BASE, self.token, method)
    }

    async fn send(&self, chat_id: i64, text: &str) -> Option<()> {
        self.send_with_markup(chat_id, text, None).await
    }

    async fn send_with_markup(
        &self,
        chat_id: i64,
        text: &str,
        markup: Option<InlineKeyboardMarkup>,
    ) -> Option<()> {
        let msg = SendMessage {
            chat_id,
            text: text.to_string(),
            parse_mode: "HTML".to_string(),
            reply_markup: markup,
        };
        self.http
            .post(self.api_url("sendMessage"))
            .json(&msg)
            .send()
            .await
            .ok()?;
        Some(())
    }

    async fn edit_message(&self, chat_id: i64, message_id: i64, text: &str) -> Option<()> {
        let msg = EditMessage {
            chat_id,
            message_id,
            text: text.to_string(),
            parse_mode: "HTML".to_string(),
        };
        self.http
            .post(self.api_url("editMessageText"))
            .json(&msg)
            .send()
            .await
            .ok()?;
        Some(())
    }

    async fn answer_callback(&self, id: &str, text: Option<&str>) -> Option<()> {
        let msg = AnswerCallback {
            callback_query_id: id.to_string(),
            text: text.map(|s| s.to_string()),
        };
        self.http
            .post(self.api_url("answerCallbackQuery"))
            .json(&msg)
            .send()
            .await
            .ok()?;
        Some(())
    }

    async fn handle_message(&mut self, msg: Message) {
        let chat_id = msg.chat.id;
        let text = msg.text.as_deref().unwrap_or("");

        if self.allowed_chat.is_some() && self.allowed_chat != Some(chat_id) {
            self.send(chat_id, "Unauthorized.").await;
            return;
        }

        let parts: Vec<&str> = text.splitn(3, ' ').collect();
        let cmd = parts.first().copied().unwrap_or("");

        match cmd {
            "/start" | "/help" => self.cmd_help(chat_id).await,
            "/clients" => self.cmd_clients(chat_id).await,
            "/sessions" => {
                if parts.len() < 2 {
                    self.send(chat_id, "Usage: /sessions &lt;client_name&gt;").await;
                } else {
                    self.cmd_sessions(chat_id, parts[1]).await;
                }
            }
            "/pairings" => self.cmd_pairings(chat_id).await,
            "/approve" => {
                if parts.len() < 2 {
                    self.send(chat_id, "Usage: /approve &lt;pairing_id&gt; [name]").await;
                } else {
                    let name = parts.get(2).copied();
                    self.cmd_approve(chat_id, parts[1], name).await;
                }
            }
            "/rmclient" => {
                if parts.len() < 2 {
                    self.send(chat_id, "Usage: /rmclient &lt;client_name&gt;").await;
                } else {
                    self.cmd_rmclient(chat_id, parts[1]).await;
                }
            }
            "/rmsession" => {
                if parts.len() < 2 {
                    self.send(chat_id, "Usage: /rmsession &lt;session_id&gt;").await;
                } else {
                    self.cmd_rmsession(chat_id, parts[1]).await;
                }
            }
            _ => {
                if text.starts_with('/') {
                    self.send(chat_id, "Unknown command. /help for usage.").await;
                }
            }
        }
    }

    async fn handle_callback(&self, cb: CallbackQuery) {
        let chat_id = cb.message.as_ref().map(|m| m.chat.id).unwrap_or(0);
        let msg_id = cb.message.as_ref().map(|m| m.message_id).unwrap_or(0);
        let data = cb.data.as_deref().unwrap_or("");

        if let Some(chat) = self.allowed_chat {
            if chat != chat_id {
                self.answer_callback(&cb.id, Some("Unauthorized")).await;
                return;
            }
        }

        let parts: Vec<&str> = data.splitn(3, ':').collect();
        match parts.first().copied().unwrap_or("") {
            "approve" => {
                let pair_id = parts.get(1).copied().unwrap_or("");
                let name = parts.get(2).copied();
                let mut pairing = self.state.pairing.write().await;
                let mut config = self.state.config.write().await;
                match pairing.approve(pair_id, name, &mut config, &self.state.config_path) {
                    Some(result) => {
                        let cname = result.client_name.as_deref().unwrap_or("?");
                        self.answer_callback(&cb.id, Some("Approved")).await;
                        self.edit_message(chat_id, msg_id, &format!("Approved <b>{}</b>", esc(cname)))
                            .await;
                    }
                    None => {
                        self.answer_callback(&cb.id, Some("Not found or already processed"))
                            .await;
                    }
                }
            }
            "reject" => {
                let pair_id = parts.get(1).copied().unwrap_or("");
                let mut pairing = self.state.pairing.write().await;
                pairing.reject(pair_id);
                self.answer_callback(&cb.id, Some("Rejected")).await;
                self.edit_message(chat_id, msg_id, "Rejected").await;
            }
            "rmsession" => {
                let session_id = parts.get(1).copied().unwrap_or("");
                let mut sessions = self.state.sessions.write().await;
                if sessions.get(session_id).is_some() {
                    sessions.close(session_id);
                    self.answer_callback(&cb.id, Some("Session removed")).await;
                    self.edit_message(chat_id, msg_id, &format!("Removed session <code>{}</code>", esc(session_id))).await;
                } else {
                    self.answer_callback(&cb.id, Some("Session not found")).await;
                }
            }
            "rmclient" => {
                let client_name = parts.get(1).copied().unwrap_or("");
                let mut config = self.state.config.write().await;
                let before = config.clients.len();
                config.clients.retain(|c| c.name != client_name);
                if config.clients.len() < before {
                    save_config(&config, &self.state.config_path);
                    // Also close all sessions for this client's keys
                    drop(config);
                    self.answer_callback(&cb.id, Some("Client removed")).await;
                    self.edit_message(chat_id, msg_id, &format!("Removed client <b>{}</b>", esc(client_name))).await;
                } else {
                    self.answer_callback(&cb.id, Some("Client not found")).await;
                }
            }
            _ => {
                self.answer_callback(&cb.id, None).await;
            }
        }
    }

    async fn cmd_help(&self, chat_id: i64) {
        self.send(
            chat_id,
            "<b>Free CC Relay Bot</b>\n\n\
             /clients - List clients with session counts\n\
             /sessions &lt;name&gt; - List sessions of a client\n\
             /pairings - List pending pairing requests\n\
             /approve &lt;id&gt; [name] - Approve a pairing\n\
             /rmclient &lt;name&gt; - Remove a client\n\
             /rmsession &lt;id&gt; - Remove a session",
        )
        .await;
    }

    async fn cmd_clients(&self, chat_id: i64) {
        let config = self.state.config.read().await;
        let sessions = self.state.sessions.read().await;

        if config.clients.is_empty() {
            self.send(chat_id, "No clients configured.").await;
            return;
        }

        let mut text = String::from("<b>Clients</b>\n");
        for client in &config.clients {
            let list = sessions.list_by_client(&client.key);
            let active = list.iter().filter(|s| s.status == "active").count();

            // Get IPs from pairing requests
            let pairing = self.state.pairing.read().await;
            let ip = pairing
                .requests
                .values()
                .find(|r| r.client_name.as_deref() == Some(&client.name))
                .map(|r| r.ip.as_str())
                .unwrap_or("-");

            text.push_str(&format!(
                "\n<b>{}</b> — {} sessions ({} active) — ip: <code>{}</code>",
                esc(&client.name),
                list.len(),
                active,
                esc(ip),
            ));
        }
        self.send(chat_id, &text).await;
    }

    async fn cmd_sessions(&self, chat_id: i64, client_name: &str) {
        let config = self.state.config.read().await;
        let sessions = self.state.sessions.read().await;

        let keys: Vec<&str> = config
            .clients
            .iter()
            .filter(|c| c.name == client_name)
            .map(|c| c.key.as_str())
            .collect();

        if keys.is_empty() {
            self.send(chat_id, &format!("Client <b>{}</b> not found.", esc(client_name)))
                .await;
            return;
        }

        let mut all_sessions = Vec::new();
        for key in &keys {
            all_sessions.extend(sessions.list_by_client(key));
        }

        if all_sessions.is_empty() {
            self.send(
                chat_id,
                &format!("No sessions for <b>{}</b>.", esc(client_name)),
            )
            .await;
            return;
        }

        all_sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

        let mut text = format!("<b>Sessions for {}</b>\n", esc(client_name));
        for s in &all_sessions {
            let cli = if s.has_cli { "cli" } else { "no-cli" };
            let age = format_age(s.last_activity);
            text.push_str(&format!(
                "\n<code>{}</code>\n  {} | {} | web:{} | msgs:{} | {}\n",
                esc(&s.id),
                s.status,
                cli,
                s.web_clients,
                0, // messageCount not in SessionListItem
                age,
            ));
        }

        // Add remove buttons
        let buttons: Vec<Vec<InlineKeyboardButton>> = all_sessions
            .iter()
            .map(|s| {
                vec![InlineKeyboardButton {
                    text: format!("Remove {}", &s.id[..s.id.len().min(16)]),
                    callback_data: format!("rmsession:{}", s.id),
                }]
            })
            .collect();

        self.send_with_markup(
            chat_id,
            &text,
            Some(InlineKeyboardMarkup {
                inline_keyboard: buttons,
            }),
        )
        .await;
    }

    async fn cmd_pairings(&self, chat_id: i64) {
        let pairing = self.state.pairing.read().await;
        let pending: Vec<_> = pairing
            .list_all()
            .into_iter()
            .filter(|p| p.status == "pending")
            .collect();

        if pending.is_empty() {
            self.send(chat_id, "No pending pairing requests.").await;
            return;
        }

        for p in &pending {
            let text = format!(
                "<b>Pairing Request</b>\nID: <code>{}</code>\nHost: <b>{}</b>\nIP: <code>{}</code>\nTime: {}",
                esc(&p.id),
                esc(&p.hostname),
                esc(&p.ip),
                format_age(p.created_at),
            );

            let buttons = vec![vec![
                InlineKeyboardButton {
                    text: "Approve".to_string(),
                    callback_data: format!("approve:{}:{}", p.id, p.hostname),
                },
                InlineKeyboardButton {
                    text: "Reject".to_string(),
                    callback_data: format!("reject:{}", p.id),
                },
            ]];

            self.send_with_markup(
                chat_id,
                &text,
                Some(InlineKeyboardMarkup {
                    inline_keyboard: buttons,
                }),
            )
            .await;
        }
    }

    async fn cmd_approve(&self, chat_id: i64, pair_id: &str, name: Option<&str>) {
        let mut pairing = self.state.pairing.write().await;
        let mut config = self.state.config.write().await;
        match pairing.approve(pair_id, name, &mut config, &self.state.config_path) {
            Some(result) => {
                let cname = result.client_name.as_deref().unwrap_or("?");
                let key = result.client_key.as_deref().unwrap_or("?");
                self.send(
                    chat_id,
                    &format!(
                        "Approved <b>{}</b>\nKey: <code>{}</code>",
                        esc(cname),
                        esc(key)
                    ),
                )
                .await;
            }
            None => {
                self.send(chat_id, "Pairing not found or already processed.")
                    .await;
            }
        }
    }

    async fn cmd_rmclient(&self, chat_id: i64, client_name: &str) {
        let config = self.state.config.read().await;
        let exists = config.clients.iter().any(|c| c.name == client_name);
        drop(config);

        if !exists {
            self.send(chat_id, &format!("Client <b>{}</b> not found.", esc(client_name)))
                .await;
            return;
        }

        let buttons = vec![vec![
            InlineKeyboardButton {
                text: format!("Confirm remove {}", client_name),
                callback_data: format!("rmclient:{}", client_name),
            },
        ]];

        self.send_with_markup(
            chat_id,
            &format!("Remove client <b>{}</b>?", esc(client_name)),
            Some(InlineKeyboardMarkup {
                inline_keyboard: buttons,
            }),
        )
        .await;
    }

    async fn cmd_rmsession(&self, chat_id: i64, session_id: &str) {
        let sessions = self.state.sessions.read().await;
        if sessions.get(session_id).is_none() {
            self.send(chat_id, "Session not found.").await;
            return;
        }
        drop(sessions);

        let buttons = vec![vec![InlineKeyboardButton {
            text: "Confirm remove".to_string(),
            callback_data: format!("rmsession:{}", session_id),
        }]];

        self.send_with_markup(
            chat_id,
            &format!("Remove session <code>{}</code>?", esc(session_id)),
            Some(InlineKeyboardMarkup {
                inline_keyboard: buttons,
            }),
        )
        .await;
    }

    /// Notify admin about new pairing request
    pub async fn notify_pairing(&self, hostname: &str, ip: &str, pair_id: &str) {
        let chat_id = match self.allowed_chat {
            Some(id) => id,
            None => return,
        };

        let text = format!(
            "New pairing request\nHost: <b>{}</b>\nIP: <code>{}</code>",
            esc(hostname),
            esc(ip),
        );

        let buttons = vec![vec![
            InlineKeyboardButton {
                text: "Approve".to_string(),
                callback_data: format!("approve:{}:{}", pair_id, hostname),
            },
            InlineKeyboardButton {
                text: "Reject".to_string(),
                callback_data: format!("reject:{}", pair_id),
            },
        ]];

        self.send_with_markup(
            chat_id,
            &text,
            Some(InlineKeyboardMarkup {
                inline_keyboard: buttons,
            }),
        )
        .await;
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_age(ts: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let secs = (now.saturating_sub(ts)) / 1000;
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

pub type SharedBot = std::sync::Arc<tokio::sync::RwLock<Bot>>;

pub fn create_bot(token: String, state: AppState, admin_chat_id: Option<i64>) -> SharedBot {
    std::sync::Arc::new(tokio::sync::RwLock::new(Bot::new(token, state, admin_chat_id)))
}

pub async fn run_polling(bot: SharedBot) {
    let mut offset: i64 = 0;

    println!("[telegram] Bot starting...");

    // Verify token
    {
        let b = bot.read().await;
        let url = b.api_url("getMe");
        match b.http.get(&url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if data["ok"].as_bool() == Some(true) {
                        let username = data["result"]["username"].as_str().unwrap_or("?");
                        println!("[telegram] Bot connected: @{}", username);
                    } else {
                        println!("[telegram] ERROR: Invalid bot token");
                        return;
                    }
                }
            }
            Err(e) => {
                println!("[telegram] ERROR: Cannot connect: {}", e);
                return;
            }
        }
    }

    loop {
        let url = {
            let b = bot.read().await;
            format!(
                "{}?offset={}&timeout=30&allowed_updates=[\"message\",\"callback_query\"]",
                b.api_url("getUpdates"),
                offset
            )
        };

        let resp = {
            let b = bot.read().await;
            b.http.get(&url).timeout(std::time::Duration::from_secs(35)).send().await
        };

        let updates: Vec<Update> = match resp {
            Ok(r) => match r.json::<TgResponse<Vec<Update>>>().await {
                Ok(data) if data.ok => data.result.unwrap_or_default(),
                _ => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            },
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        for update in updates {
            offset = update.update_id + 1;

            if let Some(msg) = update.message {
                bot.write().await.handle_message(msg).await;
            }
            if let Some(cb) = update.callback_query {
                bot.read().await.handle_callback(cb).await;
            }
        }
    }
}

pub async fn notify_new_pairing(bot: &SharedBot, hostname: &str, ip: &str, pair_id: &str) {
    bot.read().await.notify_pairing(hostname, ip, pair_id).await;
}
