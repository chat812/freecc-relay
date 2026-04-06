use crate::auth::{generate_key, save_config, Client, Config};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct PairingRequest {
    pub id: String,
    pub hostname: String,
    pub ip: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    pub status: String, // pending | approved | rejected
    #[serde(rename = "clientKey")]
    pub client_key: Option<String>,
    #[serde(rename = "clientName")]
    pub client_name: Option<String>,
}

pub struct PairingManager {
    pub requests: HashMap<String, PairingRequest>,
}

impl PairingManager {
    pub fn new() -> Self {
        PairingManager {
            requests: HashMap::new(),
        }
    }

    pub fn create(&mut self, hostname: &str, ip: &str) -> PairingRequest {
        let id = format!("pair_{}", crate::auth::generate_random_base64(12));
        let request = PairingRequest {
            id: id.clone(),
            hostname: hostname.to_string(),
            ip: ip.to_string(),
            created_at: now_ms(),
            status: "pending".to_string(),
            client_key: None,
            client_name: None,
        };
        self.requests.insert(id, request.clone());
        request
    }

    pub fn get(&self, id: &str) -> Option<&PairingRequest> {
        self.requests.get(id)
    }

    pub fn list_all(&self) -> Vec<PairingRequest> {
        let mut result: Vec<PairingRequest> = self.requests.values().cloned().collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        result
    }

    pub fn approve(
        &mut self,
        id: &str,
        name: Option<&str>,
        config: &mut Config,
        config_path: &PathBuf,
    ) -> Option<PairingRequest> {
        let request = self.requests.get_mut(id)?;
        if request.status != "pending" {
            return None;
        }

        let client_key = generate_key("ck");
        let client_name = name
            .filter(|n| !n.is_empty())
            .unwrap_or(&request.hostname)
            .to_string();

        config.clients.push(Client {
            name: client_name.clone(),
            key: client_key.clone(),
        });
        save_config(config, config_path);

        request.status = "approved".to_string();
        request.client_key = Some(client_key);
        request.client_name = Some(client_name);

        Some(request.clone())
    }

    pub fn reject(&mut self, id: &str) -> bool {
        let request = match self.requests.get_mut(id) {
            Some(r) => r,
            None => return false,
        };
        if request.status != "pending" {
            return false;
        }
        request.status = "rejected".to_string();
        true
    }

    /// Remove expired pairing requests:
    /// - pending: 10 minutes
    /// - approved: 5 minutes
    /// - rejected: 1 minute
    pub fn cleanup_expired(&mut self) {
        let now = now_ms();
        self.requests.retain(|_, r| {
            let age = now - r.created_at;
            match r.status.as_str() {
                "pending" => age < 10 * 60 * 1000,
                "approved" => age < 5 * 60 * 1000,
                "rejected" => age < 60 * 1000,
                _ => false,
            }
        });
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
