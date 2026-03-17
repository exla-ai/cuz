use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::git;
use crate::prompt;

pub fn run() -> Result<()> {
    let claude_dir = dirs_claude()?;
    fs::create_dir_all(&claude_dir)?;

    // 1. Patch CLAUDE.md
    let claude_md = claude_dir.join("CLAUDE.md");
    let existing = if claude_md.exists() {
        fs::read_to_string(&claude_md)?
    } else {
        String::new()
    };

    let patched = prompt::inject_prompt(&existing);
    if patched != existing {
        fs::write(&claude_md, &patched)?;
        println!("  Patched {}", claude_md.display());
    } else {
        println!("  {} already up to date", claude_md.display());
    }

    // 2. Patch settings.json with PostToolUse hook
    let settings_path = claude_dir.join("settings.json");
    patch_settings(&settings_path)?;

    // 3. If inside a git repo, initialize .cuz/
    if let Ok(root) = git::repo_root() {
        init_cuz_dir(&root)?;
    } else {
        println!("  Not inside a git repo — skipping .cuz/ initialization");
    }

    println!("\ncuz setup complete!");
    Ok(())
}

fn dirs_claude() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".claude"))
}

fn patch_settings(settings_path: &PathBuf) -> Result<()> {
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(settings_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Navigate to hooks.PostToolUse, creating path as needed
    let hooks = settings
        .as_object_mut()
        .context("settings.json is not an object")?
        .entry("hooks")
        .or_insert(serde_json::json!({}));
    let post_tool_use = hooks
        .as_object_mut()
        .context("hooks is not an object")?
        .entry("PostToolUse")
        .or_insert(serde_json::json!([]));
    let arr = post_tool_use
        .as_array_mut()
        .context("PostToolUse is not an array")?;

    // Check if cuz hook already exists
    let has_cuz = arr.iter().any(|entry| {
        entry
            .get("matcher")
            .and_then(|m| m.as_str())
            .map(|m| m == "Bash")
            .unwrap_or(false)
            && entry
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
                .unwrap_or(false)
    });

    if has_cuz {
        println!("  {} already has cuz hook", settings_path.display());
    } else {
        let hook_entry = serde_json::json!({
            "matcher": "Bash",
            "hooks": [{"type": "command", "command": "cuz verify"}]
        });
        arr.push(hook_entry);
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(settings_path, content)?;
        println!("  Patched {}", settings_path.display());
    }

    Ok(())
}

fn init_cuz_dir(root: &std::path::Path) -> Result<()> {
    let cuz = root.join(".cuz");
    let created = !cuz.exists();

    fs::create_dir_all(cuz.join("intents"))?;
    fs::create_dir_all(cuz.join("parents"))?;

    let schema_path = cuz.join("schema.json");
    if !schema_path.exists() {
        let schema = serde_json::json!({
            "version": "0.1"
        });
        fs::write(&schema_path, serde_json::to_string_pretty(&schema)?)?;
    }

    if created {
        println!("  Created .cuz/ directory");
    } else {
        println!("  .cuz/ directory already exists");
    }
    Ok(())
}
