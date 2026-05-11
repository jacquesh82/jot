use crate::error::CliError;
use crate::t;
use storage::Db;

pub async fn run() -> Result<(), CliError> {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("jot");

    let db_path = data_dir.join("jot.db");
    if !db_path.exists() {
        println!("{}", t!("cmd.migrate.noDb", "path" => db_path.display()));
        println!("{}", t!("cmd.migrate.runServe"));
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
        println!("{}", t!("cmd.migrate.migrated", "from" => before, "to" => after));
    } else {
        println!("{}", t!("cmd.migrate.upToDate", "v" => after));
    }

    Ok(())
}
