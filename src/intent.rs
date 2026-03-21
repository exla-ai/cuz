use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::git;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRecord {
    pub id: String,
    pub version: String,
    pub goal: String,
    pub approach: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<Alternative>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_modified: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_cost: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_intent: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub option: String,
    pub rejected_because: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentIntent {
    pub id: String,
    pub version: String,
    pub goal: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_intents: Vec<String>,
    pub timestamp: String,
}

/// Walk up from CWD to find the `.cuz/` directory (must be inside a git repo).
pub fn find_cuz_dir() -> Result<PathBuf> {
    let root = git::repo_root()?;
    let cuz_dir = root.join(".cuz");
    if cuz_dir.is_dir() {
        Ok(cuz_dir)
    } else {
        anyhow::bail!(".cuz/ directory not found. Run `cuz setup` first.")
    }
}

/// Find `.cuz/` or create it if we're in a git repo. Used by MCP tools
/// so the first `create_intent` call auto-initializes without manual setup.
pub fn find_or_init_cuz_dir() -> Result<PathBuf> {
    let root = git::repo_root()?;
    let cuz_dir = root.join(".cuz");
    if !cuz_dir.is_dir() {
        std::fs::create_dir_all(cuz_dir.join("intents"))?;
        std::fs::create_dir_all(cuz_dir.join("parents"))?;
        let schema_path = cuz_dir.join("schema.json");
        if !schema_path.exists() {
            std::fs::write(&schema_path, r#"{"version": "0.1"}"#)?;
        }
    }
    Ok(cuz_dir)
}

/// Read and deserialize an intent record from `.cuz/intents/{id}.json`.
pub fn read_intent(id: &str) -> Result<IntentRecord> {
    let cuz_dir = find_cuz_dir()?;
    let path = cuz_dir.join("intents").join(format!("{}.json", id));
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("Failed to read intent {}", id))?;
    let record: IntentRecord =
        serde_json::from_str(&content).with_context(|| format!("Failed to parse intent {}", id))?;
    Ok(record)
}

/// Read and deserialize a parent intent from `.cuz/parents/{id}.json`.
pub fn read_parent_intent(id: &str) -> Result<ParentIntent> {
    let cuz_dir = find_cuz_dir()?;
    let path = cuz_dir.join("parents").join(format!("{}.json", id));
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read parent intent {}", id))?;
    let record: ParentIntent = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse parent intent {}", id))?;
    Ok(record)
}

/// Check whether an intent file exists on disk.
pub fn intent_exists(id: &str) -> bool {
    find_cuz_dir()
        .map(|d| d.join("intents").join(format!("{}.json", id)).exists())
        .unwrap_or(false)
}

/// Read the active parent intent ID from `.cuz/active_parent`, if present.
pub fn read_active_parent() -> Result<Option<String>> {
    let cuz_dir = find_cuz_dir()?;
    let path = cuz_dir.join("active_parent");
    if path.exists() {
        let content = std::fs::read_to_string(&path)?.trim().to_string();
        if content.is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    } else {
        Ok(None)
    }
}

/// Validate that an intent ID matches `cuz_[a-f0-9]{6}`.
#[allow(dead_code)]
pub fn validate_intent_id(id: &str) -> bool {
    if !id.starts_with("cuz_") {
        return false;
    }
    let hex = &id[4..];
    hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// List all intent files in `.cuz/intents/`.
pub fn list_intents(cuz_dir: &Path) -> Result<Vec<String>> {
    let intents_dir = cuz_dir.join("intents");
    if !intents_dir.is_dir() {
        return Ok(vec![]);
    }
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(&intents_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(id) = name.strip_suffix(".json") {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

/// List all parent intent files in `.cuz/parents/`.
pub fn list_parent_intents(cuz_dir: &Path) -> Result<Vec<String>> {
    let parents_dir = cuz_dir.join("parents");
    if !parents_dir.is_dir() {
        return Ok(vec![]);
    }
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(&parents_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(id) = name.strip_suffix(".json") {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

/// Print a single rejected alternative with colored formatting.
/// `indent` is the leading whitespace (e.g. "  " or "    ").
pub fn print_alternative(alt: &Alternative, indent: &str) {
    use colored::Colorize;
    print!(
        "{}{} {} — {}",
        indent,
        "✗".red(),
        alt.option.yellow(),
        alt.rejected_because.dimmed()
    );
    if let Some(ref c) = alt.constraints {
        print!(" [{}]", c.dimmed());
    }
    println!();
}

/// Format a single rejected alternative as plain text (no color, for hooks/JSON output).
pub fn format_alternative(alt: &Alternative) -> String {
    let mut line = format!("✗ {} — {}", alt.option, alt.rejected_because);
    if let Some(ref c) = alt.constraints {
        line.push_str(&format!(" [{}]", c));
    }
    line
}

/// Find all intents that reference a given file in `files_modified`.
/// Returns intents sorted by timestamp (newest first).
pub fn intents_for_file(file: &str) -> Result<Vec<IntentRecord>> {
    let cuz_dir = find_cuz_dir()?;
    let ids = list_intents(&cuz_dir)?;
    let mut matches = Vec::new();
    for id in &ids {
        if let Ok(record) = read_intent(id) {
            if record.files_modified.iter().any(|f| f == file) {
                matches.push(record);
            }
        }
    }
    // Sort by timestamp descending (newest first)
    matches.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(matches)
}

/// Create a new intent record: generates ID, writes JSON, returns (id, file_path).
pub fn create_intent(
    goal: String,
    approach: String,
    alternatives: Vec<Alternative>,
    files_modified: Vec<String>,
    context_files: Vec<String>,
    confidence: Option<f64>,
) -> Result<(String, PathBuf)> {
    let cuz_dir = find_or_init_cuz_dir()?;
    let id = generate_intent_id();
    let parent = read_active_parent()?;
    let record = IntentRecord {
        id: id.clone(),
        version: "0.1".to_string(),
        goal,
        approach,
        alternatives,
        context_files,
        files_modified,
        confidence,
        token_cost: None,
        agent: Some("claude-code".to_string()),
        model: None,
        parent_intent: parent,
        timestamp: Utc::now().to_rfc3339(),
    };
    let path = cuz_dir.join("intents").join(format!("{}.json", id));
    let json = serde_json::to_string_pretty(&record)?;
    std::fs::write(&path, json)?;
    Ok((id, path))
}

/// Generate a random 6-hex-char intent ID.
fn generate_intent_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Mix in process ID for uniqueness across concurrent processes
    let mixed = nanos ^ (std::process::id() as u128);
    format!("cuz_{:06x}", (mixed & 0xffffff) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_intent_id() {
        assert!(validate_intent_id("cuz_8f3a1b"));
        assert!(validate_intent_id("cuz_000000"));
        assert!(validate_intent_id("cuz_abcdef"));
        assert!(!validate_intent_id("cuz_ABCDEF"));
        assert!(!validate_intent_id("cuz_8f3a1")); // too short
        assert!(!validate_intent_id("cuz_8f3a1b2")); // too long
        assert!(!validate_intent_id("abc_8f3a1b")); // wrong prefix
        assert!(!validate_intent_id("cuz_zzzzzz")); // not hex
    }

    #[test]
    fn test_intent_round_trip() {
        let record = IntentRecord {
            id: "cuz_8f3a1b".to_string(),
            version: "0.1".to_string(),
            goal: "fix retry logic".to_string(),
            approach: "exponential backoff".to_string(),
            alternatives: vec![Alternative {
                option: "circuit breaker".to_string(),
                rejected_because: "too complex".to_string(),
                constraints: Some("requires new dependency".to_string()),
            }],
            context_files: vec!["src/retry.ts".to_string()],
            files_modified: vec!["src/retry.ts".to_string()],
            confidence: Some(0.87),
            token_cost: Some(12847),
            agent: Some("claude-code".to_string()),
            model: Some("claude-sonnet-4-20250514".to_string()),
            parent_intent: None,
            timestamp: "2026-03-16T14:32:00Z".to_string(),
        };

        let json = serde_json::to_string_pretty(&record).unwrap();
        let parsed: IntentRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "cuz_8f3a1b");
        assert_eq!(parsed.alternatives.len(), 1);
        assert_eq!(parsed.confidence, Some(0.87));
    }

    #[test]
    fn test_intent_minimal() {
        let json = r#"{
            "id": "cuz_aabbcc",
            "version": "0.1",
            "goal": "test",
            "approach": "test approach",
            "timestamp": "2026-03-16T14:32:00Z"
        }"#;
        let record: IntentRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.id, "cuz_aabbcc");
        assert!(record.alternatives.is_empty());
        assert!(record.confidence.is_none());
        assert!(record.token_cost.is_none());
        assert!(record.parent_intent.is_none());
    }
}
