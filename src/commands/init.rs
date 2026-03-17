use anyhow::Result;
use std::fs;

use crate::git;

pub fn run() -> Result<()> {
    let root = git::repo_root()?;
    let cuz = root.join(".cuz");
    let created = !cuz.exists();

    fs::create_dir_all(cuz.join("intents"))?;
    fs::create_dir_all(cuz.join("parents"))?;

    let schema_path = cuz.join("schema.json");
    if !schema_path.exists() {
        let schema = serde_json::json!({ "version": "0.1" });
        fs::write(&schema_path, serde_json::to_string_pretty(&schema)?)?;
    }

    if created {
        println!("Initialized .cuz/ in {}", root.display());
    } else {
        println!(".cuz/ already exists in {}", root.display());
    }
    Ok(())
}
