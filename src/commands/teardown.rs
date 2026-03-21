use anyhow::Result;
use std::fs;
use std::path::PathBuf;

use crate::git;
use crate::prompt;

pub fn run() -> Result<()> {
    // 1. Clean up global config
    cleanup_global()?;

    // 2. Clean up project-level config if in a repo
    if let Ok(root) = git::repo_root() {
        cleanup_project_config(&root)?;
    }

    // 3. Clean up old-style CLAUDE.md injection
    prompt::cleanup_legacy_global()?;

    println!("\ncuz teardown complete. .cuz/ data left intact.");
    Ok(())
}

fn cleanup_global() -> Result<()> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Ok(()),
    };
    let home = PathBuf::from(home);

    // Remove cuz from ~/.claude.json
    remove_cuz_from_mcp_json(&home.join(".claude.json"))?;

    // Remove cuz hooks from ~/.claude/settings.json
    let settings_path = home.join(".claude").join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let mut settings: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
        if remove_cuz_hooks(&mut settings) {
            fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
            println!("  Removed cuz hooks from ~/.claude/settings.json");
        }
    }

    Ok(())
}

fn cleanup_project_config(root: &std::path::Path) -> Result<()> {
    remove_cuz_from_mcp_json(&root.join(".mcp.json"))?;

    let rules_path = root.join(".claude").join("rules").join("cuz.md");
    if rules_path.exists() {
        fs::remove_file(&rules_path)?;
        println!("  Removed .claude/rules/cuz.md");
    }

    let settings_path = root.join(".claude").join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let mut settings: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
        if remove_cuz_hooks(&mut settings) {
            fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
            println!("  Removed cuz hooks from .claude/settings.json");
        }
    }

    Ok(())
}

fn remove_cuz_from_mcp_json(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let mut mcp: serde_json::Value =
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
    if let Some(servers) = mcp.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        if servers.remove("cuz").is_some() {
            if servers.is_empty() {
                fs::remove_file(path)?;
                println!("  Removed {} (no servers left)", path.display());
            } else {
                fs::write(path, serde_json::to_string_pretty(&mcp)?)?;
                println!("  Removed cuz from {}", path.display());
            }
        }
    }
    Ok(())
}

fn remove_cuz_hooks(settings: &mut serde_json::Value) -> bool {
    let mut removed = false;
    for event in &["PreToolUse", "PostToolUse", "Stop"] {
        let pointer = format!("/hooks/{}", event);
        if let Some(arr) = settings
            .pointer_mut(&pointer)
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
                                .map(|c| c.starts_with("cuz "))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                !is_cuz
            });
            if arr.len() < before {
                removed = true;
            }
        }
    }
    removed
}
