use regex::Regex;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
    input: Value,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "snoozeline-effort-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self {
            input: json!({
                "model": {"id": "claude-fable-5-1", "display_name": "Fable 5.1"},
                "workspace": {"current_dir": root},
                "context_window": {
                    "context_window_size": 1_000_000,
                    "used_percentage": 26,
                    "current_usage": {"input_tokens": 256_800}
                },
                "rate_limits": {"five_hour": {}, "seven_day": {"used_percentage": 24}},
                "output_style": {"name": "default"}
            }),
            root,
        }
    }

    fn render(&self, theme: &str) -> String {
        let mut child = Command::new(env!("CARGO_BIN_EXE_snoozeline"))
            .args(["--theme", theme])
            .env("SNOOZELINE_HOME", &self.root)
            // 故意让启动环境与会话档位冲突，验证显示始终跟随本次原生输入。
            .env("CLAUDE_CODE_EFFORT_LEVEL", "max")
            .current_dir(&self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(self.input.to_string().as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert!(output.stderr.is_empty(), "{:?}", output);
        let text = String::from_utf8(output.stdout).unwrap();
        Regex::new(r"\x1b\[[0-9;]*m")
            .unwrap()
            .replace_all(&text, "")
            .into_owned()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn follows_native_effort_on_every_refresh() {
    let mut fixture = Fixture::new();
    for level in ["low", "medium", "high", "xhigh", "max", "low"] {
        fixture.input["effort"] = json!({"level": level});
        let line = fixture.render("snooze26h");
        assert!(line.contains(&format!("Fable 5.1 · {level}")), "{line}");
        assert!(line.contains("26% · 256.8k tokens"), "{line}");
        assert!(line.contains("5h - · 7d 24%"), "{line}");
    }

    fixture.input.as_object_mut().unwrap().remove("effort");
    let line = fixture.render("snooze26h");
    assert!(line.contains("Fable 5.1"), "{line}");
    assert!(!line.contains("Fable 5.1 ·"), "{line}");
}

#[test]
fn missing_or_malformed_effort_keeps_the_rest_of_the_line() {
    let mut fixture = Fixture::new();
    let original = fixture.render("snooze26h");
    for effort in [
        Value::Null,
        json!({}),
        json!({"level": null}),
        json!({"level": "unknown"}),
        json!({"level": "high\n\u{1b}[31m"}),
        json!({"level": "x".repeat(4096)}),
        json!({"level": 42}),
        json!({"level": true}),
        json!({"level": []}),
        json!({"level": {}}),
        json!("high"),
        json!(42),
        json!([]),
    ] {
        fixture.input["effort"] = effort;
        assert_eq!(fixture.render("snooze26h"), original);
    }
}

#[test]
fn effort_preserves_model_aliases_and_context_suffixes_across_styles() {
    let mut fixture = Fixture::new();
    std::fs::write(
        fixture.root.join("models.toml"),
        "[[models]]\npattern = \"claude-fable-5\"\ndisplay_name = \"Work Fable\"\ncontext_limit = 1000000\n",
    )
    .unwrap();
    fixture.input["model"]["id"] = json!("claude-fable-5-1[1m]");
    fixture.input["effort"] = json!({"level": "high"});
    for theme in ["snooze26h", "default", "powerline-dark"] {
        let line = fixture.render(theme);
        assert!(line.contains("Work Fable 1M · high"), "{line}");
    }
}
