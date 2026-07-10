use crate::git::git_root;
use crate::*;
#[cfg(not(target_os = "windows"))]
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceDirectoryEntry {
    pub(crate) name: String,
    pub(crate) path: String,
}

pub(crate) fn workspace_root_candidates(cwd: &Path) -> Vec<WorkspaceDirectoryEntry> {
    let mut roots = Vec::new();
    if let Ok(canonical) = fs::canonicalize(cwd) {
        if canonical.is_dir() {
            roots.push(WorkspaceDirectoryEntry {
                name: "Service working directory".to_string(),
                path: display_path(&canonical),
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        for letter in b'A'..=b'Z' {
            let path = format!("{}:\\", letter as char);
            let candidate = PathBuf::from(&path);
            if candidate.is_dir() {
                roots.push(WorkspaceDirectoryEntry {
                    name: path.clone(),
                    path,
                });
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let root = PathBuf::from("/");
        if root.is_dir() {
            roots.push(WorkspaceDirectoryEntry {
                name: "/".to_string(),
                path: "/".to_string(),
            });
        }
        if let Ok(home) = env::var("HOME") {
            if !home.trim().is_empty() {
                let home_path = PathBuf::from(&home);
                if home_path.is_dir() {
                    roots.push(WorkspaceDirectoryEntry {
                        name: "Home".to_string(),
                        path: display_path(&home_path),
                    });
                }
            }
        }
    }

    roots
}

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceDirectoryListing {
    pub(crate) path: String,
    pub(crate) parent_path: Option<String>,
    pub(crate) directories: Vec<WorkspaceDirectoryEntry>,
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn validate_browse_directory_value(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("path is required.".to_string());
    }
    if trimmed.contains('\0') {
        return Err("path contains an invalid null byte.".to_string());
    }
    if is_url_like_repository_path(trimmed) {
        return Err(
            "path must be a local filesystem path, not a URL or remote repository reference."
                .to_string(),
        );
    }
    let canonical = fs::canonicalize(PathBuf::from(normalize_local_path_input(trimmed)))
        .map_err(|_| "path does not exist or cannot be accessed.".to_string())?;
    if !canonical.is_dir() {
        return Err("path must point to a directory.".to_string());
    }
    Ok(canonical)
}

pub(crate) fn workspace_directory_listing(
    path: &Path,
) -> Result<WorkspaceDirectoryListing, String> {
    let mut directories = Vec::new();
    let mut warnings = Vec::new();
    let entries =
        fs::read_dir(path).map_err(|error| format!("Could not read directory: {error}"))?;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("Could not read one directory entry: {error}"));
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                warnings.push(format!("Could not read one directory entry type: {error}"));
                continue;
            }
        };
        if !file_type.is_dir() {
            continue;
        }
        let child_path = entry.path();
        directories.push(WorkspaceDirectoryEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: display_path(&child_path),
        });
    }
    directories.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
    let parent_path = path
        .parent()
        .filter(|parent| *parent != path)
        .map(display_path);
    Ok(WorkspaceDirectoryListing {
        path: display_path(path),
        parent_path,
        directories,
        warnings,
    })
}

pub(crate) fn workspace_roots_json(roots: &[WorkspaceDirectoryEntry], cwd: &Path) -> String {
    let current_path = fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", "workspace roots", true);
    write_json_bool_field(&mut json, "success", true);
    write_json_string_field(
        &mut json,
        "currentPath",
        &display_path(&current_path),
        false,
    );
    write_workspace_directory_array_field(&mut json, "roots", roots);
    write_json_array_field(&mut json, "warnings", &[]);
    write_json_array_field(&mut json, "errors", &[]);
    json.push('}');
    json
}

pub(crate) fn workspace_directory_listing_json(report: &WorkspaceDirectoryListing) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", "workspace list-directories", true);
    write_json_bool_field(&mut json, "success", true);
    write_json_string_field(&mut json, "path", &report.path, false);
    write_json_optional_string_field(&mut json, "parentPath", report.parent_path.as_deref());
    write_workspace_directory_array_field(&mut json, "directories", &report.directories);
    write_json_array_field(&mut json, "warnings", &report.warnings);
    write_json_array_field(&mut json, "errors", &[]);
    json.push('}');
    json
}

pub(crate) fn write_workspace_directory_array_field(
    json: &mut String,
    name: &str,
    values: &[WorkspaceDirectoryEntry],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "name", &value.name, true);
        write_json_string_field(json, "path", &value.path, false);
        json.push('}');
    }
    json.push(']');
}

pub(crate) fn resolve_service_workspace(
    repository_path: Option<&str>,
    fallback: &Path,
) -> Result<PathBuf, String> {
    let Some(repository_path) = repository_path else {
        return Ok(fallback.to_path_buf());
    };
    let trimmed = repository_path.trim();
    if trimmed.is_empty() {
        return Ok(fallback.to_path_buf());
    }
    validate_repository_path_value(trimmed)
}

fn validate_repository_path_value(value: &str) -> Result<PathBuf, String> {
    if value.contains('\0') {
        return Err("repositoryPath contains an invalid null byte.".to_string());
    }
    if is_url_like_repository_path(value) {
        return Err("repositoryPath must be a local filesystem path, not a URL or remote repository reference.".to_string());
    }

    let path = PathBuf::from(normalize_local_path_input(value));
    let canonical = fs::canonicalize(&path)
        .map_err(|_| "repositoryPath does not exist or cannot be accessed.".to_string())?;
    if !canonical.is_dir() {
        return Err("repositoryPath must point to a directory.".to_string());
    }
    if git_root(&canonical).is_none() {
        return Err("repositoryPath must be inside a local Git working tree.".to_string());
    }
    Ok(canonical)
}

fn is_url_like_repository_path(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ssh://")
        || lower.starts_with("postgres://")
        || lower.starts_with("postgresql://")
        || lower.starts_with("git@")
}

pub(crate) fn normalize_local_path_input(value: &str) -> String {
    let normalized = value.trim().replace('\\', "/");
    normalized
        .strip_prefix("///?/")
        .or_else(|| normalized.strip_prefix("//?/"))
        .unwrap_or(&normalized)
        .to_string()
}
