use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::prompt;

pub fn run() -> Result<()> {
    let claude_dir = dirs_claude()?;

    // 1. Remove cuz prompt from CLAUDE.md
    let claude_md = claude_dir.join("CLAUDE.md");
    if claude_md.exists() {
        let content = fs::read_to_string(&claude_md)?;
        if prompt::has_cuz_markers(&content) {
            let cleaned = prompt::remove_prompt(&content);
            fs::write(&claude_md, &cleaned)?;
            println!("  Removed cuz prompt from {}", claude_md.display());
        } else {
            println!("  {} has no cuz prompt", claude_md.display());
        }
    }

    // 2. Remove cuz hook from settings.json
    let settings_path = claude_dir.join("settings.json");
    if settings_path.exists() {
        remove_hook(&settings_path)?;
    }

    println!("\ncuz teardown complete. .cuz/ data left intact.");
    Ok(())
}

fn dirs_claude() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".claude"))
}

fn remove_hook(settings_path: &PathBuf) -> Result<()> {
    let content = fs::read_to_string(settings_path)?;
    let mut settings: serde_json::Value =
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

    let removed = if let Some(arr) = settings
        .pointer_mut("/hooks/PostToolUse")
        .and_then(|v| v.as_array_mut())
    {
        let before = arr.len();
        arr.retain(|entry| {
            let is_cuz = entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|hooks| {
                    hooks.iter().any(|hook| {
                        hook.get("command")
                            .and_then(|c| c.as_str())
                            .map(|c| c.contains("cuz verify"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            !is_cuz
        });
        arr.len() < before
    } else {
        false
    };

    if removed {
        fs::write(settings_path, serde_json::to_string_pretty(&settings)?)?;
        println!("  Removed cuz hook from {}", settings_path.display());
    } else {
        println!("  {} has no cuz hook", settings_path.display());
    }

    Ok(())
}
