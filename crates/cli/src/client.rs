use crate::config::Config;
use crate::error::CliError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSummary {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSummary {
    pub id: Uuid,
    pub name: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSummary {
    pub id: Uuid,
    pub note_type: String,
    pub blob_key: String,
    pub color: String,
    pub position: i32,
}

#[derive(Clone)]
pub struct JotClient {
    pub base_url: String,
    pub token: Option<String>,
    inner: reqwest::Client,
}

impl JotClient {
    pub fn new(config: &Config) -> Self {
        Self {
            base_url: config.server_url().to_string(),
            token: config.token.clone(),
            inner: reqwest::Client::new(),
        }
    }

    pub fn from_config() -> Self {
        Self::new(&Config::load())
    }

    fn auth_header(&self) -> Result<String, CliError> {
        self.token
            .as_ref()
            .map(|t| format!("Bearer {}", t))
            .ok_or(CliError::NotAuthenticated)
    }

    pub async fn create_board(&self, name: &str) -> Result<BoardSummary, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}/boards", self.base_url))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "name": name, "position": 0 }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }

    pub async fn get_boards(&self) -> Result<Vec<BoardSummary>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/boards", self.base_url))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }

    pub async fn get_notes(&self, board_id: Uuid) -> Result<Vec<NoteSummary>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/notes", self.base_url))
            .query(&[("board_id", board_id.to_string())])
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }

    pub async fn create_note(&self, board_id: Uuid, content: &str) -> Result<Uuid, CliError> {
        let auth = self.auth_header()?;
        let blob_key = Uuid::new_v4().to_string();
        let body = serde_json::json!({
            "note_type": "text",
            "board_id": board_id,
            "blob_key": blob_key,
            "size": content.len() as i64,
        });
        let resp = self
            .inner
            .post(format!("{}/notes", self.base_url))
            .header("Authorization", auth.clone())
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        let created: serde_json::Value = resp.json().await?;
        let note_id: Uuid = created["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| CliError::Server("missing note id in response".into()))?;

        // Upload blob
        let blob_resp = self
            .inner
            .put(format!("{}/notes/{}/blob", self.base_url, note_id))
            .header("Authorization", auth)
            .body(content.as_bytes().to_vec())
            .send()
            .await?;
        if !blob_resp.status().is_success() {
            return Err(CliError::Server(blob_resp.status().to_string()));
        }
        Ok(note_id)
    }

    pub async fn get_blob(&self, note_id: Uuid) -> Result<Vec<u8>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/notes/{}/blob", self.base_url, note_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn get_devices(&self) -> Result<Vec<DeviceSummary>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/devices", self.base_url))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }

    pub async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", auth)
            .json(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(resp.json().await?)
    }

    pub async fn confirm_link(&self, token: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}/link/confirm", self.base_url))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "token": token }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn delete_note(&self, note_id: Uuid) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}/notes/{}", self.base_url, note_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn register_device(
        &self,
        device_id: Uuid,
        identity_id: Uuid,
        pub_key_x25519: &str,
        pub_key_ed25519: &str,
        name: &str,
    ) -> Result<String, CliError> {
        let body = serde_json::json!({
            "device_id": device_id,
            "identity_id": identity_id,
            "pub_key_x25519": pub_key_x25519,
            "pub_key_ed25519": pub_key_ed25519,
            "name": name,
        });
        let resp = self
            .inner
            .post(format!("{}/auth/device", self.base_url))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        let json: serde_json::Value = resp.json().await?;
        json["token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| CliError::Server("missing token in response".into()))
    }
}
