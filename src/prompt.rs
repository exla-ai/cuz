pub const CUZ_MARKER_START: &str = "<!-- cuz:start -->";
pub const CUZ_MARKER_END: &str = "<!-- cuz:end -->";

pub const CUZ_PROMPT: &str = r#"<!-- cuz:start -->
## cuz — intent tracking

### On every commit
1. Create `.cuz/intents/cuz_XXXXXX.json` (6 random hex chars) with:
   {"id","version":"0.1","goal":<user's words>,"approach":<what+why>,
    "alternatives":[{"option","rejected_because"}],
    "context_files","files_modified","confidence":0-1,
    "token_cost","agent":"claude-code","model",
    "parent_intent":<from .cuz/active_parent or null>,"timestamp"}
2. Stage `.cuz/` files in the commit
3. Add `Intent: cuz_XXXXXX` as a trailer (after blank line) in commit message
4. If `.cuz/active_parent` exists, set parent_intent to its contents

### Before modifying existing code
If `.cuz/intents/` exists, check git log for Intent: trailers on files you're changing.
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
