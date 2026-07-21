use crate::git::{
    git_branch, git_default_branch, git_dirty_paths, git_root, git_working_tree_status,
    is_protected_branch, safe_relative_repo_path,
};
use crate::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const DEFAULT_REGISTRY: &str = "version: 1\ntables: []\n";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbStateProjectStatus {
    NotGitRepository,
    GitRepositoryWithoutDbStateStructure,
    PartialDbStateStructure,
    CompleteDbStateStructure,
}

impl DbStateProjectStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotGitRepository => "notGitRepository",
            Self::GitRepositoryWithoutDbStateStructure => "gitRepositoryWithoutDbStateStructure",
            Self::PartialDbStateStructure => "partialDbStateStructure",
            Self::CompleteDbStateStructure => "completeDbStateStructure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkingTreeStatus {
    Clean,
    Dirty,
    Unknown,
}

impl WorkingTreeStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub default_branch: Option<String>,
    pub is_protected_branch: bool,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub dirty_paths: Vec<String>,
    pub dbstate_project_status: DbStateProjectStatus,
    pub missing_paths: Vec<String>,
    pub existing_paths: Vec<String>,
    pub planned_creates: Vec<String>,
    pub created_paths: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpectedPath {
    pub(crate) relative: &'static str,
    pub(crate) kind: PathKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathKind {
    Directory,
    File,
}

pub(crate) const EXPECTED_PATHS: &[ExpectedPath] = &[
    ExpectedPath {
        relative: "database",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/schemas",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/extensions",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/enums",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/domains",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/aggregates",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/sequences",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/tables",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/indexes",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/views",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/constraints",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/constraints/primary-keys",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/constraints/unique-constraints",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/constraints/foreign-keys",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/constraints/check-constraints",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/materialized-views",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/functions",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/triggers",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/grants",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/grants/schemas",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/grants/tables",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/grants/views",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/grants/materialized-views",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/grants/sequences",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/grants/functions",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/objects/rls-policies",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/reference-data",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/reference-data/dbstate.reference-data.yml",
        kind: PathKind::File,
    },
    ExpectedPath {
        relative: "database/reference-data/tables",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/releases",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/releases/objects",
        kind: PathKind::Directory,
    },
    ExpectedPath {
        relative: "database/releases/reference-data",
        kind: PathKind::Directory,
    },
];

pub(crate) fn missing_paths_are_only_release_artifact_subfolders(paths: &[String]) -> bool {
    !paths.is_empty()
        && paths.iter().all(|path| {
            path == "database/releases/objects" || path == "database/releases/reference-data"
        })
}

pub(crate) fn project_structure_allows_release_subfolder_backfill(report: &ProjectReport) -> bool {
    report.is_git_repository
        && report.dbstate_project_status == DbStateProjectStatus::PartialDbStateStructure
        && missing_paths_are_only_release_artifact_subfolders(&report.missing_paths)
}

pub fn status_report(cwd: &Path, command: CommandKind) -> ProjectReport {
    let repository_path = display_path(cwd);
    let git_root = git_root(cwd);

    let mut report = ProjectReport {
        command,
        success: git_root.is_some(),
        repository_path,
        git_root: git_root.as_ref().map(|path| display_path(path)),
        is_git_repository: git_root.is_some(),
        branch: None,
        default_branch: None,
        is_protected_branch: false,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        dirty_paths: Vec::new(),
        dbstate_project_status: DbStateProjectStatus::NotGitRepository,
        missing_paths: Vec::new(),
        existing_paths: Vec::new(),
        planned_creates: Vec::new(),
        created_paths: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let Some(root) = git_root else {
        report
            .errors
            .push("Current path is not inside a Git repository.".to_string());
        return report;
    };

    report.branch = git_branch(&root);
    report.default_branch = git_default_branch(&root);
    report.is_protected_branch =
        is_protected_branch(report.branch.as_deref(), report.default_branch.as_deref());
    report.working_tree_status = git_working_tree_status(&root);
    report.is_dirty = report.working_tree_status == WorkingTreeStatus::Dirty;
    report.dirty_paths = git_dirty_paths(&root)
        .into_iter()
        .map(|entry| entry.path)
        .collect();
    if report.is_dirty {
        report
            .warnings
            .push("Working tree has changes. Read-only actions are allowed. File writes are allowed only when dirty paths do not overlap intended DbState write paths.".to_string());
    }

    let validation = validate_layout(&root);
    report.missing_paths = validation.missing_paths;
    report.existing_paths = validation.existing_paths;
    report.dbstate_project_status = validation.status;
    report
}

pub fn init_project(cwd: &Path, dry_run: bool) -> Result<ProjectReport, String> {
    let mut report = status_report(cwd, CommandKind::Init);

    if !report.is_git_repository {
        report.success = false;
        return Ok(report);
    }

    report.planned_creates = report.missing_paths.clone();

    if dry_run {
        report.success = true;
        return Ok(report);
    }

    let Some(root) = &report.git_root else {
        return Ok(report);
    };
    let root = PathBuf::from(root);
    let guard = scoped_write_guard(
        &root,
        report.branch.as_deref(),
        report.default_branch.as_deref(),
        &report.planned_creates,
        GitHandoffWorkflow::ProjectInit,
        "initialize dbstate project",
    );
    report.warnings.extend(guard.warnings);
    if !guard.allowed {
        report.success = false;
        report.errors.extend(guard.errors);
        return Ok(report);
    }

    for expected in EXPECTED_PATHS {
        let target = root.join(expected.relative);
        match expected.kind {
            PathKind::Directory => {
                if !target.exists() {
                    fs::create_dir_all(&target).map_err(|error| {
                        format!("Could not create {}: {error}", expected.relative)
                    })?;
                    report.created_paths.push(expected.relative.to_string());
                }
            }
            PathKind::File => {
                if !target.exists() {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|error| {
                            format!("Could not create {}: {error}", display_path(parent))
                        })?;
                    }
                    fs::write(&target, DEFAULT_REGISTRY).map_err(|error| {
                        format!("Could not create {}: {error}", expected.relative)
                    })?;
                    report.created_paths.push(expected.relative.to_string());
                }
            }
        }
    }

    let validation = validate_layout(&root);
    report.missing_paths = validation.missing_paths;
    report.existing_paths = validation.existing_paths;
    report.dbstate_project_status = validation.status;
    report.success = report.errors.is_empty();
    Ok(report)
}

#[derive(Debug, Clone)]
struct LayoutValidation {
    status: DbStateProjectStatus,
    missing_paths: Vec<String>,
    existing_paths: Vec<String>,
}

fn validate_layout(root: &Path) -> LayoutValidation {
    let mut missing_paths = Vec::new();
    let mut existing_paths = Vec::new();

    for expected in EXPECTED_PATHS {
        let target = root.join(expected.relative);
        let exists = match expected.kind {
            PathKind::Directory => target.is_dir(),
            PathKind::File => target.is_file(),
        };

        if exists {
            existing_paths.push(expected.relative.to_string());
        } else {
            missing_paths.push(expected.relative.to_string());
        }
    }

    let status = if existing_paths.is_empty() {
        DbStateProjectStatus::GitRepositoryWithoutDbStateStructure
    } else if missing_paths.is_empty() {
        DbStateProjectStatus::CompleteDbStateStructure
    } else {
        DbStateProjectStatus::PartialDbStateStructure
    };

    LayoutValidation {
        status,
        missing_paths,
        existing_paths,
    }
}

impl ProjectReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Repository path: {}", self.repository_path).ok();
        writeln!(
            text,
            "Git root: {}",
            self.git_root.as_deref().unwrap_or("<none>")
        )
        .ok();
        writeln!(text, "Git repository: {}", self.is_git_repository).ok();
        writeln!(
            text,
            "Branch: {}",
            self.branch.as_deref().unwrap_or("<none>")
        )
        .ok();
        writeln!(text, "Protected branch: {}", self.is_protected_branch).ok();
        writeln!(text, "Working tree: {}", self.working_tree_status.as_str()).ok();
        writeln!(text, "Dirty path count: {}", self.dirty_paths.len()).ok();
        writeln!(text, "Dirty paths:").ok();
        for path in &self.dirty_paths {
            writeln!(text, "  - {path}").ok();
        }
        writeln!(
            text,
            "DbState status: {}",
            self.dbstate_project_status.as_str()
        )
        .ok();
        writeln!(text, "Missing paths:").ok();
        for path in &self.missing_paths {
            writeln!(text, "  - {path}").ok();
        }
        writeln!(text, "Planned creates:").ok();
        for path in &self.planned_creates {
            writeln!(text, "  - {path}").ok();
        }
        writeln!(text, "Created paths:").ok();
        for path in &self.created_paths {
            writeln!(text, "  - {path}").ok();
        }
        for warning in &self.warnings {
            writeln!(text, "Warning: {warning}").ok();
        }
        for error in &self.errors {
            writeln!(text, "Error: {error}").ok();
        }
        text
    }

    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "command", self.command.as_str(), true);
        write_json_bool_field(&mut json, "success", self.success);
        write_json_string_field(&mut json, "repositoryPath", &self.repository_path, false);
        write_json_optional_string_field(&mut json, "gitRoot", self.git_root.as_deref());
        write_json_bool_field(&mut json, "isGitRepository", self.is_git_repository);
        write_json_optional_string_field(&mut json, "branch", self.branch.as_deref());
        write_json_optional_string_field(
            &mut json,
            "defaultBranch",
            self.default_branch.as_deref(),
        );
        write_json_bool_field(&mut json, "isProtectedBranch", self.is_protected_branch);
        write_json_string_field(
            &mut json,
            "workingTreeStatus",
            self.working_tree_status.as_str(),
            false,
        );
        write_json_bool_field(&mut json, "isDirty", self.is_dirty);
        write!(
            json,
            ",\"{}\":{}",
            escape_json("dirtyPathCount"),
            self.dirty_paths.len()
        )
        .ok();
        write_json_array_field(&mut json, "dirtyPaths", &self.dirty_paths);
        write_json_string_field(
            &mut json,
            "dbstateProjectStatus",
            self.dbstate_project_status.as_str(),
            false,
        );
        write_json_array_field(&mut json, "missingPaths", &self.missing_paths);
        write_json_array_field(&mut json, "existingPaths", &self.existing_paths);
        write_json_array_field(&mut json, "plannedCreates", &self.planned_creates);
        write_json_array_field(&mut json, "createdPaths", &self.created_paths);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        json.push('}');
        json
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHandoffWorkflow {
    ProjectInit,
    SchemaExport,
    SchemaRelease,
    ReferenceDataExport,
    ReferenceDataReview,
}

impl GitHandoffWorkflow {
    pub(crate) fn branch_segment(self) -> &'static str {
        match self {
            Self::ProjectInit => "project-init",
            Self::SchemaExport => "schema-export",
            Self::SchemaRelease => "schema-release",
            Self::ReferenceDataExport => "reference-data-export",
            Self::ReferenceDataReview => "reference-data-review",
        }
    }

    pub(crate) fn commit_title(self) -> &'static str {
        match self {
            Self::ProjectInit => "chore: initialize DbState project structure",
            Self::SchemaExport => "sync: update schema objects from PostgreSQL",
            Self::SchemaRelease => "review: generate schema release artifacts",
            Self::ReferenceDataExport => "sync: export reference data from PostgreSQL",
            Self::ReferenceDataReview => "review: generate reference-data review script",
        }
    }

    pub(crate) fn commit_body(self) -> &'static str {
        match self {
            Self::ProjectInit => {
                "Initialize local DbState project folders and default reference-data registry.\n\nNo SQL was executed.\nNo database changes were applied."
            }
            Self::SchemaExport => {
                "Updated repository desired-state files from PostgreSQL.\n\nNo database changes were applied."
            }
            Self::SchemaRelease => {
                "Generated review-only schema release artifacts under database/releases/objects/.\n\nNo SQL was executed.\nNo database changes were applied."
            }
            Self::ReferenceDataExport => {
                "Exported selected PostgreSQL reference-data tables to repository YAML files.\n\nUpdated database/reference-data/dbstate.reference-data.yml and/or database/reference-data/tables/.\nNo PostgreSQL data was changed."
            }
            Self::ReferenceDataReview => {
                "Generated review-only reference-data artifacts under database/releases/reference-data/.\n\nIncludes INSERT candidates for repository-only rows, UPDATE candidates for different rows, and manual-review comments for database-only rows.\nNo DELETE statements were generated.\nNo data was applied."
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScopedWriteGuard {
    pub allowed: bool,
    pub current_branch: Option<String>,
    pub default_branch: Option<String>,
    pub protected_branch: bool,
    pub dirty_paths: Vec<String>,
    pub dirty_path_count: usize,
    pub intended_write_paths: Vec<String>,
    pub overlapping_paths: Vec<String>,
    pub suggested_branch_name: String,
    pub suggested_branch_command: String,
    pub suggested_commit_title: String,
    pub suggested_commit_body: String,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn scoped_write_guard(
    root: &Path,
    current_branch: Option<&str>,
    default_branch: Option<&str>,
    intended_write_paths: &[String],
    workflow: GitHandoffWorkflow,
    summary: &str,
) -> ScopedWriteGuard {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut intended: Vec<String> = intended_write_paths
        .iter()
        .map(|path| path.replace('\\', "/"))
        .collect();
    intended.sort();
    intended.dedup();

    for path in &intended {
        if !safe_relative_repo_path(path) {
            errors.push(format!(
                "Write blocked because the target path is not safe: {path}"
            ));
        }
    }

    let branch = current_branch.map(str::to_string);
    let default = default_branch.map(str::to_string);
    let protected = is_protected_branch(current_branch, default_branch);
    let suggested_branch_name = suggested_branch_name(workflow, None, None, summary);
    let suggested_branch_command = format!("git switch -c {suggested_branch_name}");

    if protected {
        let branch_name = current_branch.unwrap_or("unknown");
        errors.push(format!(
            "Write blocked on protected branch: {branch_name}\n\nDbState does not write files on protected branches such as main, master, dev, develop, or the repository default branch.\n\nCreate a working branch first, then run this action again.\n\nRecommended before writing files:\n{suggested_branch_command}\n\nAfter switching branches manually, run the write/generate action again."
        ));
    }

    let dirty_entries = git_dirty_paths(root);
    let dirty_paths: Vec<String> = dirty_entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    let mut overlapping_paths = Vec::new();
    for dirty in &dirty_entries {
        for target in &intended {
            if paths_overlap(target, &dirty.path) {
                overlapping_paths.push(target.clone());
            }
        }
    }
    overlapping_paths.sort();
    overlapping_paths.dedup();

    if !overlapping_paths.is_empty() {
        for path in &overlapping_paths {
            errors.push(format!(
                "Write blocked because the target file has uncommitted changes:\n{path}\n\nCommit, stash, or discard the file before writing."
            ));
        }
    } else if !dirty_paths.is_empty() {
        warnings.push(
            "Your Git worktree has unrelated changes. DbState will write only the selected files. Review all changes before committing."
                .to_string(),
        );
    }

    if errors.is_empty() {
        warnings.push(format!(
            "Git Handoff: suggested manual branch command before writing files: {suggested_branch_command}"
        ));
    }

    ScopedWriteGuard {
        allowed: errors.is_empty(),
        current_branch: branch,
        default_branch: default,
        protected_branch: protected,
        dirty_path_count: dirty_paths.len(),
        dirty_paths,
        intended_write_paths: intended,
        overlapping_paths,
        suggested_branch_name,
        suggested_branch_command,
        suggested_commit_title: workflow.commit_title().to_string(),
        suggested_commit_body: workflow.commit_body().to_string(),
        warnings,
        errors,
    }
}

pub fn suggested_branch_name(
    workflow: GitHandoffWorkflow,
    timestamp: Option<&str>,
    short_id: Option<&str>,
    summary: &str,
) -> String {
    let timestamp = timestamp
        .map(str::to_string)
        .unwrap_or_else(current_branch_timestamp);
    let short_id = short_id
        .map(str::to_string)
        .unwrap_or_else(generate_short_id);
    let slug = safe_summary_slug(summary);
    format!(
        "dbstate/{}/{}-{}-{}",
        workflow.branch_segment(),
        timestamp,
        short_id,
        slug
    )
}

fn paths_overlap(target: &str, dirty: &str) -> bool {
    let target = target.trim_end_matches('/');
    let dirty = dirty.trim_end_matches('/');
    target == dirty
        || dirty
            .strip_prefix(target)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || target
            .strip_prefix(dirty)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn safe_summary_slug(summary: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in summary.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "dbstate-write".to_string()
    } else {
        slug
    }
}

fn generate_short_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{:06x}", (nanos ^ std::process::id() as u128) & 0x00ff_ffff)
}

fn current_branch_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format_timestamp_utc(seconds)
}

fn format_timestamp_utc(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m, d)
}
