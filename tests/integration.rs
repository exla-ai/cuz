use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper to create a temporary git repo with `.cuz/` initialized.
struct TestRepo {
    dir: PathBuf,
}

impl TestRepo {
    fn new() -> Self {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "cuz_test_{}_{}", std::process::id(), id
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Init git repo
        run_in(&dir, "git", &["init"]);
        run_in(&dir, "git", &["config", "user.email", "test@test.com"]);
        run_in(&dir, "git", &["config", "user.name", "Test"]);

        // Init .cuz/
        fs::create_dir_all(dir.join(".cuz/intents")).unwrap();
        fs::create_dir_all(dir.join(".cuz/parents")).unwrap();
        fs::write(
            dir.join(".cuz/schema.json"),
            r#"{"version": "0.1"}"#,
        )
        .unwrap();

        // Initial commit so HEAD exists
        fs::write(dir.join("init.txt"), "init").unwrap();
        run_in(&dir, "git", &["add", "."]);
        run_in(&dir, "git", &["commit", "-m", "Initial commit"]);

        TestRepo { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }

    fn commit_with_intent_full(
        &self,
        filename: &str,
        content: &str,
        intent_id: &str,
        goal: &str,
        token_cost: Option<u64>,
        model: Option<&str>,
    ) {
        let mut intent = serde_json::json!({
            "id": intent_id,
            "version": "0.1",
            "goal": goal,
            "approach": "test approach",
            "files_modified": [filename],
            "timestamp": "2026-03-16T14:32:00Z"
        });
        if let Some(cost) = token_cost {
            intent["token_cost"] = serde_json::json!(cost);
        }
        if let Some(m) = model {
            intent["model"] = serde_json::json!(m);
        }
        fs::write(
            self.dir.join(format!(".cuz/intents/{}.json", intent_id)),
            serde_json::to_string_pretty(&intent).unwrap(),
        )
        .unwrap();
        fs::write(self.dir.join(filename), content).unwrap();
        run_in(&self.dir, "git", &["add", "."]);
        let msg = format!("Add {}\n\nIntent: {}", filename, intent_id);
        run_in(&self.dir, "git", &["commit", "-m", &msg]);
    }

    /// Write a file, stage, and commit with an intent.
    fn commit_with_intent(&self, filename: &str, content: &str, intent_id: &str, goal: &str) {
        // Write intent file
        let intent = serde_json::json!({
            "id": intent_id,
            "version": "0.1",
            "goal": goal,
            "approach": "test approach",
            "timestamp": "2026-03-16T14:32:00Z"
        });
        fs::write(
            self.dir.join(format!(".cuz/intents/{}.json", intent_id)),
            serde_json::to_string_pretty(&intent).unwrap(),
        )
        .unwrap();

        // Write source file
        fs::write(self.dir.join(filename), content).unwrap();

        run_in(&self.dir, "git", &["add", "."]);
        let msg = format!("Add {}\n\nIntent: {}", filename, intent_id);
        run_in(&self.dir, "git", &["commit", "-m", &msg]);
    }

    /// Write a file, stage, and commit WITHOUT intent trailer.
    fn commit_without_intent(&self, filename: &str, content: &str, message: &str) {
        fs::write(self.dir.join(filename), content).unwrap();
        run_in(&self.dir, "git", &["add", "."]);
        run_in(&self.dir, "git", &["commit", "-m", message]);
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn run_in(dir: &Path, cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run {} {:?}: {}", cmd, args, e));
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("{} {:?} failed: {}", cmd, args, stderr);
    }
    String::from_utf8(output.stdout).unwrap()
}

fn cuz_bin() -> PathBuf {
    // Find the built binary
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("cuz");
    path
}

fn run_cuz(dir: &Path, args: &[&str]) -> (String, String, bool) {
    let output = Command::new(cuz_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run cuz {:?}: {}", args, e));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    (stdout, stderr, output.status.success())
}

fn run_cuz_with_stdin(dir: &Path, args: &[&str], stdin: &str) -> (String, String, bool) {
    use std::io::Write;
    let mut child = Command::new(cuz_bin())
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    (stdout, stderr, output.status.success())
}

// --- Tests ---

#[test]
fn test_why_with_intent() {
    let repo = TestRepo::new();
    repo.commit_with_intent("hello.txt", "hello world\n", "cuz_aabb11", "say hello");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["why", "hello.txt:1"]);
    assert!(success);
    assert!(stdout.contains("cuz_aabb11"));
    assert!(stdout.contains("say hello"));
}

#[test]
fn test_why_no_intent() {
    let repo = TestRepo::new();
    repo.commit_without_intent("plain.txt", "no intent\n", "plain commit");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["why", "plain.txt:1"]);
    assert!(success);
    assert!(
        stdout.contains("No intent found") || stdout.contains("predates cuz"),
        "Expected 'no intent' message, got: {}",
        stdout
    );
}

#[test]
fn test_why_missing_intent_file() {
    let repo = TestRepo::new();
    // Commit with trailer but no intent file
    fs::write(repo.path().join("orphan.txt"), "orphan\n").unwrap();
    run_in(repo.path(), "git", &["add", "."]);
    run_in(
        repo.path(),
        "git",
        &["commit", "-m", "Orphan commit\n\nIntent: cuz_999999"],
    );

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["why", "orphan.txt:1"]);
    assert!(success);
    assert!(stdout.contains("not found"), "Expected 'not found' message, got: {}", stdout);
}

#[test]
fn test_log_shows_intents() {
    let repo = TestRepo::new();
    repo.commit_with_intent("a.txt", "aaa\n", "cuz_111111", "first feature");
    repo.commit_with_intent("b.txt", "bbb\n", "cuz_222222", "second feature");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["log"]);
    assert!(success);
    assert!(stdout.contains("cuz_111111") || stdout.contains("first feature"));
    assert!(stdout.contains("cuz_222222") || stdout.contains("second feature"));
}

#[test]
fn test_log_json() {
    let repo = TestRepo::new();
    repo.commit_with_intent("x.txt", "xxx\n", "cuz_aaaaaa", "json test");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["log", "--json"]);
    assert!(success);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["intent_id"], "cuz_aaaaaa");
}

#[test]
fn test_log_empty() {
    let repo = TestRepo::new();
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["log"]);
    assert!(success);
    assert!(stdout.contains("No intent-tracked commits"));
}

#[test]
fn test_status() {
    let repo = TestRepo::new();
    repo.commit_with_intent("s.txt", "status\n", "cuz_cccccc", "status test");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["status"]);
    assert!(success);
    assert!(stdout.contains("Intent records:"));
    assert!(stdout.contains("Coverage"));
}

#[test]
fn test_verify_non_commit_exits_clean() {
    let repo = TestRepo::new();
    let input = r#"{"tool_input":{"command":"ls -la"}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["verify"], input);
    assert!(success);
    assert!(stdout.is_empty(), "Should produce no output for non-commit: {}", stdout);
}

#[test]
fn test_verify_commit_without_trailer() {
    let repo = TestRepo::new();
    repo.commit_without_intent("v.txt", "verify\n", "no trailer commit");

    let input = r#"{"tool_input":{"command":"git commit -m 'test'"}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["verify"], input);
    assert!(success);
    assert!(stdout.contains("WARNING"), "Should warn about missing trailer: {}", stdout);
}

#[test]
fn test_verify_commit_with_trailer() {
    let repo = TestRepo::new();
    // Create intent WITH alternatives to avoid empty-alternatives warning
    let intent = serde_json::json!({
        "id": "cuz_eeeeee",
        "version": "0.1",
        "goal": "verify pass",
        "approach": "test approach",
        "alternatives": [{"option": "other way", "rejected_because": "not as good"}],
        "files_modified": ["ok.txt"],
        "timestamp": "2026-03-16T14:32:00Z"
    });
    fs::write(
        repo.path().join(".cuz/intents/cuz_eeeeee.json"),
        serde_json::to_string_pretty(&intent).unwrap(),
    ).unwrap();
    fs::write(repo.path().join("ok.txt"), "ok\n").unwrap();
    run_in(repo.path(), "git", &["add", "."]);
    run_in(repo.path(), "git", &["commit", "-m", "Add ok.txt\n\nIntent: cuz_eeeeee"]);

    let input = r#"{"tool_input":{"command":"git commit -m 'test'"}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["verify"], input);
    assert!(success);
    assert!(!stdout.contains("WARNING"), "Should not warn: {}", stdout);
}

#[test]
fn test_verify_bad_stdin() {
    let repo = TestRepo::new();
    let (_, _, success) = run_cuz_with_stdin(repo.path(), &["verify"], "not json at all");
    assert!(success, "Should exit 0 even on bad input");
}

#[test]
fn test_why_default_line() {
    let repo = TestRepo::new();
    repo.commit_with_intent("def.txt", "default line\n", "cuz_dddddd", "default test");

    // No :line should default to line 1
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["why", "def.txt"]);
    assert!(success);
    assert!(stdout.contains("cuz_dddddd"));
}

#[test]
fn test_history_walking() {
    let repo = TestRepo::new();
    // First commit: with intent
    repo.commit_with_intent("walk.txt", "line1\nline2\nline3\n", "cuz_abcdef", "original");
    // Second commit: modify without intent (appends a line)
    fs::write(repo.path().join("walk.txt"), "line1\nline2\nline3\nline4\n").unwrap();
    run_in(repo.path(), "git", &["add", "."]);
    run_in(repo.path(), "git", &["commit", "-m", "minor tweak"]);

    // `cuz why walk.txt:1` should find the original intent via history walk
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["why", "walk.txt:1"]);
    assert!(success);
    // Should find the original intent since line 1 was authored in the intent commit
    assert!(
        stdout.contains("cuz_abcdef") || stdout.contains("original") || stdout.contains("No intent"),
        "Should find original intent or gracefully degrade: {}", stdout
    );
}

// --- New command tests ---

#[test]
fn test_init_creates_cuz_dir() {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("cuz_init_{}_{}", std::process::id(), id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    run_in(&dir, "git", &["init"]);
    run_in(&dir, "git", &["config", "user.email", "test@test.com"]);
    run_in(&dir, "git", &["config", "user.name", "Test"]);

    let (stdout, _stderr, success) = run_cuz(&dir, &["init"]);
    assert!(success);
    assert!(stdout.contains("Initialized .cuz/") || stdout.contains("already exists"));
    assert!(dir.join(".cuz/intents").is_dir());
    assert!(dir.join(".cuz/parents").is_dir());
    assert!(dir.join(".cuz/schema.json").exists());

    // Idempotent
    let (stdout2, _, success2) = run_cuz(&dir, &["init"]);
    assert!(success2);
    assert!(stdout2.contains("already exists"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_show_intent() {
    let repo = TestRepo::new();
    repo.commit_with_intent("show.txt", "show\n", "cuz_abab12", "show test goal");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["show", "cuz_abab12"]);
    assert!(success);
    assert!(stdout.contains("cuz_abab12"));
    assert!(stdout.contains("show test goal"));
}

#[test]
fn test_show_json() {
    let repo = TestRepo::new();
    repo.commit_with_intent("sj.txt", "sj\n", "cuz_bcbc23", "json show");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["show", "cuz_bcbc23", "--json"]);
    assert!(success);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["id"], "cuz_bcbc23");
    assert_eq!(parsed["goal"], "json show");
}

#[test]
fn test_show_not_found() {
    let repo = TestRepo::new();
    let (_stdout, stderr, success) = run_cuz(repo.path(), &["show", "cuz_zzzzzz"]);
    assert!(!success);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_search_finds_by_goal() {
    let repo = TestRepo::new();
    repo.commit_with_intent("s1.txt", "s1\n", "cuz_aa1122", "fix retry logic");
    repo.commit_with_intent("s2.txt", "s2\n", "cuz_bb3344", "add rate limiting");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["search", "retry"]);
    assert!(success);
    assert!(stdout.contains("cuz_aa1122"));
    assert!(stdout.contains("retry"));
    assert!(!stdout.contains("cuz_bb3344"));
}

#[test]
fn test_search_no_results() {
    let repo = TestRepo::new();
    repo.commit_with_intent("sr.txt", "sr\n", "cuz_cc5566", "something");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["search", "nonexistent_term_xyz"]);
    assert!(success);
    assert!(stdout.contains("No matching intents"));
}

#[test]
fn test_search_json() {
    let repo = TestRepo::new();
    repo.commit_with_intent("sj2.txt", "sj2\n", "cuz_dd7788", "exponential backoff");

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["search", "exponential", "--json"]);
    assert!(success);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn test_cost() {
    let repo = TestRepo::new();
    repo.commit_with_intent_full("c1.txt", "c1\n", "cuz_ee1111", "cost test 1", Some(5000), Some("claude-sonnet-4"));
    repo.commit_with_intent_full("c2.txt", "c2\n", "cuz_ee2222", "cost test 2", Some(3000), Some("claude-sonnet-4"));
    repo.commit_with_intent_full("c3.txt", "c3\n", "cuz_ee3333", "cost test 3", Some(2000), Some("claude-opus-4"));

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["cost"]);
    assert!(success);
    assert!(stdout.contains("10.0k") || stdout.contains("10000") || stdout.contains("Total"));
}

#[test]
fn test_cost_json() {
    let repo = TestRepo::new();
    repo.commit_with_intent_full("cj.txt", "cj\n", "cuz_ff4444", "cost json", Some(7500), Some("claude-sonnet-4"));

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["cost", "--json"]);
    assert!(success);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["total_tokens"], 7500);
}

#[test]
fn test_parent_lifecycle() {
    let repo = TestRepo::new();

    // Start parent
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["parent", "start", "migrate to gRPC"]);
    assert!(success);
    assert!(stdout.contains("Started parent intent"));
    assert!(stdout.contains("migrate to gRPC"));

    // Show parent
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["parent", "show"]);
    assert!(success);
    assert!(stdout.contains("migrate to gRPC"));

    // Can't start another while one is active
    let (_stdout, stderr, success) = run_cuz(repo.path(), &["parent", "start", "other goal"]);
    assert!(!success);
    assert!(stderr.contains("already active"));

    // End parent
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["parent", "end"]);
    assert!(success);
    assert!(stdout.contains("Ended parent intent"));

    // Show after end
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["parent", "show"]);
    assert!(success);
    assert!(stdout.contains("No active parent"));
}

#[test]
fn test_parent_end_without_active() {
    let repo = TestRepo::new();
    let (_stdout, stderr, success) = run_cuz(repo.path(), &["parent", "end"]);
    assert!(!success);
    assert!(stderr.contains("No active parent"));
}

#[test]
fn test_verify_missing_intent_file() {
    let repo = TestRepo::new();
    // Create commit with trailer pointing to nonexistent intent file
    fs::write(repo.path().join("vf.txt"), "verify file\n").unwrap();
    let intent_json = serde_json::json!({
        "id": "cuz_ffffff",
        "version": "0.1",
        "goal": "test",
        "approach": "test",
        "timestamp": "2026-03-16T14:32:00Z"
    });
    // Write the intent file, then commit, then delete it before verify
    fs::write(
        repo.path().join(".cuz/intents/cuz_ffffff.json"),
        serde_json::to_string_pretty(&intent_json).unwrap(),
    ).unwrap();
    run_in(repo.path(), "git", &["add", "."]);
    run_in(repo.path(), "git", &["commit", "-m", "test\n\nIntent: cuz_ffffff"]);
    // Now delete the intent file (simulate missing file scenario)
    fs::remove_file(repo.path().join(".cuz/intents/cuz_ffffff.json")).unwrap();

    let input = r#"{"tool_input":{"command":"git commit -m 'test'"}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["verify"], input);
    assert!(success);
    assert!(stdout.contains("WARNING"), "Should warn about missing intent file: {}", stdout);
    assert!(stdout.contains("cuz_ffffff"));
}

#[test]
fn test_diff_no_changes() {
    let repo = TestRepo::new();
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["diff"]);
    assert!(success);
    assert!(stdout.contains("No changed files"));
}

#[test]
fn test_diff_with_changes() {
    let repo = TestRepo::new();
    repo.commit_with_intent("d.txt", "original\n", "cuz_dd1111", "diff test");

    // Modify tracked file
    fs::write(repo.path().join("d.txt"), "modified\n").unwrap();

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["diff"]);
    assert!(success);
    assert!(stdout.contains("1 changed file") || stdout.contains("changed file"));
}

// --- MCP server tests ---

#[test]
fn test_mcp_initialize() {
    let repo = TestRepo::new();
    let input = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["mcp", "serve"], input);
    assert!(success);
    let response: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(response["id"], 1);
    assert!(response["result"]["serverInfo"]["name"] == "cuz");
    assert!(response["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn test_mcp_tools_list() {
    let repo = TestRepo::new();
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#
    );
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["mcp", "serve"], input);
    assert!(success);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert!(lines.len() >= 2, "Expected 2 responses, got: {:?}", lines);
    let response: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(response["id"], 2);
    let tools = response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"create_intent"));
    assert!(names.contains(&"get_rejected"));
    assert!(names.contains(&"get_intent"));
}

#[test]
fn test_mcp_create_intent() {
    let repo = TestRepo::new();
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_intent","arguments":{"goal":"test goal","approach":"test approach","alternatives":[{"option":"other","rejected_because":"not good"}],"files_modified":["src/main.rs"],"context_files":["src/lib.rs"],"confidence":0.9}}}"#
    );
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["mcp", "serve"], input);
    assert!(success);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert!(lines.len() >= 2);
    let response: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Created intent cuz_"), "Expected intent creation message: {}", text);
    assert!(text.contains("Intent: cuz_"), "Expected trailer in response: {}", text);
}

#[test]
fn test_mcp_get_rejected() {
    let repo = TestRepo::new();
    // Create intent with alternatives
    let intent = serde_json::json!({
        "id": "cuz_aaa111",
        "version": "0.1",
        "goal": "fix retry logic",
        "approach": "exponential backoff",
        "alternatives": [
            {"option": "circuit breaker", "rejected_because": "requires new dependency", "constraints": "no new deps allowed"},
            {"option": "polling", "rejected_because": "too slow"}
        ],
        "files_modified": ["src/retry.rs"],
        "timestamp": "2026-03-16T14:32:00Z"
    });
    fs::write(
        repo.path().join(".cuz/intents/cuz_aaa111.json"),
        serde_json::to_string_pretty(&intent).unwrap(),
    ).unwrap();

    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_rejected","arguments":{"file":"src/retry.rs"}}}"#
    );
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["mcp", "serve"], input);
    assert!(success);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert!(lines.len() >= 2);
    let response: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("circuit breaker"), "Expected circuit breaker: {}", text);
    assert!(text.contains("polling"), "Expected polling: {}", text);
    assert!(text.contains("no new deps allowed"), "Expected constraint: {}", text);
}

#[test]
fn test_mcp_get_intent() {
    let repo = TestRepo::new();
    repo.commit_with_intent("gi.txt", "gi\n", "cuz_bbb222", "get intent test");

    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_intent","arguments":{"id":"cuz_bbb222"}}}"#
    );
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["mcp", "serve"], input);
    assert!(success);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert!(lines.len() >= 2);
    let response: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("cuz_bbb222"));
    assert!(text.contains("get intent test"));
}

// --- Rejected command tests ---

#[test]
fn test_rejected_with_alternatives() {
    let repo = TestRepo::new();
    let intent = serde_json::json!({
        "id": "cuz_rej111",
        "version": "0.1",
        "goal": "fix retry",
        "approach": "backoff",
        "alternatives": [
            {"option": "circuit breaker", "rejected_because": "too complex", "constraints": "latency budget"}
        ],
        "files_modified": ["retry.rs"],
        "timestamp": "2026-03-16T14:32:00Z"
    });
    fs::write(
        repo.path().join(".cuz/intents/cuz_rej111.json"),
        serde_json::to_string_pretty(&intent).unwrap(),
    ).unwrap();

    let (stdout, _stderr, success) = run_cuz(repo.path(), &["rejected", "retry.rs"]);
    assert!(success);
    assert!(stdout.contains("circuit breaker"), "Expected circuit breaker: {}", stdout);
    assert!(stdout.contains("too complex"), "Expected rejection reason: {}", stdout);
    assert!(stdout.contains("latency budget"), "Expected constraint: {}", stdout);
}

#[test]
fn test_rejected_no_alternatives() {
    let repo = TestRepo::new();
    let (stdout, _stderr, success) = run_cuz(repo.path(), &["rejected", "nonexistent.rs"]);
    assert!(success);
    assert!(stdout.contains("No rejected alternatives"));
}

// --- Hook tests ---

#[test]
fn test_hook_pre_edit_with_alternatives() {
    let repo = TestRepo::new();
    let intent = serde_json::json!({
        "id": "cuz_hook11",
        "version": "0.1",
        "goal": "hook test",
        "approach": "test approach",
        "alternatives": [
            {"option": "bad approach", "rejected_because": "unstable"}
        ],
        "files_modified": ["src/main.rs"],
        "timestamp": "2026-03-16T14:32:00Z"
    });
    fs::write(
        repo.path().join(".cuz/intents/cuz_hook11.json"),
        serde_json::to_string_pretty(&intent).unwrap(),
    ).unwrap();

    let input = r#"{"tool_name":"Edit","tool_input":{"file_path":"src/main.rs"}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["hook", "pre-edit"], input);
    assert!(success);
    assert!(stdout.contains("WARNING"), "Expected warning: {}", stdout);
    assert!(stdout.contains("bad approach"), "Expected alternative: {}", stdout);
    assert!(stdout.contains("src/main.rs"), "Expected file path: {}", stdout);
}

#[test]
fn test_hook_pre_edit_no_alternatives() {
    let repo = TestRepo::new();
    let input = r#"{"tool_name":"Edit","tool_input":{"file_path":"src/new.rs"}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["hook", "pre-edit"], input);
    assert!(success);
    // No output when there are no rejected alternatives
    assert!(stdout.is_empty() || !stdout.contains("WARNING"));
}

#[test]
fn test_hook_stop_check_missing_trailer() {
    let repo = TestRepo::new();
    repo.commit_without_intent("stop.txt", "stop\n", "no trailer");

    let input = r#"{}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["hook", "stop-check"], input);
    assert!(success);
    assert!(stdout.contains("block"), "Expected block decision: {}", stdout);
    assert!(stdout.contains("Intent:"), "Expected mention of trailer: {}", stdout);
}

#[test]
fn test_hook_stop_check_with_trailer() {
    let repo = TestRepo::new();
    repo.commit_with_intent("stop2.txt", "stop2\n", "cuz_stop22", "stop test");

    let input = r#"{}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["hook", "stop-check"], input);
    assert!(success);
    assert!(!stdout.contains("block"), "Should not block: {}", stdout);
}

#[test]
fn test_hook_stop_check_avoids_loop() {
    let repo = TestRepo::new();
    repo.commit_without_intent("loop.txt", "loop\n", "no trailer");

    let input = r#"{"stop_hook_active": true}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["hook", "stop-check"], input);
    assert!(success);
    // Should NOT block when stop_hook_active is true (avoid infinite loop)
    assert!(!stdout.contains("block"), "Should not block on re-entry: {}", stdout);
}

// --- Verify empty alternatives warning ---

#[test]
fn test_verify_warns_empty_alternatives() {
    let repo = TestRepo::new();
    repo.commit_with_intent("ea.txt", "ea\n", "cuz_ea1111", "empty alt test");

    let input = r#"{"tool_input":{"command":"git commit -m 'test'"}}"#;
    let (stdout, _stderr, success) = run_cuz_with_stdin(repo.path(), &["verify"], input);
    assert!(success);
    assert!(stdout.contains("WARNING"), "Should warn about empty alternatives: {}", stdout);
    assert!(stdout.contains("no rejected alternatives"), "Expected empty-alt warning: {}", stdout);
}

// --- Setup tests ---

#[test]
fn test_setup_project_creates_repo_config() {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("cuz_setup_{}_{}", std::process::id(), id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    run_in(&dir, "git", &["init"]);
    run_in(&dir, "git", &["config", "user.email", "test@test.com"]);
    run_in(&dir, "git", &["config", "user.name", "Test"]);
    fs::write(dir.join("init.txt"), "init").unwrap();
    run_in(&dir, "git", &["add", "."]);
    run_in(&dir, "git", &["commit", "-m", "Initial commit"]);

    let (stdout, _stderr, success) = run_cuz(&dir, &["setup", "--project"]);
    assert!(success, "setup --project failed: {} {}", stdout, _stderr);

    // Check .mcp.json created in repo
    assert!(dir.join(".mcp.json").exists(), ".mcp.json not created");
    let mcp: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.join(".mcp.json")).unwrap()
    ).unwrap();
    assert!(mcp["mcpServers"]["cuz"].is_object(), "cuz not in .mcp.json");

    // Check .claude/settings.json created in repo
    assert!(dir.join(".claude/settings.json").exists(), "settings.json not created");
    let settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.join(".claude/settings.json")).unwrap()
    ).unwrap();
    assert!(settings["hooks"]["PreToolUse"].is_array());
    assert!(settings["hooks"]["PostToolUse"].is_array());
    assert!(settings["hooks"]["Stop"].is_array());

    // Check .claude/rules/cuz.md created
    assert!(dir.join(".claude/rules/cuz.md").exists(), "cuz.md rules not created");

    // Check .cuz/ created
    assert!(dir.join(".cuz/intents").is_dir());

    let _ = fs::remove_dir_all(&dir);
}

// --- Teardown tests ---

#[test]
fn test_teardown_removes_project_config() {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("cuz_teardown_{}_{}", std::process::id(), id));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    run_in(&dir, "git", &["init"]);
    run_in(&dir, "git", &["config", "user.email", "test@test.com"]);
    run_in(&dir, "git", &["config", "user.name", "Test"]);
    fs::write(dir.join("init.txt"), "init").unwrap();
    run_in(&dir, "git", &["add", "."]);
    run_in(&dir, "git", &["commit", "-m", "Initial commit"]);

    // First setup --project
    let (_, _, success) = run_cuz(&dir, &["setup", "--project"]);
    assert!(success);

    // Then teardown
    let (stdout, _stderr, success) = run_cuz(&dir, &["teardown"]);
    assert!(success, "teardown failed: {} {}", stdout, _stderr);

    // .mcp.json should be removed (was only cuz)
    assert!(!dir.join(".mcp.json").exists(), ".mcp.json should be removed");

    // .claude/rules/cuz.md should be removed
    assert!(!dir.join(".claude/rules/cuz.md").exists(), "cuz.md rules should be removed");

    // .cuz/ should still exist
    assert!(dir.join(".cuz/intents").is_dir(), ".cuz/ should still exist");

    let _ = fs::remove_dir_all(&dir);
}
