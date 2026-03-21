use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::intent;

/// Run the MCP stdio server. Reads newline-delimited JSON-RPC from stdin,
/// writes JSON-RPC responses to stdout.
pub fn serve() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {"code": -32700, "message": "Parse error"}
                });
                writeln!(stdout, "{}", err_resp)?;
                stdout.flush()?;
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");

        let response = match method {
            "initialize" => handle_initialize(id),
            "notifications/initialized" => continue, // notification, no response
            "tools/list" => handle_tools_list(id),
            "tools/call" => handle_tools_call(id, &request),
            "ping" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {}
            }),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("Method not found: {}", method)}
            }),
        };

        writeln!(stdout, "{}", response)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_initialize(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "cuz",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

fn handle_tools_list(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "create_intent",
                    "description": "Create an intent record before committing. Returns the intent ID, file path, and git trailer to add to the commit message. Always call this before every git commit.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "goal": {
                                "type": "string",
                                "description": "What the user asked for (their words)"
                            },
                            "approach": {
                                "type": "string",
                                "description": "What you did and why"
                            },
                            "alternatives": {
                                "type": "array",
                                "description": "Other approaches considered and why they were rejected. Include at least one.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "option": {"type": "string", "description": "The alternative approach"},
                                        "rejected_because": {"type": "string", "description": "Why it was rejected"},
                                        "constraints": {"type": "string", "description": "Specific constraint that led to rejection (optional, helps future sessions check if constraint still holds)"}
                                    },
                                    "required": ["option", "rejected_because"]
                                }
                            },
                            "files_modified": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Files that will be in this commit"
                            },
                            "context_files": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Files you read to understand the problem"
                            },
                            "confidence": {
                                "type": "number",
                                "description": "Confidence level 0-1 (optional)"
                            }
                        },
                        "required": ["goal", "approach", "alternatives", "files_modified"]
                    }
                },
                {
                    "name": "get_rejected",
                    "description": "Get previously rejected alternatives for a file. Call this BEFORE editing any file to check what was already tried and rejected. Do NOT re-implement a rejected approach without explaining what changed.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "file": {
                                "type": "string",
                                "description": "File path to check (relative to repo root)"
                            }
                        },
                        "required": ["file"]
                    }
                },
                {
                    "name": "get_intent",
                    "description": "Read a specific intent record by ID.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Intent ID (e.g. cuz_abcdef)"
                            }
                        },
                        "required": ["id"]
                    }
                }
            ]
        }
    })
}

fn handle_tools_call(id: Value, request: &Value) -> Value {
    let params = request.get("params").cloned().unwrap_or(json!({}));
    let tool_name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "create_intent" => tool_create_intent(id, &arguments),
        "get_rejected" => tool_get_rejected(id, &arguments),
        "get_intent" => tool_get_intent(id, &arguments),
        _ => mcp_error(id, &format!("Unknown tool: {}", tool_name)),
    }
}

fn mcp_error(id: Value, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "isError": true,
            "content": [{"type": "text", "text": message}]
        }
    })
}

/// Extract a JSON array of strings into a Vec<String>.
fn get_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn tool_create_intent(id: Value, args: &Value) -> Value {
    let goal = args
        .get("goal")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let approach = args
        .get("approach")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let alternatives: Vec<intent::Alternative> = args
        .get("alternatives")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(intent::Alternative {
                        option: a.get("option")?.as_str()?.to_string(),
                        rejected_because: a.get("rejected_because")?.as_str()?.to_string(),
                        constraints: a
                            .get("constraints")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let files_modified = get_string_array(args, "files_modified");
    let context_files = get_string_array(args, "context_files");
    let confidence = args.get("confidence").and_then(|v| v.as_f64());

    match intent::create_intent(
        goal,
        approach,
        alternatives,
        files_modified,
        context_files,
        confidence,
    ) {
        Ok((intent_id, file_path)) => {
            let trailer = format!("Intent: {}", intent_id);
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{
                        "type": "text",
                        "text": format!(
                            "Created intent {}\nFile: {}\nTrailer: {}\n\nStage the intent file with: git add {}\nAdd this trailer as the last line of your commit message (after a blank line):\n\n{}",
                            intent_id,
                            file_path.display(),
                            trailer,
                            file_path.display(),
                            trailer
                        )
                    }]
                }
            })
        }
        Err(e) => mcp_error(id, &format!("Failed to create intent: {}", e)),
    }
}

fn tool_get_rejected(id: Value, args: &Value) -> Value {
    let file = args
        .get("file")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match intent::intents_for_file(file) {
        Ok(intents) => {
            let mut text = String::new();
            let mut count = 0;

            for record in &intents {
                for alt in &record.alternatives {
                    text.push_str(&intent::format_alternative(alt));
                    text.push_str(&format!(
                        "\n  (from intent {} — {})\n",
                        record.id, record.goal,
                    ));
                    if let Some(ref c) = alt.constraints {
                        text.push_str(&format!("  Constraint: {}\n", c));
                    }
                    text.push('\n');
                    count += 1;
                }
            }

            let content = if count == 0 {
                format!("No rejected alternatives found for {}", file)
            } else {
                format!(
                    "Rejected alternatives for {} ({} found):\n\n{}",
                    file, count, text
                )
            };

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": content}]
                }
            })
        }
        Err(e) => mcp_error(id, &format!("Failed to look up rejected alternatives: {}", e)),
    }
}

fn tool_get_intent(id: Value, args: &Value) -> Value {
    let intent_id = args
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match intent::read_intent(intent_id) {
        Ok(record) => {
            let json_str =
                serde_json::to_string_pretty(&record).unwrap_or_else(|_| "{}".to_string());
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": json_str}]
                }
            })
        }
        Err(e) => mcp_error(id, &format!("Intent not found: {}", e)),
    }
}
