//! 按工作目录缓存 Git 状态；沿用 CCometixLine v2 的后台刷新思路。

use super::git::{GitInfo, GitSegment};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_STALE_MS: u64 = 30_000;
const LOCK_STALE: Duration = Duration::from_secs(30);
const REFRESH_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_CACHE_BYTES: u64 = 16 * 1024;

#[derive(Serialize, Deserialize)]
struct Entry {
    directory: PathBuf,
    at_ms: u64,
    info: GitInfo,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

fn paths(directory: &Path) -> (PathBuf, PathBuf) {
    let mut hasher = DefaultHasher::new();
    directory.hash(&mut hasher);
    let key = format!("{:016x}", hasher.finish());
    let root = crate::config::paths::root_dir().join("git-cache");
    (
        root.join(format!("{key}.json")),
        root.join(format!("{key}.lock")),
    )
}

fn read(path: &Path, directory: &Path, max_age_ms: u64) -> Option<(GitInfo, u64)> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(MAX_CACHE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_CACHE_BYTES {
        return None;
    }
    let entry: Entry = serde_json::from_slice(&bytes).ok()?;
    let age = now_ms().checked_sub(entry.at_ms)?;
    if entry.directory != directory || age >= max_age_ms || entry.info.branch.is_empty() {
        return None;
    }
    Some((entry.info, age))
}

fn store(path: &Path, directory: &Path, info: &GitInfo) {
    let Some(parent) = path.parent() else { return };
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec(&Entry {
            directory: directory.to_path_buf(),
            at_ms: now_ms(),
            info: info.clone(),
        })?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(&bytes)?;
        drop(file);
        // 原子替换，前台永远不会读到后台只写了一半的 JSON。
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
}

fn claim(lock: &Path) -> bool {
    let Some(parent) = lock.parent() else {
        return false;
    };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    if fs::metadata(lock)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > LOCK_STALE)
    {
        let _ = fs::remove_file(lock);
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock)
        .is_ok()
}

fn refresh_in_background(directory: &Path, lock: &Path) {
    if !claim(lock) {
        return;
    }
    let launched = std::env::current_exe().ok().is_some_and(|executable| {
        let mut command = Command::new(executable);
        command
            .arg("--refresh-git")
            .arg(directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        command.spawn().is_ok()
    });
    if !launched {
        let _ = fs::remove_file(lock);
    }
}

pub(super) fn status(directory: &Path, ttl_ms: u64) -> Option<GitInfo> {
    let directory = directory.canonicalize().ok()?;
    if !GitSegment::in_repository(&directory) {
        return None;
    }
    // 自定义索引/工作树不是普通 cwd 的状态，不能与该目录的常规缓存混用。
    let custom_git = [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_NAMESPACE",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some());
    if ttl_ms == 0 || custom_git {
        return GitSegment::read_git_info(&directory, GitSegment::render_timeout());
    }
    let (path, lock) = paths(&directory);
    if let Some((info, age)) = read(&path, &directory, MAX_STALE_MS.max(ttl_ms)) {
        if age >= ttl_ms {
            refresh_in_background(&directory, &lock);
        }
        return Some(info);
    }
    match GitSegment::read_git_info(&directory, GitSegment::render_timeout()) {
        Some(info) => {
            store(&path, &directory, &info);
            Some(info)
        }
        None => {
            refresh_in_background(&directory, &lock);
            None
        }
    }
}

/// 仅由内部工作进程调用；不读取标准输入、账号或对话记录。
pub fn refresh(directory: &Path) {
    if !directory.is_absolute() {
        return;
    }
    let (path, lock) = paths(directory);
    if GitSegment::in_repository(directory) {
        if let Some(info) = GitSegment::read_git_info(directory, REFRESH_TIMEOUT) {
            store(&path, directory, &info);
        }
    }
    let _ = fs::remove_file(lock);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::segments::git::GitStatus;

    #[test]
    fn cache_checks_directory_age_and_refresh_lock() {
        let root =
            std::env::temp_dir().join(format!("snoozeline-git-cache-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("state.json");
        let lock = root.join("refresh.lock");
        let info = GitInfo {
            branch: "main".into(),
            status: GitStatus::Clean,
            ahead: 0,
            behind: 0,
            sha: None,
        };
        store(&path, &root, &info);
        assert_eq!(read(&path, &root, 60_000).unwrap().0.branch, "main");
        assert!(read(&path, &root.join("other"), 60_000).is_none());
        assert!(read(&path, &root, 0).is_none());
        assert!(claim(&lock));
        assert!(!claim(&lock));
        fs::write(&path, b"{incomplete").unwrap();
        assert!(read(&path, &root, 60_000).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
