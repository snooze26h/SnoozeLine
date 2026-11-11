use super::{sanitize_text, Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_millis(500);
const GIT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const MAX_GIT_STDOUT_BYTES: u64 = 1024 * 1024;

fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    working_dir: &str,
    timeout: Duration,
) -> Option<Vec<u8>> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut output = Vec::new();
        let result = stdout
            .take(MAX_GIT_STDOUT_BYTES + 1)
            .read_to_end(&mut output)
            .map(|_| output);
        let _ = sender.send(result);
    });

    let deadline = Instant::now().checked_add(timeout)?;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                thread::sleep(remaining.min(GIT_POLL_INTERVAL));
            }
            Ok(None) | Err(_) => {
                if child.kill().is_ok() {
                    let _ = child.wait();
                }
                return None;
            }
        }
    };

    if !status.success() {
        return None;
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    let output = receiver.recv_timeout(remaining).ok()?.ok()?;
    (output.len() as u64 <= MAX_GIT_STDOUT_BYTES).then_some(output)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub status: GitStatus,
    pub ahead: u32,
    pub behind: u32,
    pub sha: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GitStatus {
    Clean,
    Dirty,
    Conflicts,
}

pub struct GitSegment {
    show_sha: bool,
    cache_ttl_ms: u64,
}

impl Default for GitSegment {
    fn default() -> Self {
        Self::new()
    }
}

impl GitSegment {
    pub fn new() -> Self {
        Self {
            show_sha: false,
            cache_ttl_ms: 2_000,
        }
    }

    pub fn with_sha(mut self, show_sha: bool) -> Self {
        self.show_sha = show_sha;
        self
    }

    pub fn with_cache_ttl(mut self, milliseconds: u64) -> Self {
        self.cache_ttl_ms = milliseconds.min(60_000);
        self
    }

    fn get_git_info(&self, working_dir: &str) -> Option<GitInfo> {
        super::git_cache::status(Path::new(working_dir), self.cache_ttl_ms)
    }

    pub(super) fn in_repository(directory: &Path) -> bool {
        std::env::var_os("GIT_DIR").is_some()
            || directory
                .ancestors()
                .any(|parent| parent.join(".git").exists())
    }

    pub(super) fn read_git_info(directory: &Path, timeout: Duration) -> Option<GitInfo> {
        // v2 一次返回分支、状态、上下游差距和 SHA，替代原来的五到六次进程调用。
        let output = run_command_with_timeout(
            "git",
            &[
                "--no-optional-locks",
                "status",
                "--porcelain=v2",
                "--branch",
                "-z",
            ],
            directory.to_str()?,
            timeout,
        )?;
        Self::parse_porcelain(&output)
    }

    pub(super) fn render_timeout() -> Duration {
        GIT_COMMAND_TIMEOUT
    }

    fn parse_porcelain(output: &[u8]) -> Option<GitInfo> {
        if !output.ends_with(&[0]) {
            return None;
        }
        let mut info = GitInfo {
            branch: String::new(),
            status: GitStatus::Clean,
            ahead: 0,
            behind: 0,
            sha: None,
        };
        let mut records = output.split(|byte| *byte == 0);
        while let Some(record) = records.next() {
            if let Some(name) = record.strip_prefix(b"# branch.head ") {
                info.branch = match name {
                    b"(detached)" => "detached".to_string(),
                    _ => String::from_utf8_lossy(name).into_owned(),
                };
            } else if let Some(oid) = record.strip_prefix(b"# branch.oid ") {
                if oid.len() >= 7 && oid.iter().all(u8::is_ascii_hexdigit) {
                    info.sha = Some(String::from_utf8_lossy(&oid[..7]).into_owned());
                }
            } else if let Some(ab) = record.strip_prefix(b"# branch.ab ") {
                for part in String::from_utf8_lossy(ab).split_whitespace() {
                    if let Some(ahead) = part.strip_prefix('+') {
                        info.ahead = ahead.parse().ok()?;
                    } else if let Some(behind) = part.strip_prefix('-') {
                        info.behind = behind.parse().ok()?;
                    }
                }
            } else if record.starts_with(b"u ") {
                info.status = GitStatus::Conflicts;
            } else if matches!(record.first(), Some(b'1' | b'2' | b'?')) {
                if info.status != GitStatus::Conflicts {
                    info.status = GitStatus::Dirty;
                }
                // -z 的重命名记录有第二个路径；文件名不能被当成状态或分支头解析。
                if record.starts_with(b"2 ") && records.next()?.is_empty() {
                    return None;
                }
            }
        }
        (!info.branch.is_empty()).then_some(info)
    }
}

impl Segment for GitSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let git_info = self.get_git_info(input.current_dir())?;

        let mut metadata = HashMap::new();
        metadata.insert("branch".to_string(), git_info.branch.clone());
        metadata.insert("status".to_string(), format!("{:?}", git_info.status));
        metadata.insert("ahead".to_string(), git_info.ahead.to_string());
        metadata.insert("behind".to_string(), git_info.behind.to_string());

        if let Some(sha) = git_info.sha.as_ref().filter(|_| self.show_sha) {
            metadata.insert("sha".to_string(), sha.clone());
        }

        let primary = sanitize_text(&git_info.branch);
        let mut status_parts = Vec::new();

        match git_info.status {
            GitStatus::Clean => status_parts.push("✓".to_string()),
            GitStatus::Dirty => status_parts.push("●".to_string()),
            GitStatus::Conflicts => status_parts.push("⚠".to_string()),
        }

        if git_info.ahead > 0 {
            status_parts.push(format!("↑{}", git_info.ahead));
        }
        if git_info.behind > 0 {
            status_parts.push(format!("↓{}", git_info.behind));
        }

        if let Some(sha) = git_info.sha.as_ref().filter(|_| self.show_sha) {
            status_parts.push(sha.clone());
        }

        Some(SegmentData {
            primary,
            secondary: status_parts.join(" "),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Git
    }
}

#[cfg(test)]
mod tests {
    use super::{run_command_with_timeout, GitSegment, GitStatus};
    use std::time::{Duration, Instant};

    #[test]
    fn reads_branch_sha_and_divergence_from_one_status() {
        let info = GitSegment::parse_porcelain(
            b"# branch.oid abcdef0123456789\0# branch.head feature/test\0# branch.ab +2 -3\0",
        )
        .unwrap();
        assert_eq!(info.branch, "feature/test");
        assert_eq!(info.sha.as_deref(), Some("abcdef0"));
        assert_eq!((info.ahead, info.behind), (2, 3));
        assert_eq!(info.status, GitStatus::Clean);
    }

    #[test]
    fn filenames_do_not_change_branch_or_conflict_status() {
        let info = GitSegment::parse_porcelain(
            b"# branch.head main\x002 R. N... fields new.txt\0# branch.head fake\0? u UU notes\0",
        )
        .unwrap();
        assert_eq!(info.branch, "main");
        assert_eq!(info.status, GitStatus::Dirty);
    }

    #[test]
    fn handles_unborn_detached_and_conflicted_repositories() {
        let unborn =
            GitSegment::parse_porcelain(b"# branch.oid (initial)\0# branch.head main\0").unwrap();
        assert_eq!(unborn.sha, None);
        assert_eq!(unborn.status, GitStatus::Clean);
        let detached = GitSegment::parse_porcelain(
            b"# branch.oid abcdef0123456789\0# branch.head (detached)\0u UU fields conflict\0? file\0",
        ).unwrap();
        assert_eq!(detached.branch, "detached");
        assert_eq!(detached.status, GitStatus::Conflicts);
    }

    #[test]
    fn incomplete_output_is_not_reported_as_clean() {
        assert!(GitSegment::parse_porcelain(b"# branch.head main").is_none());
        assert!(GitSegment::parse_porcelain(b"# branch.head main\x002 R. fields new\0").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn local_command_timeout_is_bounded() {
        let started = Instant::now();
        let output = run_command_with_timeout("/bin/sleep", &["2"], ".", Duration::from_millis(30));
        assert!(output.is_none());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
