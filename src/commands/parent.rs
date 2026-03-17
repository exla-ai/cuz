use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::intent;

pub fn run_start(goal: &str) -> Result<()> {
    let cuz_dir = intent::find_cuz_dir()?;

    // Check if there's already an active parent
    if let Some(existing) = intent::read_active_parent()? {
        anyhow::bail!(
            "Parent {} is already active. Run `cuz parent end` first.",
            existing
        );
    }

    let id = generate_parent_id();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let parent = intent::ParentIntent {
        id: id.clone(),
        version: "0.1".to_string(),
        goal: goal.to_string(),
        child_intents: vec![],
        timestamp: now,
    };

    let path = cuz_dir.join("parents").join(format!("{}.json", id));
    fs::write(&path, serde_json::to_string_pretty(&parent)?)?;

    let active_path = cuz_dir.join("active_parent");
    fs::write(&active_path, &id)?;

    println!("Started parent intent {} — {}", id.cyan(), goal.bold());
    Ok(())
}

pub fn run_end() -> Result<()> {
    let cuz_dir = intent::find_cuz_dir()?;
    let active_path = cuz_dir.join("active_parent");

    let parent_id = intent::read_active_parent()?
        .context("No active parent intent. Start one with `cuz parent start \"goal\"`")?;

    fs::remove_file(&active_path)?;

    let goal = intent::read_parent_intent(&parent_id)
        .map(|p| p.goal)
        .unwrap_or_else(|_| "unknown".to_string());

    println!("Ended parent intent {} — {}", parent_id.cyan(), goal);
    Ok(())
}

pub fn run_show() -> Result<()> {
    match intent::read_active_parent()? {
        Some(id) => {
            let parent = intent::read_parent_intent(&id)?;
            println!("{} {}", "Parent:".dimmed(), id.cyan());
            println!("{} {}", "Goal:".dimmed(), parent.goal.bold());
            println!(
                "{} {}",
                "Children:".dimmed(),
                if parent.child_intents.is_empty() {
                    "none yet".dimmed().to_string()
                } else {
                    parent.child_intents.join(", ")
                }
            );
            println!("{} {}", "Started:".dimmed(), parent.timestamp.dimmed());
        }
        None => {
            println!("{}", "No active parent intent.".dimmed());
        }
    }
    Ok(())
}

fn generate_parent_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("cuz_parent_{:06x}", seed & 0xFFFFFF)
}
