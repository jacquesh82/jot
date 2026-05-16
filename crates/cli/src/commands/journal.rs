use crate::client::JotClient;
use crate::error::CliError;
use std::collections::BTreeMap;

pub async fn run(date: Option<String>) -> Result<(), CliError> {
    let client = JotClient::from_config();
    let boards = client.get_boards().await?;
    let mut by_day: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for b in &boards {
        let raw = client
            .get_json(&format!("/notes?board_id={}", b.id))
            .await?;
        for n in raw.as_array().cloned().unwrap_or_default() {
            let id = n["id"].as_str().unwrap_or("").to_string();
            let created = n["created_at"].as_str().unwrap_or("");
            let day = created.get(..10).unwrap_or(created).to_string();
            by_day.entry(day).or_default().push((id, b.name.clone()));
        }
    }
    if let Some(d) = date {
        let entries = by_day.remove(&d).unwrap_or_default();
        println!("{d} ({} note(s))", entries.len());
        for (id, board) in entries {
            println!("  {id}  [{board}]");
        }
    } else {
        for (d, entries) in by_day.iter().rev() {
            println!("{d} ({})", entries.len());
            for (id, board) in entries {
                println!("  {id}  [{board}]");
            }
        }
    }
    Ok(())
}
