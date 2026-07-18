use crate::WorkingTreeStatus;
use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn git_root(cwd: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout.trim();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

pub(crate) fn git_branch(root: &Path) -> Option<String> {
    let git_dir = git_dir(root)?;
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    let branch = head.strip_prefix("ref: refs/heads/")?;
    if branch.is_empty() {
        return None;
    }
    Some(branch.to_string())
}

pub(crate) fn git_working_tree_status(root: &Path) -> WorkingTreeStatus {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .arg("--untracked-files=all")
        .current_dir(root)
        .output();

    let Ok(output) = output else {
        return WorkingTreeStatus::Unknown;
    };

    if !output.status.success() {
        return WorkingTreeStatus::Unknown;
    }

    if output.stdout.is_empty() {
        WorkingTreeStatus::Clean
    } else {
        WorkingTreeStatus::Dirty
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDirtyPath {
    pub(crate) path: String,
    pub(crate) status: String,
}

pub(crate) fn git_dirty_paths(root: &Path) -> Vec<GitDirtyPath> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain=v1")
        .arg("-z")
        .arg("--untracked-files=all")
        .current_dir(root)
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    parse_porcelain_z(&output.stdout)
}

fn parse_porcelain_z(output: &[u8]) -> Vec<GitDirtyPath> {
    let mut paths = Vec::new();
    let mut entries = output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());

    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let status = String::from_utf8_lossy(&entry[0..2]).to_string();
        let path = normalize_git_status_path(&entry[3..]);
        if !path.is_empty() {
            paths.push(GitDirtyPath {
                path: path.clone(),
                status: status.clone(),
            });
        }
        if matches!(status.as_bytes().first(), Some(b'R' | b'C'))
            || matches!(status.as_bytes().get(1), Some(b'R' | b'C'))
        {
            if let Some(original) = entries.next() {
                let original_path = normalize_git_status_path(original);
                if !original_path.is_empty() {
                    paths.push(GitDirtyPath {
                        path: original_path,
                        status: status.clone(),
                    });
                }
            }
        }
    }

    paths.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.status.cmp(&right.status))
    });
    paths.dedup();
    paths
}

fn normalize_git_status_path(path: &[u8]) -> String {
    String::from_utf8_lossy(path).trim().replace('\\', "/")
}

pub(crate) fn git_default_branch(root: &Path) -> Option<String> {
    let git_dir = git_dir(root)?;
    let origin_head = git_dir.join("refs/remotes/origin/HEAD");
    let head = fs::read_to_string(origin_head).ok()?;
    let branch = head.trim().strip_prefix("ref: refs/remotes/origin/")?;
    if branch.is_empty() {
        return None;
    }
    Some(branch.to_string())
}

pub(crate) fn is_protected_branch(branch: Option<&str>, default_branch: Option<&str>) -> bool {
    let Some(branch) = branch else {
        return false;
    };
    matches!(branch, "main" | "master" | "dev" | "develop")
        || default_branch
            .filter(|default| !default.is_empty())
            .is_some_and(|default| default == branch)
}

pub(crate) fn safe_relative_repo_path(path: &str) -> bool {
    if path.trim().is_empty() || path.contains('\\') || path.contains(':') || path.starts_with('/')
    {
        return false;
    }
    Path::new(path)
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn git_dir(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let content = fs::read_to_string(&dot_git).ok()?;
    let gitdir = content.trim().strip_prefix("gitdir:")?.trim();
    let path = PathBuf::from(gitdir);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(root.join(path))
    }
}
