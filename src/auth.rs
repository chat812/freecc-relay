use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub mode: String,
    #[serde(default = "default_session_expiry")]
    pub session_expiry: String,
    #[serde(default = "default_max_sessions")]
    pub max_sessions_per_key: usize,
    #[serde(default = "default_web_access_policy")]
    pub web_access_policy: String,
}

fn default_session_expiry() -> String {
    "24h".to_string()
}
fn default_max_sessions() -> usize {
    10
}
fn default_web_access_policy() -> String {
    "token".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub auth: AuthConfig,
    pub clients: Vec<Client>,
}

pub fn generate_key(prefix: &str) -> String {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{}_{}", prefix, URL_SAFE_NO_PAD.encode(bytes))
}

pub fn generate_random_base64(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(&bytes)
}

pub fn load_config(config_path: &Path) -> Config {
    if config_path.exists() {
        let data = fs::read_to_string(config_path).expect("Failed to read config");
        serde_json::from_str(&data).expect("Failed to parse config")
    } else {
        let config = Config {
            auth: AuthConfig {
                mode: "key".to_string(),
                session_expiry: "24h".to_string(),
                max_sessions_per_key: 10,
                web_access_policy: "token".to_string(),
            },
            clients: vec![Client {
                name: "default".to_string(),
                key: generate_key("ck"),
            }],
        };
        if let Some(dir) = config_path.parent() {
            fs::create_dir_all(dir).ok();
        }
        save_config(&config, config_path);
        config
    }
}

pub fn save_config(config: &Config, config_path: &Path) {
    if let Some(dir) = config_path.parent() {
        fs::create_dir_all(dir).ok();
    }
    let data = serde_json::to_string_pretty(config).unwrap();
    fs::write(config_path, data).expect("Failed to write config");
}

/// Returns Some(client_name) if valid, None if not.
pub fn authenticate_client(config: &Config, key: &str) -> Option<String> {
    if config.auth.mode == "open" {
        return Some("anonymous".to_string());
    }
    config
        .clients
        .iter()
        .find(|c| c.key == key)
        .map(|c| c.name.clone())
}
