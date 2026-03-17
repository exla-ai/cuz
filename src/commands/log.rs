use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;

use crate::git;
use crate::intent;

pub fn run(limit: u32, all: bool, json: bool) -> Result<()> {
    let commits = git::commits_with_intents(Some(limit), all)?;

    if commits.is_empty() {
        println!("{}", "No intent-tracked commits found.".dimmed());
        return Ok(());
    }

    // Group commits by intent ID
    let mut intent_commits: HashMap<String, Vec<&git::IntentCommit>> = HashMap::new();
    let mut intent_order: Vec<String> = Vec::new();

    for commit in &commits {
        for intent_id in &commit.intent_ids {
            intent_commits
                .entry(intent_id.clone())
                .or_default()
                .push(commit);
            if !intent_order.contains(intent_id) {
                intent_order.push(intent_id.clone());
            }
        }
    }

    if json {
        return print_json(&intent_order, &intent_commits);
    }

    // Check for parent intents
    let active_parent = intent::read_active_parent().ok().flatten();

    // Try to group under parents
    let cuz_dir = intent::find_cuz_dir().ok();
    let parent_ids = cuz_dir
        .as_ref()
        .and_then(|d| intent::list_parent_intents(d).ok())
        .unwrap_or_default();

    // Build parent → children mapping
    let mut parent_children: HashMap<String, Vec<String>> = HashMap::new();
    let mut orphan_intents: Vec<String> = Vec::new();

    for parent_id in &parent_ids {
        if let Ok(parent) = intent::read_parent_intent(parent_id) {
            for child in &parent.child_intents {
                parent_children
                    .entry(parent_id.clone())
                    .or_default()
                    .push(child.clone());
            }
        }
    }

    // Categorize intents
    let mut claimed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for children in parent_children.values() {
        for child in children {
            claimed.insert(child.clone());
        }
    }
    for id in &intent_order {
        if !claimed.contains(id) {
            orphan_intents.push(id.clone());
        }
    }

    // Print parent groups
    for parent_id in &parent_ids {
        if let Ok(parent) = intent::read_parent_intent(parent_id) {
            let active_marker = if active_parent.as_deref() == Some(parent_id) {
                " (active)".green().to_string()
            } else {
                String::new()
            };
            println!(
                "{} — {}{}",
                parent_id.cyan(),
                parent.goal.bold(),
                active_marker
            );

            let children = parent_children.get(parent_id).cloned().unwrap_or_default();
            for (i, child_id) in children.iter().enumerate() {
                let is_last = i == children.len() - 1;
                let prefix = if is_last { "└── " } else { "├── " };
                let commit_count = intent_commits
                    .get(child_id)
                    .map(|c| c.len())
                    .unwrap_or(0);
                let goal = intent::read_intent(child_id)
                    .map(|r| r.goal)
                    .unwrap_or_else(|_| "unknown".to_string());
                println!(
                    "{}{} — {} ({} commit{})",
                    prefix.dimmed(),
                    child_id.cyan(),
                    goal,
                    commit_count,
                    if commit_count == 1 { "" } else { "s" }
                );
            }
            println!();
        }
    }

    // Print orphan intents (not under any parent)
    for intent_id in &orphan_intents {
        if !intent_order.contains(intent_id) {
            continue;
        }
        let commit_count = intent_commits
            .get(intent_id)
            .map(|c| c.len())
            .unwrap_or(0);
        let goal = intent::read_intent(intent_id)
            .map(|r| r.goal)
            .unwrap_or_else(|_| "unknown".to_string());
        println!(
            "{} — {} ({} commit{})",
            intent_id.cyan(),
            goal,
            commit_count,
            if commit_count == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

fn print_json(
    intent_order: &[String],
    intent_commits: &HashMap<String, Vec<&git::IntentCommit>>,
) -> Result<()> {
    let mut entries = Vec::new();
    for id in intent_order {
        let commits: Vec<serde_json::Value> = intent_commits
            .get(id)
            .map(|cs| {
                cs.iter()
                    .map(|c| {
                        serde_json::json!({
                            "sha": c.sha,
                            "subject": c.subject,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let goal = intent::read_intent(id)
            .map(|r| r.goal)
            .unwrap_or_else(|_| "unknown".to_string());

        entries.push(serde_json::json!({
            "intent_id": id,
            "goal": goal,
            "commits": commits,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}
