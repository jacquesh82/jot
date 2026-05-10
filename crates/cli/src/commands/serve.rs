use crate::config::Config;
use crate::error::CliError;
use api::{
    auth::{make_claims, sign_token},
    AppState,
};
use ed25519_dalek::pkcs8::spki::EncodePublicKey;
use ed25519_dalek::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use jot_core::crypto::signing::generate_device_keypair;
use std::net::SocketAddr;
use std::sync::Arc;
use storage::{Db, LocalStore};
use uuid::Uuid;

pub async fn run(port: u16, open_registration: bool) -> Result<(), CliError> {
    let mut config = Config::load();

    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("jot");
    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("jot.db");
    let blobs_path = data_dir.join("blobs");
    std::fs::create_dir_all(&blobs_path)?;

    let db_url = format!("sqlite://{}", db_path.display());
    let db = Db::connect(&db_url)
        .await
        .map_err(|e| CliError::Server(e.to_string()))?;
    db.migrate()
        .await
        .map_err(|e| CliError::Server(e.to_string()))?;
    let schema_version = db
        .schema_version()
        .await
        .map_err(|e| CliError::Server(e.to_string()))?;
    println!("Database schema: v{}", schema_version);

    let key_path = data_dir.join("server_key.pem");
    let (signing_pem, verifying_pem) = load_or_generate_keypair(&key_path)?;

    let blobs = Arc::new(LocalStore::new(&blobs_path));
    let state = AppState::new(db, blobs, signing_pem.clone(), verifying_pem)
        .with_open_registration(open_registration)
        .with_schema_version(schema_version);
    if open_registration {
        println!("Open registration enabled — anyone can create an account.");
    }
    let router = api::build_router(state.clone());

    if config.token.is_none() {
        let device_id = config
            .device_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);
        let identity_id = config
            .identity_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let (_, dev_verifying) = generate_device_keypair();
        let dev_pub_ed = hex::encode(dev_verifying.to_bytes());
        let dev_pub_x = "00".repeat(32);

        let device = jot_core::models::Device {
            id: device_id,
            identity_id,
            pub_key_x25519: dev_pub_x,
            pub_key_ed25519: dev_pub_ed,
            name: hostname(),
            last_seen: chrono::Utc::now(),
        };

        state
            .db
            .insert_device(&device)
            .await
            .map_err(|e| CliError::Server(e.to_string()))?;

        let claims = make_claims(&device_id.to_string(), &identity_id.to_string());
        let token =
            sign_token(&claims, &signing_pem).map_err(|e| CliError::Server(e.to_string()))?;

        // Insert identity record with a friendly name derived from the identity UUID
        let friendly_name = format!("user-{}", &identity_id.to_string()[..8]);
        state.db.insert_identity(&identity_id.to_string(), &friendly_name)
            .await
            .map_err(|e| CliError::Server(e.to_string()))?;
        println!("Identity friendly name: {}", friendly_name);

        config.token = Some(token);
        config.device_id = Some(device_id.to_string());
        config.identity_id = Some(identity_id.to_string());
        config.save()?;
        println!("Device registered. Token saved to config.");

        // Auto-create a default board so the user can start immediately
        let default_board = jot_core::models::Board {
            id: uuid::Uuid::new_v4(),
            identity_id,
            name: "Default".to_string(),
            position: 0,
            created_at: chrono::Utc::now(),
        };
        state
            .db
            .insert_board(&default_board)
            .await
            .map_err(|e| CliError::Server(e.to_string()))?;
        println!("Default board created ({}).", default_board.id);
    }

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| CliError::Server(e.to_string()))?;
    axum::serve(listener, router)
        .await
        .map_err(|e| CliError::Server(e.to_string()))?;

    Ok(())
}

fn load_or_generate_keypair(key_path: &std::path::Path) -> Result<(String, String), CliError> {
    if key_path.exists() {
        let pem = std::fs::read_to_string(key_path)?;
        let signing_key = ed25519_dalek::SigningKey::from_pkcs8_pem(&pem)
            .map_err(|e| CliError::Config(e.to_string()))?;
        let v_pem = signing_key
            .verifying_key()
            .to_public_key_pem(Default::default())
            .map_err(|e| CliError::Config(e.to_string()))?;
        Ok((pem, v_pem))
    } else {
        let (signing_key, _) = generate_device_keypair();
        let s_pem = signing_key
            .to_pkcs8_pem(Default::default())
            .map_err(|e| CliError::Config(e.to_string()))?
            .to_string();
        let v_pem = signing_key
            .verifying_key()
            .to_public_key_pem(Default::default())
            .map_err(|e| CliError::Config(e.to_string()))?;
        std::fs::write(key_path, &s_pem)?;
        Ok((s_pem, v_pem))
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "jot-device".to_string())
}
