use anyhow::{Context, Result};
use colored::Colorize;

use crate::git;
use crate::intent;

pub fn run(target: &str) -> Result<()> {
    let (file, line) = parse_target(target)?;

    // Find nearest intent
    let result = git::find_nearest_intent(&file, line, 10)?;

    let (intent_id, commit_sha) = match result {
        Some(r) => r,
        None => {
            println!(
                "{}",
                "No intent found for this line. This code predates cuz tracking."
                    .dimmed()
            );
            return Ok(());
        }
    };

    // Read the intent file
    let record = match intent::read_intent(&intent_id) {
        Ok(r) => r,
        Err(_) => {
            println!(
                "Intent {} referenced in commit {} but record not found.",
                intent_id.cyan(),
                &commit_sha[..8].dimmed()
            );
            return Ok(());
        }
    };

    // Pretty-print
    print_intent(&record, &commit_sha);
    Ok(())
}

fn parse_target(target: &str) -> Result<(String, u32)> {
    if let Some((file, line_str)) = target.rsplit_once(':') {
        let line: u32 = line_str
            .parse()
            .with_context(|| format!("Invalid line number: {}", line_str))?;
        Ok((file.to_string(), line))
    } else {
        Ok((target.to_string(), 1))
    }
}

fn print_intent(record: &intent::IntentRecord, commit_sha: &str) {
    println!("{} {}", "Intent:".dimmed(), record.id.cyan());
    println!("{} {}", "Commit:".dimmed(), &commit_sha[..8.min(commit_sha.len())].dimmed());
    println!("{} {}", "Goal:".dimmed(), record.goal.bold());
    println!("{} {}", "Approach:".dimmed(), record.approach);

    if !record.alternatives.is_empty() {
        println!("{}", "Alternatives considered:".dimmed());
        for alt in &record.alternatives {
            println!(
                "  {} {} — {}",
                "•".dimmed(),
                alt.option.yellow(),
                alt.rejected_because.dimmed()
            );
        }
    }

    if let Some(conf) = record.confidence {
        let conf_str = format!("{:.0}%", conf * 100.0);
        let colored = if conf >= 0.8 {
            conf_str.green()
        } else if conf >= 0.5 {
            conf_str.yellow()
        } else {
            conf_str.red()
        };
        println!("{} {}", "Confidence:".dimmed(), colored);
    }

    if let Some(cost) = record.token_cost {
        println!("{} {} tokens", "Token cost:".dimmed(), cost);
    }

    if let Some(ref parent) = record.parent_intent {
        println!("{} {}", "Parent:".dimmed(), parent.cyan());
    }

    println!("{} {}", "Date:".dimmed(), record.timestamp.dimmed());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_with_line() {
        let (file, line) = parse_target("src/main.rs:42").unwrap();
        assert_eq!(file, "src/main.rs");
        assert_eq!(line, 42);
    }

    #[test]
    fn test_parse_target_no_line() {
        let (file, line) = parse_target("src/main.rs").unwrap();
        assert_eq!(file, "src/main.rs");
        assert_eq!(line, 1);
    }

    #[test]
    fn test_parse_target_windows_path() {
        // Ensure colons in drive letters don't confuse parser
        // rsplit_once picks the last colon
        let (file, line) = parse_target("C:\\src\\main.rs:10").unwrap();
        assert_eq!(file, "C:\\src\\main.rs");
        assert_eq!(line, 10);
    }
}
