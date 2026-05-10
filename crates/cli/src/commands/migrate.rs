use crate::error::CliError;
use storage::Db;

pub async fn run() -> Result<(), CliError> {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("jot");

    let db_path = data_dir.join("jot.db");
    if !db_path.exists() {
        println!("No database found at {}.", db_path.display());
        println!("Run `jot serve` first to create the database.");
        return Ok(());
    }

    let db_url = format!("sqlite://{}", db_path.display());
    let db = Db::connect(&db_url)
        .await
        .map_err(|e| CliError::Server(e.to_string()))?;

    let (before, after) = db
        .migrate_with_version()
        .await
        .map_err(|e| CliError::Server(e.to_string()))?;

    if after > before {
        println!("Database schema migrated: v{} → v{}", before, after);
    } else {
        println!("Database schema already up to date (v{}).", after);
    }

    Ok(())
}
