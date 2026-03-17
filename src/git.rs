use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Get the repository root via `git rev-parse --show-toplevel`.
pub fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to run git")?;
    if !output.status.success() {
        anyhow::bail!("Not inside a git repository");
    }
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(PathBuf::from(path))
}

/// Get the commit SHA that last touched a specific line via `git blame`.
pub fn blame_line(file: &str, line: u32) -> Result<String> {
    let output = Command::new("git")
        .args([
            "blame",
            "-L",
            &format!("{},{}", line, line),
            "--porcelain",
            file,
        ])
        .output()
        .context("Failed to run git blame")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git blame failed: {}", stderr);
    }
    let stdout = String::from_utf8(output.stdout)?;
    // First line of porcelain output is: <sha> <orig_line> <final_line> [<num_lines>]
    let sha = stdout
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().next())
        .ok_or_else(|| anyhow::anyhow!("Could not parse blame output"))?
        .to_string();
    Ok(sha)
}

/// Extract Intent: trailer value from a commit message.
pub fn extract_intent_from_commit(sha: &str) -> Result<Option<String>> {
    let msg = commit_message(sha)?;
    let intents = parse_intent_trailers(&msg);
    Ok(intents.into_iter().next())
}

/// Walk blame history to find the nearest commit with an Intent: trailer.
/// Returns (intent_id, commit_sha) if found.
pub fn find_nearest_intent(file: &str, line: u32, max_depth: u32) -> Result<Option<(String, String)>> {
    let mut sha = blame_line(file, line)?;

    // All zeros means uncommitted
    if sha.chars().all(|c| c == '0') {
        return Ok(None);
    }

    for _ in 0..max_depth {
        if let Some(intent_id) = extract_intent_from_commit(&sha)? {
            return Ok(Some((intent_id, sha.clone())));
        }
        // Move to parent commit
        let output = Command::new("git")
            .args(["log", "-1", "--format=%H", &format!("{}^", sha)])
            .output()
            .context("Failed to get parent commit")?;
        if !output.status.success() {
            break; // Reached root commit
        }
        let parent = String::from_utf8(output.stdout)?.trim().to_string();
        if parent.is_empty() {
            break;
        }
        sha = parent;
    }

    // Fallback: scan file history for any intent-bearing commit
    let output = Command::new("git")
        .args(["log", "--follow", "--format=%H", "--", file])
        .output()
        .context("Failed to run git log")?;
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        for commit_sha in stdout.lines() {
            let commit_sha = commit_sha.trim();
            if commit_sha.is_empty() {
                continue;
            }
            if let Some(intent_id) = extract_intent_from_commit(commit_sha)? {
                return Ok(Some((intent_id, commit_sha.to_string())));
            }
        }
    }

    Ok(None)
}

/// Get the SHA of the last commit.
pub fn last_commit_sha() -> Result<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%H"])
        .output()
        .context("Failed to run git log")?;
    if !output.status.success() {
        anyhow::bail!("No commits in repository");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Get the full commit message for a given SHA.
pub fn commit_message(sha: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%B", sha])
        .output()
        .context("Failed to run git log")?;
    if !output.status.success() {
        anyhow::bail!("Failed to get commit message for {}", sha);
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Get the commit subject (first line) for a given SHA.
pub fn commit_subject(sha: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%s", sha])
        .output()
        .context("Failed to run git log")?;
    if !output.status.success() {
        anyhow::bail!("Failed to get commit subject for {}", sha);
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Struct representing a commit with an intent trailer.
pub struct IntentCommit {
    pub sha: String,
    pub subject: String,
    pub intent_ids: Vec<String>,
}

/// Get commits that contain Intent: trailers.
pub fn commits_with_intents(limit: Option<u32>, all: bool) -> Result<Vec<IntentCommit>> {
    let mut args = vec![
        "log".to_string(),
        "--grep=Intent:".to_string(),
        "--format=%H|%s|%B<<<END>>>".to_string(),
    ];
    if let Some(n) = limit {
        if !all {
            args.push(format!("-{}", n));
        }
    }
    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to run git log")?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    let stdout = String::from_utf8(output.stdout)?;
    let mut results = Vec::new();

    for entry in stdout.split("<<<END>>>") {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        // Format: SHA|subject|full_body
        let mut parts = entry.splitn(3, '|');
        let sha = match parts.next() {
            Some(s) => s.trim().to_string(),
            None => continue,
        };
        let subject = match parts.next() {
            Some(s) => s.trim().to_string(),
            None => continue,
        };
        let body = parts.next().unwrap_or("").to_string();
        let intent_ids = parse_intent_trailers(&body);
        if !intent_ids.is_empty() {
            results.push(IntentCommit {
                sha,
                subject,
                intent_ids,
            });
        }
    }

    Ok(results)
}

/// Count commits since a given date.
pub fn commit_count_since(since: &str) -> Result<u32> {
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD", &format!("--since={}", since)])
        .output()
        .context("Failed to count commits")?;
    if !output.status.success() {
        return Ok(0);
    }
    let count = String::from_utf8(output.stdout)?.trim().parse().unwrap_or(0);
    Ok(count)
}

/// Count commits with Intent: trailers since a given date.
pub fn intent_commit_count_since(since: &str) -> Result<u32> {
    let output = Command::new("git")
        .args([
            "log",
            "--grep=Intent:",
            "--format=%H",
            &format!("--since={}", since),
        ])
        .output()
        .context("Failed to count intent commits")?;
    if !output.status.success() {
        return Ok(0);
    }
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.lines().filter(|l| !l.trim().is_empty()).count() as u32)
}

/// Parse `Intent: cuz_XXXXXX` trailers from a commit message.
/// Trailers appear in the last paragraph of the message (after a blank line).
pub fn parse_intent_trailers(message: &str) -> Vec<String> {
    let mut intents = Vec::new();
    // Trailers are in the last paragraph. Split on blank lines, take last block.
    let paragraphs: Vec<&str> = message.split("\n\n").collect();
    let trailer_block = paragraphs.last().unwrap_or(&"");

    for line in trailer_block.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("Intent:") {
            let value = value.trim();
            if !value.is_empty() {
                intents.push(value.to_string());
            }
        }
    }
    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_intent_trailers() {
        let msg = "Fix retry logic\n\nSwitched to exponential backoff.\n\nIntent: cuz_8f3a1b";
        let intents = parse_intent_trailers(msg);
        assert_eq!(intents, vec!["cuz_8f3a1b"]);
    }

    #[test]
    fn test_parse_multiple_trailers() {
        let msg = "Big refactor\n\nIntent: cuz_aaaaaa\nIntent: cuz_bbbbbb";
        let intents = parse_intent_trailers(msg);
        assert_eq!(intents, vec!["cuz_aaaaaa", "cuz_bbbbbb"]);
    }

    #[test]
    fn test_parse_no_trailers() {
        let msg = "Just a normal commit message";
        let intents = parse_intent_trailers(msg);
        assert!(intents.is_empty());
    }

    #[test]
    fn test_parse_intent_in_body_not_trailer() {
        // Intent mentioned in body but not as trailer (not in last paragraph)
        let msg = "Title\n\nIntent: cuz_111111\n\nSome other trailing text";
        let intents = parse_intent_trailers(msg);
        assert!(intents.is_empty());
    }
}
