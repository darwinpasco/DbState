use crate::git::git_root;
use crate::project::{PathKind, EXPECTED_PATHS};
use crate::service::{
    parse_service_request, request_string, service_error_response, service_json_response,
    validate_service_request_is_safe, ServiceHttpResponse,
};
use crate::workspace::resolve_service_workspace;
use crate::*;
use std::fs;
use std::path::{Component, Path, PathBuf};

const OBJECTS_ROOT: &str = "database/objects";
pub(crate) const REPOSITORY_OBJECT_PREVIEW_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepositoryObjectEntry {
    kind: RepositoryObjectEntryKind,
    name: String,
    relative_path: String,
    depth: usize,
    object_category: Option<String>,
    extension: Option<String>,
    size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepositoryObjectEntryKind {
    Directory,
    File,
}

pub(crate) fn service_repository_object_files_list_endpoint(
    body: &str,
    cwd: &Path,
) -> ServiceHttpResponse {
    let command = "repository object files list";
    let root = match resolve_repository_root_from_request(command, body, cwd) {
        Ok(root) => root,
        Err(response) => return response,
    };
    match list_repository_object_files(&root) {
        Ok(entries) => service_json_response(200, &repository_object_listing_json(&root, &entries)),
        Err(error) => service_error_response(400, command, &error),
    }
}

pub(crate) fn service_repository_object_file_preview_endpoint(
    body: &str,
    cwd: &Path,
) -> ServiceHttpResponse {
    let command = "repository object file preview";
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, command, &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, command, &error);
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, command, &error),
        };
    let Some(root) = git_root(&workspace) else {
        return service_error_response(
            400,
            command,
            "repositoryPath must be inside a local Git working tree.",
        );
    };
    let Some(relative_path) = request_string(&request, "path")
        .or_else(|| request_string(&request, "relativePath"))
        .or_else(|| request_string(&request, "objectPath"))
    else {
        return service_error_response(400, command, "repository object file path is required.");
    };
    match preview_repository_object_file(&root, &relative_path) {
        Ok(preview) => service_json_response(200, &repository_object_preview_json(&preview)),
        Err(error) => service_error_response(400, command, &error),
    }
}

fn resolve_repository_root_from_request(
    command: &str,
    body: &str,
    cwd: &Path,
) -> Result<PathBuf, ServiceHttpResponse> {
    let request = parse_service_request(body)
        .map_err(|error| service_error_response(400, command, &error))?;
    validate_service_request_is_safe(&request)
        .map_err(|error| service_error_response(400, command, &error))?;
    let workspace =
        resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
            .map_err(|error| service_error_response(400, command, &error))?;
    git_root(&workspace).ok_or_else(|| {
        service_error_response(
            400,
            command,
            "repositoryPath must be inside a local Git working tree.",
        )
    })
}

fn list_repository_object_files(root: &Path) -> Result<Vec<RepositoryObjectEntry>, String> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|_| "Could not resolve selected repository root.".to_string())?;
    let objects_root = canonical_objects_root(root)?;
    let mut entries = Vec::new();
    if canonical_root
        .join("database")
        .join("reference-data")
        .join("dbstate.reference-data.yml")
        .is_file()
    {
        collect_initialized_project_directories(&canonical_root, &mut entries)?;
    }
    collect_repository_object_entries(&objects_root, &objects_root, 0, &mut entries)?;
    entries.sort_by(|left, right| {
        let left_kind = match left.kind {
            RepositoryObjectEntryKind::Directory => 0,
            RepositoryObjectEntryKind::File => 1,
        };
        let right_kind = match right.kind {
            RepositoryObjectEntryKind::Directory => 0,
            RepositoryObjectEntryKind::File => 1,
        };
        left_kind
            .cmp(&right_kind)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    entries.dedup_by(|left, right| {
        left.kind == right.kind && left.relative_path == right.relative_path
    });
    Ok(entries)
}

fn collect_initialized_project_directories(
    root: &Path,
    entries: &mut Vec<RepositoryObjectEntry>,
) -> Result<(), String> {
    for expected in EXPECTED_PATHS {
        if expected.kind != PathKind::Directory {
            continue;
        }
        let target = root.join(expected.relative);
        if !target.is_dir() {
            continue;
        }
        let canonical = fs::canonicalize(&target)
            .map_err(|_| "Could not resolve DbState project directory path.".to_string())?;
        if !canonical.starts_with(root) {
            return Err(
                "Refusing to list DbState project directory outside repository.".to_string(),
            );
        }
        entries.push(RepositoryObjectEntry {
            kind: RepositoryObjectEntryKind::Directory,
            name: file_name(&canonical)?,
            relative_path: expected.relative.to_string(),
            depth: expected.relative.matches('/').count(),
            object_category: object_category(expected.relative),
            extension: None,
            size_bytes: None,
        });
    }
    Ok(())
}

fn collect_repository_object_entries(
    objects_root: &Path,
    directory: &Path,
    depth: usize,
    entries: &mut Vec<RepositoryObjectEntry>,
) -> Result<(), String> {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| {
        format!(
            "Could not read repository object directory {}: {error}",
            display_path(directory)
        )
    })? {
        let entry =
            entry.map_err(|error| format!("Could not read repository object entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not read repository object entry type: {error}"))?;
        if file_type.is_dir() {
            directories.push(entry.path());
        } else if file_type.is_file() && has_sql_extension(&entry.path()) {
            files.push(entry.path());
        }
    }

    directories.sort_by_key(|left| path_name_key(left));
    files.sort_by_key(|left| path_name_key(left));

    for directory in directories {
        let canonical = fs::canonicalize(&directory)
            .map_err(|_| "Could not resolve repository object directory path.".to_string())?;
        if !canonical.starts_with(objects_root) {
            return Err(
                "Refusing to list repository object directory outside database/objects/."
                    .to_string(),
            );
        }
        let relative_path = repository_relative_path(objects_root, &canonical, true)?;
        if repository_entry_exists(
            entries,
            RepositoryObjectEntryKind::Directory,
            &relative_path,
        ) {
            collect_repository_object_entries(objects_root, &canonical, depth + 1, entries)?;
            continue;
        }
        entries.push(RepositoryObjectEntry {
            kind: RepositoryObjectEntryKind::Directory,
            name: file_name(&canonical)?,
            object_category: object_category(&relative_path),
            relative_path,
            depth,
            extension: None,
            size_bytes: None,
        });
        collect_repository_object_entries(objects_root, &canonical, depth + 1, entries)?;
    }

    for file in files {
        let canonical = fs::canonicalize(&file)
            .map_err(|_| "Could not resolve repository object file path.".to_string())?;
        if !canonical.starts_with(objects_root) {
            return Err(
                "Refusing to list repository object file outside database/objects/.".to_string(),
            );
        }
        let metadata = fs::metadata(&canonical)
            .map_err(|error| format!("Could not read repository object file metadata: {error}"))?;
        let relative_path = repository_relative_path(objects_root, &canonical, false)?;
        entries.push(RepositoryObjectEntry {
            kind: RepositoryObjectEntryKind::File,
            name: file_name(&canonical)?,
            object_category: object_category(&relative_path),
            relative_path,
            depth,
            extension: Some(".sql".to_string()),
            size_bytes: Some(metadata.len()),
        });
    }
    Ok(())
}

fn repository_entry_exists(
    entries: &[RepositoryObjectEntry],
    kind: RepositoryObjectEntryKind,
    relative_path: &str,
) -> bool {
    entries
        .iter()
        .any(|entry| entry.kind == kind && entry.relative_path == relative_path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepositoryObjectPreview {
    relative_path: String,
    name: String,
    content: String,
    size_bytes: u64,
    object_category: Option<String>,
}

fn preview_repository_object_file(
    root: &Path,
    relative_path: &str,
) -> Result<RepositoryObjectPreview, String> {
    let normalized = validate_repository_object_preview_path(relative_path)?;
    let objects_root = canonical_objects_root(root)?;
    let target = root.join(normalized.replace('/', std::path::MAIN_SEPARATOR_STR));
    let canonical = fs::canonicalize(&target)
        .map_err(|_| "Repository object file was not found or cannot be accessed.".to_string())?;
    if !canonical.starts_with(&objects_root) {
        return Err("Refusing to preview outside database/objects/.".to_string());
    }
    if canonical.is_dir() {
        return Err("Repository object preview requires a SQL file, not a directory.".to_string());
    }
    if !has_sql_extension(&canonical) {
        return Err("Repository object preview supports only .sql files.".to_string());
    }
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("Could not read repository object file metadata: {error}"))?;
    if metadata.len() > REPOSITORY_OBJECT_PREVIEW_MAX_BYTES {
        return Err(
            "Repository object file is too large to preview. Maximum preview size is 1 MiB."
                .to_string(),
        );
    }
    let content = fs::read_to_string(&canonical)
        .map_err(|error| format!("Could not read repository object SQL file: {error}"))?;
    Ok(RepositoryObjectPreview {
        object_category: object_category(&normalized),
        name: file_name(&canonical)?,
        relative_path: normalized,
        content,
        size_bytes: metadata.len(),
    })
}

fn canonical_objects_root(root: &Path) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root)
        .map_err(|_| "Could not resolve selected repository root.".to_string())?;
    let objects_root = root.join("database").join("objects");
    let canonical = fs::canonicalize(&objects_root).map_err(|_| {
        "database/objects/ was not found. Initialize or update the DbState project structure first."
            .to_string()
    })?;
    if !canonical.is_dir() {
        return Err("database/objects/ must be a directory.".to_string());
    }
    Ok(canonical)
}

fn validate_repository_object_preview_path(relative_path: &str) -> Result<String, String> {
    let raw = relative_path.trim();
    if raw.is_empty() {
        return Err("repository object file path is required.".to_string());
    }
    if raw.contains('\0') || raw.contains('%') {
        return Err("repository object file path contains unsafe characters.".to_string());
    }
    if raw.contains(':') {
        return Err(
            "repository object file path must be repository-relative, not drive-qualified."
                .to_string(),
        );
    }
    let normalized = raw.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err("repository object file path must be relative.".to_string());
    }
    let path = Path::new(&normalized);
    if path.is_absolute() {
        return Err("repository object file path must be relative.".to_string());
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(
                "repository object file path must not contain traversal segments.".to_string(),
            );
        }
    }
    if !normalized.starts_with("database/objects/") {
        return Err(
            "Repository object preview supports only files under database/objects/.".to_string(),
        );
    }
    if normalized == "database/objects/" || normalized.ends_with('/') {
        return Err("Repository object preview requires a SQL file, not a directory.".to_string());
    }
    if !normalized.to_ascii_lowercase().ends_with(".sql") {
        return Err("Repository object preview supports only .sql files.".to_string());
    }
    Ok(normalized)
}

fn has_sql_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sql"))
}

fn repository_relative_path(
    objects_root: &Path,
    canonical: &Path,
    directory: bool,
) -> Result<String, String> {
    let relative = canonical
        .strip_prefix(objects_root)
        .map_err(|_| "Could not create repository-relative object path.".to_string())?;
    let suffix = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    let path = if suffix.is_empty() {
        OBJECTS_ROOT.to_string()
    } else {
        format!("{OBJECTS_ROOT}/{suffix}")
    };
    let _ = directory;
    Ok(path)
}

fn file_name(path: &Path) -> Result<String, String> {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| "Repository object path has no file name.".to_string())
}

fn path_name_key(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn object_category(relative_path: &str) -> Option<String> {
    let rest = relative_path
        .trim_end_matches('/')
        .strip_prefix("database/objects/")?;
    let first = rest.split('/').next().unwrap_or_default();
    match first {
        "schemas" => Some("schema".to_string()),
        "extensions" => Some("extension".to_string()),
        "enums" => Some("enum".to_string()),
        "domains" => Some("domain".to_string()),
        "aggregates" => Some("aggregate".to_string()),
        "sequences" => Some("sequence".to_string()),
        "tables" => Some("table".to_string()),
        "indexes" => Some("index".to_string()),
        "views" => Some("view".to_string()),
        "materialized-views" => Some("materializedView".to_string()),
        "functions" => Some("function".to_string()),
        "triggers" => Some("trigger".to_string()),
        "grants" => Some("grant".to_string()),
        "constraints" => Some("constraint".to_string()),
        "rls-policies" => Some("rlsPolicy".to_string()),
        _ => None,
    }
}

fn repository_object_listing_json(root: &Path, entries: &[RepositoryObjectEntry]) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", "repository object files list", true);
    write_json_bool_field(&mut json, "success", true);
    write_json_string_field(&mut json, "repositoryPath", &display_path(root), false);
    write_json_string_field(&mut json, "root", OBJECTS_ROOT, false);
    write!(
        json,
        ",\"{}\":{}",
        escape_json("maxPreviewBytes"),
        REPOSITORY_OBJECT_PREVIEW_MAX_BYTES
    )
    .ok();
    write!(json, ",\"{}\":true", escape_json("readOnly")).ok();
    write!(json, ",\"{}\":false", escape_json("sqlExecutionAvailable")).ok();
    write!(json, ",\"{}\":false", escape_json("gitMutationAvailable")).ok();
    write!(json, ",\"{}\":[", escape_json("entries")).ok();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write_repository_object_entry_json(&mut json, entry);
    }
    json.push(']');
    write_json_array_field(&mut json, "warnings", &[]);
    write_json_array_field(&mut json, "errors", &[]);
    json.push('}');
    json
}

fn repository_object_preview_json(preview: &RepositoryObjectPreview) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", "repository object file preview", true);
    write_json_bool_field(&mut json, "success", true);
    write_json_string_field(&mut json, "path", &preview.relative_path, false);
    write_json_string_field(&mut json, "relativePath", &preview.relative_path, false);
    write_json_string_field(&mut json, "name", &preview.name, false);
    write_json_string_field(&mut json, "fileName", &preview.name, false);
    write_json_optional_string_field(
        &mut json,
        "objectCategory",
        preview.object_category.as_deref(),
    );
    write!(json, ",\"{}\":\".sql\"", escape_json("extension")).ok();
    write!(
        json,
        ",\"{}\":{}",
        escape_json("sizeBytes"),
        preview.size_bytes
    )
    .ok();
    write!(json, ",\"{}\":true", escape_json("readOnly")).ok();
    write!(json, ",\"{}\":false", escape_json("sqlExecutionAvailable")).ok();
    write!(json, ",\"{}\":false", escape_json("gitMutationAvailable")).ok();
    write_json_string_field(&mut json, "content", &preview.content, false);
    write_json_array_field(&mut json, "warnings", &[]);
    write_json_array_field(&mut json, "errors", &[]);
    json.push('}');
    json
}

fn write_repository_object_entry_json(json: &mut String, entry: &RepositoryObjectEntry) {
    json.push('{');
    write_json_string_field(
        json,
        "kind",
        match entry.kind {
            RepositoryObjectEntryKind::Directory => "directory",
            RepositoryObjectEntryKind::File => "file",
        },
        true,
    );
    write_json_string_field(json, "name", &entry.name, false);
    write_json_string_field(json, "path", &entry.relative_path, false);
    write_json_string_field(json, "relativePath", &entry.relative_path, false);
    write!(json, ",\"{}\":{}", escape_json("depth"), entry.depth).ok();
    write_json_optional_string_field(json, "objectCategory", entry.object_category.as_deref());
    write_json_optional_string_field(json, "extension", entry.extension.as_deref());
    match entry.size_bytes {
        Some(size) => write!(json, ",\"{}\":{}", escape_json("sizeBytes"), size).ok(),
        None => write!(json, ",\"{}\":null", escape_json("sizeBytes")).ok(),
    };
    json.push('}');
}
