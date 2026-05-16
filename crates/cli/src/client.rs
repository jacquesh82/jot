use crate::config::Config;
use crate::error::CliError;
use crate::identity;
use jot_core::crypto::{decrypt, derive_bek, derive_dek, encrypt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareEntry {
    pub shared_with_id: String,
    pub shared_with_name: Option<String>,
    pub permission: Option<String>,
    pub public_key_x25519: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityInfo {
    pub id: String,
    pub friendly_name: String,
    pub public_key_x25519: Option<String>,
}

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
    pub snippet: Option<String>,
    #[serde(default)]
    pub schema_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDetail {
    pub id: Uuid,
    pub board_id: Uuid,
    pub note_type: String,
    pub blob_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedBoardSummary {
    pub board_id: Uuid,
    pub board_name: String,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedNoteSummary {
    pub note_id: Uuid,
    pub note_type: String,
    pub board_id: Uuid,
    pub owner_identity_id: String,
    pub owner_friendly_name: Option<String>,
    pub snippet: Option<String>,
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

    pub async fn get_json(&self, path: &str) -> Result<serde_json::Value, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(resp.json().await?)
    }

    pub async fn delete_path(&self, path: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}{}", self.base_url, path))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn patch_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .patch(format!("{}{}", self.base_url, path))
            .header("Authorization", auth)
            .json(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        if resp.content_length() == Some(0) {
            return Ok(serde_json::Value::Null);
        }
        resp.json().await.or(Ok(serde_json::Value::Null))
    }

    pub async fn put_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .put(format!("{}{}", self.base_url, path))
            .header("Authorization", auth)
            .json(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        if resp.content_length() == Some(0) {
            return Ok(serde_json::Value::Null);
        }
        resp.json().await.or(Ok(serde_json::Value::Null))
    }

    pub async fn post_json_optauth(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, CliError> {
        let mut req = self
            .inner
            .post(format!("{}{}", self.base_url, path))
            .json(body);
        if let Some(t) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(resp.json().await?)
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

    /// Create and upload an E2E-encrypted note.
    /// DEK is derived deterministically: BEK = HKDF(privkey, board_id), DEK = HKDF(BEK, note_id).
    pub async fn create_note(&self, board_id: Uuid, content: &str) -> Result<Uuid, CliError> {
        let auth = self.auth_header()?;
        let (secret, public) = identity::load_or_generate()?;

        // Register our public key (idempotent — needed for board sharing).
        self.register_pubkey(&hex::encode(public.as_bytes())).await?;

        // Create note record first to obtain the note_id (needed for DEK derivation).
        let blob_key = Uuid::new_v4().to_string();
        let body = serde_json::json!({
            "note_type": "text",
            "board_id": board_id,
            "blob_key": blob_key,
            "size": 0,
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

        // Derive DEK from identity key + board_id + note_id.
        let dek = self.derive_dek_for(&secret, board_id, note_id)?;
        let ciphertext = encrypt(&dek, content.as_bytes())
            .map_err(|e| CliError::Server(format!("encryption failed: {e}")))?;

        // Upload encrypted blob and update size.
        let size = ciphertext.len() as i64;
        let blob_resp = self
            .inner
            .put(format!("{}/notes/{}/blob", self.base_url, note_id))
            .header("Authorization", auth.clone())
            .body(ciphertext)
            .send()
            .await?;
        if !blob_resp.status().is_success() {
            return Err(CliError::Server(blob_resp.status().to_string()));
        }
        // Patch the note size now that we know it.
        let _ = self
            .inner
            .patch(format!("{}/notes/{}", self.base_url, note_id))
            .header("Authorization", auth.clone())
            .json(&serde_json::json!({ "size": size }))
            .send()
            .await;

        Ok(note_id)
    }

    /// Fetch note metadata (needed for board_id to derive the DEK).
    pub async fn get_note_meta(&self, note_id: Uuid) -> Result<NoteDetail, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/notes/{}", self.base_url, note_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(format!("note {} not found", note_id)));
        }
        Ok(resp.json().await?)
    }

    /// Fetch encrypted blob and decrypt it using the derived DEK.
    pub async fn get_note_text(&self, note_id: Uuid) -> Result<String, CliError> {
        let auth = self.auth_header()?;

        // Need board_id to derive the DEK.
        let meta = self.get_note_meta(note_id).await?;

        let resp = self
            .inner
            .get(format!("{}/notes/{}/blob", self.base_url, note_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        let ciphertext = resp.bytes().await?.to_vec();

        let dek = self.derive_dek_local(note_id, meta.board_id).await?;
        let plaintext = decrypt(&dek, &ciphertext)
            .map_err(|_| CliError::Server("decryption failed".into()))?;
        String::from_utf8(plaintext)
            .map_err(|_| CliError::Server("note content is not valid UTF-8".into()))
    }

    /// Derive DEK for this identity (owner path): BEK = HKDF(privkey, board_id), DEK = HKDF(BEK, note_id).
    async fn derive_dek_local(&self, note_id: Uuid, board_id: Uuid) -> Result<[u8; 32], CliError> {
        let (secret, _) = identity::load_or_generate()?;
        self.derive_dek_for(&secret, board_id, note_id)
    }

    fn derive_dek_for(
        &self,
        secret: &x25519_dalek::StaticSecret,
        board_id: Uuid,
        note_id: Uuid,
    ) -> Result<[u8; 32], CliError> {
        let privkey = secret.to_bytes();
        let bek = derive_bek(&privkey, board_id.as_bytes())
            .map_err(|e| CliError::Server(format!("BEK derivation failed: {e}")))?;
        derive_dek(&bek, note_id.as_bytes())
            .map_err(|e| CliError::Server(format!("DEK derivation failed: {e}")))
    }

    /// Fetch raw encrypted blob bytes (for binary content like images/audio).
    #[allow(dead_code)]
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

    pub async fn register_pubkey(&self, pubkey_hex: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .put(format!("{}/identity/me/pubkey", self.base_url))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "public_key_x25519": pubkey_hex }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }


    /// Look up an identity by friendly name or UUID.
    pub async fn lookup_identity(&self, name_or_id: &str) -> Result<Option<IdentityInfo>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/identity/lookup/{}", self.base_url, name_or_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if resp.status().as_u16() == 404 { return Ok(None); }
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(Some(resp.json().await?))
    }

    pub async fn list_note_shares(&self, note_id: Uuid) -> Result<Vec<ShareEntry>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/notes/{}/shares", self.base_url, note_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }

    /// Share a note with a recipient. Derives the DEK locally and re-encrypts for them.
    pub async fn share_note(&self, note_id: Uuid, target: &str, permission: &str) -> Result<(), CliError> {
        let target_info = self.lookup_identity(target).await?
            .ok_or_else(|| CliError::Server(format!("identity \"{target}\" not found")))?;
        let pubkey_hex = target_info.public_key_x25519
            .ok_or_else(|| CliError::Server(format!("\"{target}\" has no public key registered — they must create a note first")))?;

        let (secret, public) = identity::load_or_generate()?;

        // Register our public key so the recipient can derive the ECDH wrap key.
        self.register_pubkey(&hex::encode(public.as_bytes())).await?;

        // Derive DEK locally (no server round-trip needed for owner).
        let meta = self.get_note_meta(note_id).await?;
        let raw_dek = self.derive_dek_for(&secret, meta.board_id, note_id)?;

        let recipient_wrap = identity::cross_wrap_key(&secret, &pubkey_hex)?;
        let encrypted_for_recipient = encrypt(&recipient_wrap, &raw_dek)
            .map_err(|e| CliError::Server(format!("DEK re-encryption failed: {e}")))?;

        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}/notes/{}/shares", self.base_url, note_id))
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "target": target,
                "encrypted_dek_for_recipient": hex::encode(&encrypted_for_recipient),
                "permission": permission,
            }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    /// Revoke a note share: removes the recipient's DEK entry from note_shares.
    /// With deterministic BEK→DEK derivation the owner's DEK never changes.
    pub async fn revoke_share(&self, note_id: Uuid, target_id: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self.inner
            .delete(format!("{}/notes/{}/shares/{}", self.base_url, note_id, target_id))
            .header("Authorization", auth)
            .send().await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    /// Store the BEK encrypted for a board member.
    pub async fn put_board_key(
        &self,
        board_id: Uuid,
        identity_id: &str,
        encrypted_bek_hex: &str,
    ) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .put(format!("{}/boards/{}/keys/{}", self.base_url, board_id, identity_id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "encrypted_bek": encrypted_bek_hex }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    /// Delete a member's BEK (revoke board access).
    pub async fn delete_board_key(&self, board_id: Uuid, identity_id: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}/boards/{}/keys/{}", self.base_url, board_id, identity_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn share_board(&self, board_id: Uuid, target: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}/boards/{}/shares", self.base_url, board_id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "target": target }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn revoke_board_share(&self, board_id: Uuid, identity_id: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}/boards/{}/shares/{}", self.base_url, board_id, identity_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }


    pub async fn get_identity_me(&self) -> Result<IdentityInfo, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/identity/me", self.base_url))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
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

    pub async fn delete_device(&self, id: Uuid) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}/devices/{}", self.base_url, id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    pub async fn rename_device(&self, id: Uuid, name: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}/devices/{}/rename", self.base_url, id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
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

    pub async fn link_status(&self, token: &str) -> Result<(String, Option<String>), CliError> {
        let resp = self
            .inner
            .get(format!("{}/link/status/{}", self.base_url, token))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        let v: serde_json::Value = resp.json().await?;
        let status = v["status"].as_str().unwrap_or("").to_string();
        let jwt = v["jwt"].as_str().map(str::to_owned);
        Ok((status, jwt))
    }

    pub async fn rename_board(&self, id: Uuid, name: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .patch(format!("{}/boards/{}", self.base_url, id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    pub async fn delete_board(&self, id: Uuid) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}/boards/{}", self.base_url, id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    pub async fn update_note(&self, note_id: Uuid, content: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let meta = self.get_note_meta(note_id).await?;
        let (secret, _) = identity::load_or_generate()?;
        let dek = self.derive_dek_for(&secret, meta.board_id, note_id)?;
        let ciphertext = encrypt(&dek, content.as_bytes())
            .map_err(|e| CliError::Server(format!("encryption failed: {e}")))?;
        let blob_resp = self
            .inner
            .put(format!("{}/notes/{}/blob", self.base_url, note_id))
            .header("Authorization", auth.clone())
            .body(ciphertext)
            .send()
            .await?;
        if !blob_resp.status().is_success() {
            return Err(CliError::Server(blob_resp.status().to_string()));
        }
        let snippet: String = content.chars().take(80).collect();
        let _ = self
            .inner
            .patch(format!("{}/notes/{}", self.base_url, note_id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "snippet": snippet }))
            .send()
            .await;
        Ok(())
    }

    /// Encrypt a title with the note's DEK and PATCH it. `text.is_empty()` clears the title.
    pub async fn set_note_title(&self, note_id: Uuid, text: &str) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let title_b64: Option<String> = if text.is_empty() {
            None
        } else {
            let meta = self.get_note_meta(note_id).await?;
            let dek = self.derive_dek_local(note_id, meta.board_id).await?;
            let ct = encrypt(&dek, text.as_bytes())
                .map_err(|e| CliError::Server(format!("encryption failed: {e}")))?;
            use base64::Engine;
            Some(base64::engine::general_purpose::STANDARD.encode(&ct))
        };
        let resp = self
            .inner
            .patch(format!("{}/notes/{}/title", self.base_url, note_id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "title_b64": title_b64 }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    pub async fn get_shared_boards(&self) -> Result<Vec<SharedBoardSummary>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/boards/shared", self.base_url))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }

    pub async fn get_shared_notes(&self) -> Result<Vec<SharedNoteSummary>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/notes/shared", self.base_url))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }

    pub async fn delete_account(&self) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}/identity/me", self.base_url))
            .header("Authorization", auth)
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

    // ---------- Block operations (Task 11) ----------

    pub async fn list_blocks(&self, note_id: Uuid) -> Result<Vec<jot_core::models::Block>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/notes/{}/blocks", self.base_url, note_id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        let dtos: Vec<BlockDto> = resp.json().await?;
        dtos.into_iter().map(dto_to_block).collect()
    }

    pub async fn create_block(
        &self,
        note_id: Uuid,
        parent: Option<Uuid>,
        position: Option<f64>,
        block_type: &str,
        content: &[u8],
        metadata: Option<&[u8]>,
    ) -> Result<jot_core::models::Block, CliError> {
        use base64::Engine;
        let auth = self.auth_header()?;
        let b64 = base64::engine::general_purpose::STANDARD;
        let body = serde_json::json!({
            "parent_id": parent,
            "position": position,
            "block_type": block_type,
            "content_b64": b64.encode(content),
            "metadata_b64": metadata.map(|m| b64.encode(m)),
        });
        let resp = self
            .inner
            .post(format!("{}/notes/{}/blocks", self.base_url, note_id))
            .header("Authorization", auth)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        let dto: BlockDto = resp.json().await?;
        dto_to_block(dto)
    }

    pub async fn get_block(&self, id: Uuid) -> Result<jot_core::models::Block, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/blocks/{}", self.base_url, id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        let dto: BlockDto = resp.json().await?;
        dto_to_block(dto)
    }

    pub async fn patch_block(
        &self,
        id: Uuid,
        block_type: Option<&str>,
        content: Option<&[u8]>,
    ) -> Result<jot_core::models::Block, CliError> {
        use base64::Engine;
        let auth = self.auth_header()?;
        let b64 = base64::engine::general_purpose::STANDARD;
        let body = serde_json::json!({
            "block_type": block_type,
            "content_b64": content.map(|c| b64.encode(c)),
        });
        let resp = self
            .inner
            .patch(format!("{}/blocks/{}", self.base_url, id))
            .header("Authorization", auth)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        let dto: BlockDto = resp.json().await?;
        dto_to_block(dto)
    }

    pub async fn move_block(
        &self,
        id: Uuid,
        new_parent: Option<Uuid>,
        new_position: f64,
    ) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let body = serde_json::json!({
            "new_parent_id": new_parent,
            "new_position": new_position,
        });
        let resp = self
            .inner
            .post(format!("{}/blocks/{}/move", self.base_url, id))
            .header("Authorization", auth)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn indent_block(&self, id: Uuid) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}/blocks/{}/indent", self.base_url, id))
            .header("Authorization", auth)
            .json(&serde_json::json!({}))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn outdent_block(&self, id: Uuid) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .post(format!("{}/blocks/{}/outdent", self.base_url, id))
            .header("Authorization", auth)
            .json(&serde_json::json!({}))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn delete_block(&self, id: Uuid) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .delete(format!("{}/blocks/{}", self.base_url, id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(())
    }

    /// Update a note's schema_version. Used by `jot block migrate` to mark a note
    /// as fully migrated to the block-structured form.
    pub async fn set_note_schema_version(
        &self,
        note_id: Uuid,
        version: i32,
    ) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .patch(format!(
                "{}/notes/{}/schema-version",
                self.base_url, note_id
            ))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "schema_version": version }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    /// List ids of text notes owned by the caller that have not yet been
    /// migrated to block form (schema_version = 0).
    pub async fn list_legacy_text_notes(&self) -> Result<Vec<Uuid>, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/notes/legacy-text", self.base_url))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        let ids: Vec<String> = resp.json().await?;
        Ok(ids
            .into_iter()
            .filter_map(|s| Uuid::parse_str(&s).ok())
            .collect())
    }

    /// Decrypt a block ciphertext using the note's DEK.
    /// Mirrors `create_block_encrypted` / `get_note_text` derivation:
    /// BEK = HKDF(privkey, board_id), DEK = HKDF(BEK, note_id), then AES-GCM decrypt.
    pub async fn decrypt_with_note_dek(
        &self,
        board_id: Uuid,
        note_id: Uuid,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, CliError> {
        let (secret, _) = identity::load_or_generate()?;
        let dek = self.derive_dek_for(&secret, board_id, note_id)?;
        decrypt(&dek, ciphertext)
            .map_err(|e| CliError::Server(format!("block decryption failed: {e}")))
    }

    /// Encrypt block content with the note's DEK before posting it.
    /// Mirrors the encrypt-on-write path used by `create_note` / `update_note`.
    pub async fn create_block_encrypted(
        &self,
        note_id: Uuid,
        parent: Option<Uuid>,
        position: Option<f64>,
        block_type: &str,
        plaintext: &[u8],
    ) -> Result<jot_core::models::Block, CliError> {
        let meta = self.get_note_meta(note_id).await?;
        let (secret, _) = identity::load_or_generate()?;
        let dek = self.derive_dek_for(&secret, meta.board_id, note_id)?;
        let ciphertext = encrypt(&dek, plaintext)
            .map_err(|e| CliError::Server(format!("block encryption failed: {e}")))?;
        self.create_block(note_id, parent, position, block_type, &ciphertext, None)
            .await
    }

    /// Encrypt arbitrary plaintext with the note's DEK (mirror of `decrypt_with_note_dek`).
    pub async fn encrypt_with_note_dek(
        &self,
        board_id: Uuid,
        note_id: Uuid,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CliError> {
        let (secret, _) = identity::load_or_generate()?;
        let dek = self.derive_dek_for(&secret, board_id, note_id)?;
        encrypt(&dek, plaintext)
            .map_err(|e| CliError::Server(format!("block encryption failed: {e}")))
    }

    /// PATCH /blocks/:id with just `{ content_b64 }` — leaves type/metadata/collapsed alone.
    pub async fn patch_block_content_b64(
        &self,
        id: Uuid,
        content_b64: &str,
    ) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .patch(format!("{}/blocks/{}", self.base_url, id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "content_b64": content_b64 }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    /// Share a single block with another identity (re-encrypting its content for them).
    pub async fn share_block(
        &self,
        block_id: Uuid,
        target: &str,
        permission: &str,
    ) -> Result<(), CliError> {
        use base64::Engine;
        let target_info = self
            .lookup_identity(target)
            .await?
            .ok_or_else(|| CliError::Server(format!("identity \"{target}\" not found")))?;
        let pubkey_hex = target_info
            .public_key_x25519
            .ok_or_else(|| CliError::Server(format!("\"{target}\" has no public key registered")))?;

        let block = self.get_block(block_id).await?;
        let meta = self.get_note_meta(block.note_id).await?;
        let plaintext = self
            .decrypt_with_note_dek(meta.board_id, block.note_id, &block.content)
            .await?;

        let (secret, public) = identity::load_or_generate()?;
        self.register_pubkey(&hex::encode(public.as_bytes())).await?;
        let recipient_key = identity::cross_wrap_key(&secret, &pubkey_hex)?;
        let ciphertext = encrypt(&recipient_key, &plaintext)
            .map_err(|e| CliError::Server(format!("share encryption failed: {e}")))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&ciphertext);

        self.post_json(
            &format!("/blocks/{}/share", block_id),
            &serde_json::json!({
                "target": target,
                "encrypted_content_b64": b64,
                "permission": permission,
            }),
        )
        .await?;
        Ok(())
    }

    /// PATCH /blocks/:id with just `{ collapsed }` — for `za`-style fold toggles.
    pub async fn patch_block_collapse(&self, id: Uuid, collapsed: bool) -> Result<(), CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .patch(format!("{}/blocks/{}", self.base_url, id))
            .header("Authorization", auth)
            .json(&serde_json::json!({ "collapsed": collapsed }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }

    pub async fn block_backlinks(&self, id: Uuid) -> Result<serde_json::Value, CliError> {
        let auth = self.auth_header()?;
        let resp = self
            .inner
            .get(format!("{}/blocks/{}/backlinks", self.base_url, id))
            .header("Authorization", auth)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(CliError::Server(resp.status().to_string()));
        }
        Ok(resp.json().await?)
    }
}

// ---------- Block DTO helpers (Task 11) ----------

#[derive(Debug, Clone, Deserialize)]
struct BlockDto {
    id: String,
    note_id: String,
    parent_block_id: Option<String>,
    position: f64,
    block_type: String,
    content: String,
    metadata: Option<String>,
    collapsed: bool,
    created_at: String,
    updated_at: String,
}

fn b64decode(s: &str) -> Result<Vec<u8>, CliError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| CliError::Server(format!("invalid base64 in block: {e}")))
}

fn dto_to_block(d: BlockDto) -> Result<jot_core::models::Block, CliError> {
    use chrono::{DateTime, Utc};
    use jot_core::models::{Block, BlockType};
    Ok(Block {
        id: Uuid::parse_str(&d.id).map_err(|e| CliError::Server(format!("bad block id: {e}")))?,
        note_id: Uuid::parse_str(&d.note_id)
            .map_err(|e| CliError::Server(format!("bad note id: {e}")))?,
        parent_block_id: d
            .parent_block_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
        position: d.position,
        block_type: BlockType::from_str(&d.block_type),
        content: b64decode(&d.content)?,
        metadata: d.metadata.as_deref().map(b64decode).transpose()?,
        collapsed: d.collapsed,
        created_at: DateTime::parse_from_rfc3339(&d.created_at)
            .map_err(|e| CliError::Server(format!("bad created_at: {e}")))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&d.updated_at)
            .map_err(|e| CliError::Server(format!("bad updated_at: {e}")))?
            .with_timezone(&Utc),
    })
}
