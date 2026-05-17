use crate::client::JotClient;
use crate::error::CliError;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum TagCmd {
    /// List all tags
    List,
    /// List block ids carrying a tag
    Blocks { name: String },
    /// Create or update a tag's metadata (color JSON pass-through)
    Set {
        name: String,
        /// JSON body, e.g. '{"color":"#abc"}'
        #[arg(long, default_value = "{}")]
        json: String,
    },
}

pub async fn run(cmd: TagCmd) -> Result<(), CliError> {
    let client = JotClient::from_config();
    match cmd {
        TagCmd::List => {
            let body = client.get_json("/tags").await?;
            if let Some(arr) = body.as_array() {
                if arr.is_empty() {
                    println!("(no tags)");
                }
                for t in arr {
                    let name = t["name"].as_str().unwrap_or("");
                    let color = t["color"].as_str().unwrap_or("");
                    println!("  {name}\t{color}");
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&body).unwrap_or_default()
                );
            }
        }
        TagCmd::Blocks { name } => {
            let body = client.get_json(&format!("/tags/{}/blocks", name)).await?;
            if let Some(arr) = body.as_array() {
                if arr.is_empty() {
                    println!("(no blocks for tag {name})");
                }
                for b in arr {
                    let id = b["id"].as_str().or(b.as_str()).unwrap_or("");
                    println!("  {id}");
                }
            }
        }
        TagCmd::Set { name, json } => {
            let v: serde_json::Value = serde_json::from_str(&json)
                .map_err(|e| CliError::Config(format!("bad --json: {e}")))?;
            client.put_json(&format!("/tags/{}", name), &v).await?;
            println!("tag updated");
        }
    }
    Ok(())
}
