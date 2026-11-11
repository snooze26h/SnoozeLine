use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
    runtime: PathBuf,
    cwd: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "snoozeline-backports-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed),
        ));
        let cwd = root.join("project");
        fs::create_dir_all(&cwd).unwrap();
        Self {
            runtime: root.join("runtime"),
            root,
            cwd,
        }
    }

    fn input(&self) -> Value {
        json!({
            "model": {"id": "claude-fable-5-1", "display_name": "Fable 5.1"},
            "effort": {"level": "max"},
            "workspace": {"current_dir": self.cwd},
            "context_window": {"context_window_size": 1000000, "used_percentage": 26,
                "current_usage": {"input_tokens": 256800, "output_tokens": 1000}},
            "rate_limits": {"five_hour": {"used_percentage": 8}, "seven_day": {"used_percentage": 24}},
            "output_style": {"name": "default"}
        })
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_snoozeline"));
        command
            .env("SNOOZELINE_HOME", &self.runtime)
            .env("SNOOZELINE_NATIVE_QUOTA_ONLY", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn run(&self, mut command: Command, bytes: &[u8]) -> Output {
        let mut child = command.spawn().unwrap();
        // 超长输入可能在父进程写完之前被拒绝；这时 BrokenPipe 是预期结果。
        let write = child.stdin.take().unwrap().write_all(bytes);
        if let Err(error) = write {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
        child.wait_with_output().unwrap()
    }

    fn render(&self, command: Command, input: &Value) -> String {
        let output = self.run(command, input.to_string().as_bytes());
        assert!(output.status.success(), "{:?}", output);
        assert!(output.stderr.is_empty(), "{:?}", output);
        plain(&output.stdout)
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=SnoozeLine Test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(&self.cwd)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        String::from_utf8(output.stdout).unwrap()
    }

    fn init_repository(&self) {
        self.git(&["init", "-q", "-b", "main"]);
        fs::write(self.cwd.join("tracked.txt"), "initial\n").unwrap();
        self.git(&["add", "tracked.txt"]);
        self.git(&["commit", "-qm", "fixture"]);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn plain(bytes: &[u8]) -> String {
    Regex::new(r"\x1b\[[0-9;]*m")
        .unwrap()
        .replace_all(&String::from_utf8_lossy(bytes), "")
        .into_owned()
}

#[test]
fn modern_timestamps_and_malformed_optional_fields_keep_the_line() {
    let fixture = Fixture::new();
    let mut input = fixture.input();
    input["rate_limits"]["five_hour"]["resets_at"] = json!(4102444800.25_f64);
    input["cost"] = json!({"total_duration_ms": "unavailable"});
    input["output_style"] = json!(42);
    let line = fixture.render(fixture.command(), &input);
    for expected in ["Fable 5.1 · max", "26% · 256.8k tokens", "5h 8% · 7d 24%"] {
        assert!(line.contains(expected), "{line}");
    }
    input["rate_limits"]["five_hour"] = json!("unavailable");
    let line = fixture.render(fixture.command(), &input);
    assert!(line.contains("5h - · 7d 24%"), "{line}");
    input["context_window"] = Value::Null;
    input["workspace"] = Value::Null;
    input["cwd"] = json!(fixture.cwd);
    let line = fixture.render(fixture.command(), &input);
    assert!(line.contains("- · - tokens"), "{line}");
    assert!(line.contains("project"), "{line}");
}

#[test]
fn preview_is_read_only_and_shows_the_current_quota_labels() {
    let fixture = Fixture::new();
    let mut command = fixture.command();
    command.args(["--preview", "--theme", "snooze26h"]);
    let line = fixture.render(command, &Value::Null);
    assert!(line.contains("Fable 5.1 · high"), "{line}");
    assert!(line.contains("26% · 256.8k tokens"), "{line}");
    assert!(line.contains("5h 8% · 7d 24%"), "{line}");
    assert!(!fixture.runtime.exists(), "preview created runtime files");
}

#[test]
fn invalid_or_oversized_input_does_not_echo_its_contents() {
    let fixture = Fixture::new();
    for input in [
        "{private-input-marker".to_string(),
        format!(
            "{{\"private-input-marker\":\"{}\"}}",
            "x".repeat(1024 * 1024)
        ),
    ] {
        let output = fixture.run(fixture.command(), input.as_bytes());
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-input-marker"));
    }
}

#[cfg(unix)]
fn script(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, contents).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn native_quota_only_and_empty_native_data_never_read_keychain() {
    let fixture = Fixture::new();
    let tools = fixture.root.join("tools");
    fs::create_dir(&tools).unwrap();
    let calls = fixture.root.join("security-calls");
    script(
        &tools.join("security"),
        "#!/bin/sh\nprintf called >> \"$SNOOZELINE_TEST_SECURITY_LOG\"\nprintf '{}'\n",
    );
    let mut input = fixture.input();
    input.as_object_mut().unwrap().remove("rate_limits");
    let command = || {
        let mut cmd = fixture.command();
        cmd.env("PATH", &tools)
            .env("SNOOZELINE_TEST_SECURITY_LOG", &calls);
        cmd
    };
    let line = fixture.render(command(), &input);
    assert!(!line.contains("5h"));
    for rate_limits in [
        Value::Null,
        json!({}),
        json!("unavailable"),
        json!({
            "five_hour": {"used_percentage": 99, "resets_at": 1}
        }),
    ] {
        input["rate_limits"] = rate_limits;
        let mut cmd = command();
        cmd.env_remove("SNOOZELINE_NATIVE_QUOTA_ONLY");
        let line = fixture.render(cmd, &input);
        assert!(!line.contains("5h"), "{line}");
    }
    assert!(!calls.exists(), "unexpected credential lookup");
}

#[cfg(unix)]
#[test]
fn git_cache_serves_old_status_while_one_background_refresh_runs() {
    use std::time::{Duration, Instant};
    let fixture = Fixture::new();
    fixture.init_repository();
    let tools = fixture.root.join("tools");
    fs::create_dir(&tools).unwrap();
    script(&tools.join("git"), "#!/bin/sh\nprintf 'call\\n' >> \"$SNOOZELINE_TEST_GIT_LOG\"\nif [ -n \"$SNOOZELINE_TEST_DELAY\" ]; then /bin/sleep 1; fi\nexec /usr/bin/git \"$@\"\n");
    let calls = fixture.root.join("git-calls");
    let command = |slow| {
        let mut cmd = fixture.command();
        cmd.env("PATH", &tools)
            .env("SNOOZELINE_TEST_GIT_LOG", &calls);
        if slow {
            cmd.env("SNOOZELINE_TEST_DELAY", "1");
        }
        cmd
    };
    let input = fixture.input();
    assert!(fixture.render(command(false), &input).contains("main ✓"));
    assert!(fixture.render(command(false), &input).contains("main ✓"));
    assert_eq!(fs::read_to_string(&calls).unwrap().lines().count(), 1);

    fs::write(fixture.cwd.join("tracked.txt"), "changed\n").unwrap();
    let cache_path = fs::read_dir(fixture.runtime.join("git-cache"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|ext| ext == "json"))
        .unwrap();
    let mut cached: Value = serde_json::from_slice(&fs::read(&cache_path).unwrap()).unwrap();
    cached["at_ms"] = json!(cached["at_ms"].as_u64().unwrap() - 2_500);
    fs::write(&cache_path, cached.to_string()).unwrap();

    // 后台 git 故意比前台的 500ms 截止时间更慢，前台仍必须立即提供旧状态。
    let old = fixture.render(command(true), &input);
    assert!(old.contains("main ✓"), "{old}");
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let line = fixture.render(command(true), &input);
        if line.contains("main ●") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "background status never refreshed: {line}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(fs::read_to_string(&calls).unwrap().lines().count(), 2);
}

#[test]
fn linked_worktrees_and_detached_head_keep_their_own_status() {
    let fixture = Fixture::new();
    fixture.init_repository();
    let linked = fixture.root.join("linked");
    fixture.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "linked-branch",
        linked.to_str().unwrap(),
    ]);
    let mut input = fixture.input();
    input["workspace"]["current_dir"] = json!(linked);
    assert!(fixture
        .render(fixture.command(), &input)
        .contains("linked-branch ✓"));
    let input = fixture.input();
    assert!(fixture.render(fixture.command(), &input).contains("main ✓"));
    fixture.git(&["checkout", "-q", "--detach"]);
    let mut config = snoozeline::config::Config::default();
    let git = config
        .segments
        .iter_mut()
        .find(|segment| segment.id == snoozeline::config::SegmentId::Git)
        .unwrap();
    git.options.insert("cache_ttl_ms".into(), json!(0));
    git.options.insert("show_sha".into(), json!(true));
    fs::write(
        fixture.runtime.join("config.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    let line = fixture.render(fixture.command(), &input);
    assert!(line.contains("detached ✓"), "{line}");
    assert!(
        line.contains(fixture.git(&["rev-parse", "--short=7", "HEAD"]).trim()),
        "{line}"
    );
}
