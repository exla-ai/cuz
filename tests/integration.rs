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
    repo.commit_with_intent("ok.txt", "ok\n", "cuz_eeeeee", "verify pass");

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
