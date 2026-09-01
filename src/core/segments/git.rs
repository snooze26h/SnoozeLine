use super::{sanitize_text, Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;
use std::io::Read;
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
            .take(MAX_GIT_STDOUT_BYTES)
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
    receiver.recv_timeout(remaining).ok()?.ok()
}

#[derive(Debug)]
pub struct GitInfo {
    pub branch: String,
    pub status: GitStatus,
    pub ahead: u32,
    pub behind: u32,
    pub sha: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum GitStatus {
    Clean,
    Dirty,
    Conflicts,
}

pub struct GitSegment {
    show_sha: bool,
}

impl Default for GitSegment {
    fn default() -> Self {
        Self::new()
    }
}

impl GitSegment {
    pub fn new() -> Self {
        Self { show_sha: false }
    }

    pub fn with_sha(mut self, show_sha: bool) -> Self {
        self.show_sha = show_sha;
        self
    }

    fn get_git_info(&self, working_dir: &str) -> Option<GitInfo> {
        if !self.is_git_repository(working_dir) {
            return None;
        }

        let branch = self
            .get_branch(working_dir)
            .unwrap_or_else(|| "detached".to_string());
        let status = self.get_status(working_dir)?;
        let (ahead, behind) = self.get_ahead_behind(working_dir);
        let sha = if self.show_sha {
            self.get_sha(working_dir)
        } else {
            None
        };

        Some(GitInfo {
            branch,
            status,
            ahead,
            behind,
            sha,
        })
    }

    fn is_git_repository(&self, working_dir: &str) -> bool {
        Self::run_git(
            working_dir,
            &["--no-optional-locks", "rev-parse", "--git-dir"],
        )
        .is_some()
    }

    fn get_branch(&self, working_dir: &str) -> Option<String> {
        if let Some(output) = Self::run_git(
            working_dir,
            &["--no-optional-locks", "branch", "--show-current"],
        ) {
            let branch = String::from_utf8(output).ok()?.trim().to_string();
            if !branch.is_empty() {
                return Some(branch);
            }
        }

        if let Some(output) = Self::run_git(
            working_dir,
            &["--no-optional-locks", "symbolic-ref", "--short", "HEAD"],
        ) {
            let branch = String::from_utf8(output).ok()?.trim().to_string();
            if !branch.is_empty() {
                return Some(branch);
            }
        }

        None
    }

    fn get_status(&self, working_dir: &str) -> Option<GitStatus> {
        let output = Self::run_git(
            working_dir,
            &["--no-optional-locks", "status", "--porcelain=v1", "-z"],
        )?;
        Some(Self::parse_status(&output))
    }

    fn parse_status(output: &[u8]) -> GitStatus {
        let mut dirty = false;
        let mut records = output.split(|byte| *byte == 0);

        while let Some(record) = records.next() {
            if record.len() < 3 || record[2] != b' ' {
                continue;
            }

            let status = [record[0], record[1]];
            dirty = true;
            if matches!(
                status,
                [b'D', b'D']
                    | [b'A', b'U']
                    | [b'U', b'D']
                    | [b'U', b'A']
                    | [b'D', b'U']
                    | [b'A', b'A']
                    | [b'U', b'U']
            ) {
                return GitStatus::Conflicts;
            }

            if matches!(record[0], b'R' | b'C') || matches!(record[1], b'R' | b'C') {
                records.next();
            }
        }

        if dirty {
            GitStatus::Dirty
        } else {
            GitStatus::Clean
        }
    }

    fn get_ahead_behind(&self, working_dir: &str) -> (u32, u32) {
        let ahead = self.get_commit_count(working_dir, "@{u}..HEAD");
        let behind = self.get_commit_count(working_dir, "HEAD..@{u}");
        (ahead, behind)
    }

    fn get_commit_count(&self, working_dir: &str, range: &str) -> u32 {
        Self::run_git(
            working_dir,
            &["--no-optional-locks", "rev-list", "--count", range],
        )
        .and_then(|output| String::from_utf8(output).ok())
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
    }

    fn get_sha(&self, working_dir: &str) -> Option<String> {
        let output = Self::run_git(
            working_dir,
            &["--no-optional-locks", "rev-parse", "--short=7", "HEAD"],
        )?;
        let sha = String::from_utf8(output).ok()?.trim().to_string();
        if sha.is_empty() {
            None
        } else {
            Some(sha)
        }
    }

    fn run_git(working_dir: &str, args: &[&str]) -> Option<Vec<u8>> {
        run_command_with_timeout("git", args, working_dir, GIT_COMMAND_TIMEOUT)
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

        if let Some(ref sha) = git_info.sha {
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

        if let Some(ref sha) = git_info.sha {
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
    fn recognizes_every_unmerged_status() {
        for status in ["DD", "AU", "UD", "UA", "DU", "AA", "UU"] {
            let output = format!("{} conflict.txt\0", status);
            assert_eq!(
                GitSegment::parse_status(output.as_bytes()),
                GitStatus::Conflicts,
                "status {status}"
            );
        }
    }

    #[test]
    fn filenames_do_not_trigger_false_conflicts() {
        assert_eq!(
            GitSegment::parse_status(b" M notes-UU-AA-DD.txt\0"),
            GitStatus::Dirty
        );
        assert_eq!(GitSegment::parse_status(b""), GitStatus::Clean);
    }

    #[test]
    fn skips_the_second_path_in_rename_records() {
        assert_eq!(
            GitSegment::parse_status(b"R  new.txt\0UU old-looking-name.txt\0"),
            GitStatus::Dirty
        );
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
