use anyhow::Result;
use std::io::Read;

use crate::git;
use crate::intent;

/// Handle PreToolUse hook for Edit|Write — inject rejected alternatives as context.
pub fn run_pre_edit() -> Result<()> {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return Ok(());
    }

    let payload: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    // Extract file path from tool_input
    let file_path = payload
        .pointer("/tool_input/file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if file_path.is_empty() {
        return Ok(());
    }

    // Look up rejected alternatives for this file
    let intents = match intent::intents_for_file(file_path) {
        Ok(i) => i,
        Err(_) => return Ok(()),
    };

    let mut warnings = Vec::new();
    for record in &intents {
        for alt in &record.alternatives {
            warnings.push(intent::format_alternative(alt));
        }
    }

    if !warnings.is_empty() {
        let context = format!(
            "WARNING: Previously rejected for {}:\n{}",
            file_path,
            warnings.join("\n")
        );
        let output = serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "additionalContext": context
            }
        });
        println!("{}", output);
    }

    Ok(())
}

/// Handle Stop hook — check if the most recent commit has an Intent: trailer.
pub fn run_stop_check() -> Result<()> {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return Ok(());
    }

    let payload: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    // Avoid infinite loops: if stop_hook_active is set, bail
    if payload
        .get("stop_hook_active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Ok(());
    }

    // Check the most recent commit for Intent: trailer
    let sha = match git::last_commit_sha() {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let message = match git::commit_message(&sha) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };

    let intents = git::parse_intent_trailers(&message);

    if intents.is_empty() {
        let output = serde_json::json!({
            "decision": "block",
            "reason": "Last commit is missing an Intent: trailer. Call create_intent and amend the commit."
        });
        println!("{}", output);
    }

    Ok(())
}
