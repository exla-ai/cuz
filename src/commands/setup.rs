use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::git;
use crate::prompt;

pub fn run(project: bool) -> Result<()> {
    if project {
        run_project()
    } else {
        run_global()
    }
}

/// Global setup: configure ~/.claude.json (MCP) and ~/.claude/settings.json (hooks).
/// Works from anywhere — no git repo required.
fn run_global() -> Result<()> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let home = PathBuf::from(home);

    // 1. Patch ~/.claude.json with cuz MCP server
    patch_mcp_json(&home.join(".claude.json"))?;

    // 2. Patch ~/.claude/settings.json with hooks
    let claude_dir = home.join(".claude");
    fs::create_dir_all(&claude_dir)?;
    patch_settings(&claude_dir.join("settings.json"))?;

    // 3. Clean up old-style CLAUDE.md injection if present
    prompt::cleanup_legacy_global()?;

    // 4. If inside a git repo, also init .cuz/
    if let Ok(root) = git::repo_root() {
        init_cuz_dir(&root)?;
    }

    println!("\ncuz setup complete!");
    Ok(())
}

/// Project setup: configure .mcp.json, .claude/settings.json, .claude/rules/cuz.md
/// in the current repo. For teams that commit config to the repo.
fn run_project() -> Result<()> {
    let root = git::repo_root().context("--project requires a git repository")?;

    init_cuz_dir(&root)?;
    patch_mcp_json(&root.join(".mcp.json"))?;

    let claude_dir = root.join(".claude");
    fs::create_dir_all(&claude_dir)?;
    patch_settings(&claude_dir.join("settings.json"))?;
    create_project_rules(&root)?;

    println!("\ncuz setup complete! (project-level)");
    Ok(())
}

fn init_cuz_dir(root: &std::path::Path) -> Result<()> {
    let cuz = root.join(".cuz");
    let created = !cuz.exists();

    fs::create_dir_all(cuz.join("intents"))?;
    fs::create_dir_all(cuz.join("parents"))?;

    let schema_path = cuz.join("schema.json");
    if !schema_path.exists() {
        let schema = serde_json::json!({ "version": "0.1" });
        fs::write(&schema_path, serde_json::to_string_pretty(&schema)?)?;
    }

    if created {
        println!("  Created .cuz/ directory");
    } else {
        println!("  .cuz/ directory already exists");
    }
    Ok(())
}

fn patch_mcp_json(path: &std::path::Path) -> Result<()> {
    let mut mcp: serde_json::Value = if path.exists() {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let servers = mcp
        .as_object_mut()
        .context("MCP config is not an object")?
        .entry("mcpServers")
        .or_insert(serde_json::json!({}));

    if servers.get("cuz").is_some() {
        println!("  {} already has cuz server", path.display());
    } else {
        servers
            .as_object_mut()
            .context("mcpServers is not an object")?
            .insert(
                "cuz".to_string(),
                serde_json::json!({
                    "type": "stdio",
                    "command": "cuz",
                    "args": ["mcp", "serve"]
                }),
            );
        fs::write(path, serde_json::to_string_pretty(&mcp)?)?;
        println!("  Patched {}", path.display());
    }
    Ok(())
}

fn patch_settings(settings_path: &std::path::Path) -> Result<()> {
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(settings_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let hooks = settings
        .as_object_mut()
        .context("settings.json is not an object")?
        .entry("hooks")
        .or_insert(serde_json::json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .context("hooks is not an object")?;

    let mut changed = false;

    // PreToolUse: inject rejected alternatives before edits
    changed |= ensure_hook(hooks_obj, "PreToolUse", "Edit|Write", "cuz hook pre-edit")?;

    // PostToolUse: verify intent trailers after commits
    changed |= ensure_hook(hooks_obj, "PostToolUse", "Bash", "cuz verify")?;

    // Stop: block if last commit missing trailer
    changed |= ensure_hook_no_matcher(hooks_obj, "Stop", "cuz hook stop-check")?;

    if changed {
        fs::write(settings_path, serde_json::to_string_pretty(&settings)?)?;
        println!("  Patched {}", settings_path.display());
    } else {
        println!("  {} already has cuz hooks", settings_path.display());
    }
    Ok(())
}

fn ensure_hook(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    event: &str,
    matcher: &str,
    command: &str,
) -> Result<bool> {
    let arr = hooks_obj
        .entry(event)
        .or_insert(serde_json::json!([]))
        .as_array_mut()
        .context(format!("{} is not an array", event))?;

    if has_hook_command(arr, command) {
        return Ok(false);
    }
    arr.push(serde_json::json!({
        "matcher": matcher,
        "hooks": [{"type": "command", "command": command}]
    }));
    Ok(true)
}

fn ensure_hook_no_matcher(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    event: &str,
    command: &str,
) -> Result<bool> {
    let arr = hooks_obj
        .entry(event)
        .or_insert(serde_json::json!([]))
        .as_array_mut()
        .context(format!("{} is not an array", event))?;

    if has_hook_command(arr, command) {
        return Ok(false);
    }
    arr.push(serde_json::json!({
        "hooks": [{"type": "command", "command": command}]
    }));
    Ok(true)
}

fn has_hook_command(arr: &[serde_json::Value], command: &str) -> bool {
    arr.iter().any(|entry| {
        entry
            .get("hooks")
            .and_then(|h| h.as_array())
            .map(|hooks| {
                hooks.iter().any(|hook| {
                    hook.get("command")
                        .and_then(|c| c.as_str())
                        .map(|c| c.contains(command))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    })
}

fn create_project_rules(root: &std::path::Path) -> Result<()> {
    let rules_dir = root.join(".claude").join("rules");
    fs::create_dir_all(&rules_dir)?;
    let rules_path = rules_dir.join("cuz.md");

    let rules_content = r#"## cuz — intent tracking

Use the cuz MCP tools for every commit:
1. Before editing files, call `get_rejected` to check what was previously tried and rejected
2. Before committing, call `create_intent` with goal, approach, and alternatives
3. Add the returned trailer to the commit message
4. Stage the intent file with `git add .cuz/`

The alternatives field is the most important part. Always document at least one
alternative considered and why it was rejected. If an alternative was rejected
due to a specific constraint, include it so future sessions can check if it still holds.

Do NOT re-implement a previously rejected approach without explaining what changed.
"#;

    fs::write(&rules_path, rules_content)?;
    println!("  Created .claude/rules/cuz.md");
    Ok(())
}
