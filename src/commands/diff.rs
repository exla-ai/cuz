use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::process::Command;

use crate::git;
use crate::intent;

pub fn run(cached: bool) -> Result<()> {
    let files = changed_files(cached)?;

    if files.is_empty() {
        println!("{}", "No changed files.".dimmed());
        return Ok(());
    }

    // For each changed file, find intents
    let mut file_intents: HashMap<String, Vec<String>> = HashMap::new();

    for file in &files {
        if let Ok(Some((intent_id, _sha))) = git::find_nearest_intent(file, 1, 5) {
            file_intents
                .entry(intent_id)
                .or_default()
                .push(file.clone());
        }
    }

    if file_intents.is_empty() {
        println!(
            "{} changed file{}, none with intent tracking",
            files.len(),
            if files.len() == 1 { "" } else { "s" }
        );
        return Ok(());
    }

    println!(
        "{} changed file{}, {} with intent tracking\n",
        files.len(),
        if files.len() == 1 { "" } else { "s" },
        file_intents.values().map(|v| v.len()).sum::<usize>()
    );

    for (intent_id, touched_files) in &file_intents {
        let goal = intent::read_intent(intent_id)
            .map(|r| r.goal)
            .unwrap_or_else(|_| "unknown".to_string());
        println!("{} — {}", intent_id.cyan(), goal.bold());
        for f in touched_files {
            println!("  {}", f.dimmed());
        }
        println!();
    }

    // Show files with no intent
    let tracked: std::collections::HashSet<&String> =
        file_intents.values().flatten().collect();
    let untracked: Vec<&String> = files.iter().filter(|f| !tracked.contains(f)).collect();
    if !untracked.is_empty() {
        println!("{}", "No intent found:".yellow());
        for f in untracked {
            println!("  {}", f.dimmed());
        }
    }

    Ok(())
}

fn changed_files(cached: bool) -> Result<Vec<String>> {
    let mut args = vec!["diff", "--name-only"];
    if cached {
        args.push("--cached");
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to run git diff")?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect())
}
