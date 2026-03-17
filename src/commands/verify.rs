use anyhow::Result;
use std::io::Read;

use crate::git;
use crate::intent;

pub fn run() -> Result<()> {
    // Read hook payload from stdin — never fail on bad input
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return Ok(());
    }

    // Parse JSON — bail silently on failure
    let payload: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    // Fast bail: only care about git commit commands
    let command = payload
        .pointer("/tool_input/command")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if !command.contains("git commit") {
        return Ok(());
    }

    // Check the last commit for Intent: trailer
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
        print_warning(
            "Last commit is missing an Intent: trailer. \
             Please amend the commit to add an `Intent: cuz_XXXXXX` trailer \
             and create the corresponding .cuz/intents/cuz_XXXXXX.json file.",
        );
        return Ok(());
    }

    // Check that referenced intent files exist
    for intent_id in &intents {
        if !intent::intent_exists(intent_id) {
            print_warning(&format!(
                "Commit references {} but .cuz/intents/{}.json was not found. \
                 Please create the intent file and amend the commit to include it.",
                intent_id, intent_id
            ));
        }
    }

    Ok(())
}

fn print_warning(message: &str) {
    let output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": format!("WARNING: {}", message)
        }
    });
    println!("{}", output);
}
