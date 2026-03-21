pub const CUZ_MARKER_START: &str = "<!-- cuz:start -->";
pub const CUZ_MARKER_END: &str = "<!-- cuz:end -->";

#[allow(dead_code)]
pub const CUZ_PROMPT: &str = r#"<!-- cuz:start -->
## cuz — intent tracking (REQUIRED)

IMPORTANT: You MUST create an intent record for EVERY commit. No exceptions. This is not optional.

### Before your first commit in any repo
If `.cuz/` does not exist, run `cuz init` to initialize intent tracking.

### On EVERY commit — do this BEFORE running git commit
1. Generate a random 6-hex-char ID (e.g. cuz_a3f1b2)
2. Create `.cuz/intents/cuz_XXXXXX.json` with this exact schema:
   {"id":"cuz_XXXXXX","version":"0.1","goal":<user's words — what they asked for>,
    "approach":<what you did and why>,
    "alternatives":[{"option":<other approach>,"rejected_because":<reason>}],
    "context_files":[<files you read>],"files_modified":[<files in this commit>],
    "confidence":<0-1>,"token_cost":null,"agent":"claude-code",
    "model":<your model name>,
    "parent_intent":<contents of .cuz/active_parent file, or null>,
    "timestamp":<ISO 8601>}
3. Stage the intent file: `git add .cuz/intents/cuz_XXXXXX.json`
4. Include the intent file in the same commit
5. Add `Intent: cuz_XXXXXX` as a git trailer in the commit message (after a blank line, at the end)
6. If `.cuz/active_parent` exists, read it and set parent_intent to its contents

### Before modifying existing code
If `.cuz/intents/` exists, check `git log` for `Intent:` trailers on files you're changing.
Read those intents to understand why code exists and what was already rejected.
If contradicting a previous intent, explain why in your new intent record.
<!-- cuz:end -->"#;

/// Check if content already contains cuz markers.
pub fn has_cuz_markers(content: &str) -> bool {
    content.contains(CUZ_MARKER_START) && content.contains(CUZ_MARKER_END)
}

/// Inject or replace the cuz prompt in the given content.
/// If markers already exist, replaces the content between them.
/// Otherwise, appends to the end.
#[allow(dead_code)]
pub fn inject_prompt(existing_content: &str) -> String {
    if has_cuz_markers(existing_content) {
        // Replace existing block
        let start_idx = existing_content.find(CUZ_MARKER_START).unwrap();
        let end_idx = existing_content.find(CUZ_MARKER_END).unwrap() + CUZ_MARKER_END.len();
        let before = &existing_content[..start_idx];
        let after = &existing_content[end_idx..];
        format!("{}{}{}", before.trim_end(), if before.is_empty() { "" } else { "\n\n" }, CUZ_PROMPT.to_string() + after)
    } else if existing_content.trim().is_empty() {
        CUZ_PROMPT.to_string()
    } else {
        format!("{}\n\n{}", existing_content.trim_end(), CUZ_PROMPT)
    }
}

/// Remove the cuz prompt from the given content.
pub fn remove_prompt(existing_content: &str) -> String {
    if !has_cuz_markers(existing_content) {
        return existing_content.to_string();
    }
    let start_idx = existing_content.find(CUZ_MARKER_START).unwrap();
    let end_idx = existing_content.find(CUZ_MARKER_END).unwrap() + CUZ_MARKER_END.len();
    let before = existing_content[..start_idx].trim_end();
    let after = existing_content[end_idx..].trim_start();
    if before.is_empty() && after.is_empty() {
        String::new()
    } else if before.is_empty() {
        after.to_string()
    } else if after.is_empty() {
        before.to_string()
    } else {
        format!("{}\n\n{}", before, after)
    }
}

/// Remove all cuz artifacts from the global `~/.claude/` directory.
/// Cleans up the legacy prompt from CLAUDE.md and the verify hook from settings.json.
pub fn cleanup_legacy_global() -> anyhow::Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Ok(()),
    };
    let global_claude_dir = PathBuf::from(home).join(".claude");

    // Remove cuz markers from global CLAUDE.md
    let claude_md = global_claude_dir.join("CLAUDE.md");
    if claude_md.exists() {
        let content = fs::read_to_string(&claude_md)?;
        if has_cuz_markers(&content) {
            let cleaned = remove_prompt(&content);
            fs::write(&claude_md, &cleaned)?;
            println!("  Cleaned up legacy cuz prompt from ~/.claude/CLAUDE.md");
        }
    }

    // Remove cuz hook from global settings.json
    let settings_path = global_claude_dir.join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        let mut settings: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
        if remove_cuz_hooks_from_global(&mut settings) {
            fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
            println!("  Cleaned up legacy cuz hook from ~/.claude/settings.json");
        }
    }

    Ok(())
}

/// Remove cuz verify hooks from a global settings.json Value. Returns true if anything was removed.
fn remove_cuz_hooks_from_global(settings: &mut serde_json::Value) -> bool {
    if let Some(arr) = settings
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_into_empty() {
        let result = inject_prompt("");
        assert!(result.starts_with(CUZ_MARKER_START));
        assert!(result.ends_with(CUZ_MARKER_END));
    }

    #[test]
    fn test_inject_appends() {
        let result = inject_prompt("# Existing content\n\nSome rules here.");
        assert!(result.starts_with("# Existing content"));
        assert!(result.contains(CUZ_MARKER_START));
        assert!(result.ends_with(CUZ_MARKER_END));
    }

    #[test]
    fn test_inject_idempotent() {
        let first = inject_prompt("# Header");
        let second = inject_prompt(&first);
        assert_eq!(first, second);
    }

    #[test]
    fn test_remove_prompt() {
        let injected = inject_prompt("# Header\n\nSome content.");
        let removed = remove_prompt(&injected);
        assert_eq!(removed, "# Header\n\nSome content.");
    }

    #[test]
    fn test_remove_from_empty_base() {
        let injected = inject_prompt("");
        let removed = remove_prompt(&injected);
        assert_eq!(removed, "");
    }

    #[test]
    fn test_has_markers() {
        assert!(has_cuz_markers(CUZ_PROMPT));
        assert!(!has_cuz_markers("no markers here"));
    }
}
