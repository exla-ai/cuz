use anyhow::Result;
use colored::Colorize;

use crate::intent;

pub fn run(json: bool) -> Result<()> {
    let cuz_dir = intent::find_cuz_dir()?;
    let ids = intent::list_intents(&cuz_dir)?;

    let mut total_tokens: u64 = 0;
    let mut tracked_count: u32 = 0;
    let mut untracked_count: u32 = 0;
    let mut by_model: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    for id in &ids {
        if let Ok(record) = intent::read_intent(id) {
            if let Some(cost) = record.token_cost {
                total_tokens += cost;
                tracked_count += 1;
                let model = record.model.unwrap_or_else(|| "unknown".to_string());
                *by_model.entry(model).or_default() += cost;
            } else {
                untracked_count += 1;
            }
        }
    }

    if json {
        let mut model_entries: Vec<serde_json::Value> = by_model
            .iter()
            .map(|(model, tokens)| {
                serde_json::json!({ "model": model, "tokens": tokens })
            })
            .collect();
        model_entries.sort_by(|a, b| {
            b["tokens"].as_u64().cmp(&a["tokens"].as_u64())
        });
        let output = serde_json::json!({
            "total_tokens": total_tokens,
            "tracked_intents": tracked_count,
            "untracked_intents": untracked_count,
            "by_model": model_entries,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("{}", "cuz cost".bold());
    println!();
    println!("  {} {} tokens", "Total:".dimmed(), format_tokens(total_tokens).bold());
    println!(
        "  {} {} intent{} with cost data, {} without",
        "Coverage:".dimmed(),
        tracked_count,
        if tracked_count == 1 { "" } else { "s" },
        untracked_count
    );

    if !by_model.is_empty() {
        println!();
        println!("  {}", "By model:".dimmed());
        let mut sorted: Vec<_> = by_model.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (model, tokens) in sorted {
            println!("    {} {}", format_tokens(*tokens), model.cyan());
        }
    }

    Ok(())
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
