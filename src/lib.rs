use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use postgres::{Client, NoTls};
use serde_yaml::{Mapping, Value};

const DEFAULT_REGISTRY: &str = "version: 1\ntables: []\n";
const DEFERRED_OBJECT_TYPES: &[&str] = &[
    "primaryKeys",
    "foreignKeys",
    "uniqueConstraints",
    "checkConstraints",
    "materializedViews",
    "functions",
    "triggers",
    "grants",
    "rlsPolicies",
];

const PROFILE_FILE_NAME: &str = "connection-profiles.json";
const PROFILE_FILE_VERSION: i32 = 1;
const ALLOWED_SSL_MODES: &[&str] = &["disable", "prefer", "require", "verify-ca", "verify-full"];
const FORBIDDEN_PROFILE_FIELDS: &[&str] = &[
    "password",
    "token",
    "secret",
    "url",
    "uri",
    "connectionString",
    "connectionUrl",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    RepoStatus,
    Init,
    InspectPostgres,
    ExportPostgres,
    SyncPostgres,
    ComparePostgres,
    PlanPostgres,
    ReleasePostgres,
    DataComparePostgres,
}

impl CommandKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::RepoStatus => "repo status",
            Self::Init => "init",
            Self::InspectPostgres => "inspect postgres",
            Self::ExportPostgres => "export postgres",
            Self::SyncPostgres => "sync postgres",
            Self::ComparePostgres => "compare postgres",
            Self::PlanPostgres => "plan postgres",
            Self::ReleasePostgres => "release postgres",
            Self::DataComparePostgres => "data-compare postgres",
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandOutput {
    Project(ProjectReport),
    Inspection(InspectionReport),
    Export(ExportReport),
    Sync(SyncReport),
    Compare(CompareReport),
    Plan(PlanReport),
    Release(ReleaseReport),
    DataCompare(ReferenceDataCompareReport),
}

impl CommandOutput {
    pub fn to_text(&self) -> String {
        match self {
            Self::Project(report) => report.to_text(),
            Self::Inspection(report) => report.to_text(),
            Self::Export(report) => report.to_text(),
            Self::Sync(report) => report.to_text(),
            Self::Compare(report) => report.to_text(),
            Self::Plan(report) => report.to_text(),
            Self::Release(report) => report.to_text(),
            Self::DataCompare(report) => report.to_text(),
        }
    }

    pub fn to_json(&self) -> String {
        match self {
            Self::Project(report) => report.to_json(),
            Self::Inspection(report) => report.to_json(),
            Self::Export(report) => report.to_json(),
            Self::Sync(report) => report.to_json(),
            Self::Compare(report) => report.to_json(),
            Self::Plan(report) => report.to_json(),
            Self::Release(report) => report.to_json(),
            Self::DataCompare(report) => report.to_json(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbStateProjectStatus {
    NotGitRepository,
    GitRepositoryWithoutDbStateStructure,
    PartialDbStateStructure,
    CompleteDbStateStructure,
}

impl DbStateProjectStatus {
    fn as_str(self) -> &'static str {
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
    fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub ssl_mode: String,
    pub description: Option<String>,
    pub default_schema: Option<String>,
}

impl ConnectionProfile {
    fn to_json_object(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "name", &self.name, true);
        write_json_string_field(&mut json, "host", &self.host, false);
        write!(json, ",\"{}\":{}", escape_json("port"), self.port).ok();
        write_json_string_field(&mut json, "database", &self.database, false);
        write_json_string_field(&mut json, "username", &self.username, false);
        write_json_string_field(&mut json, "sslMode", &self.ssl_mode, false);
        write_json_optional_string_field(&mut json, "description", self.description.as_deref());
        write_json_optional_string_field(
            &mut json,
            "defaultSchema",
            self.default_schema.as_deref(),
        );
        json.push('}');
        json
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionProfileStore {
    pub profiles: Vec<ConnectionProfile>,
}

#[derive(Debug, Clone)]
struct ResolvedPostgresConnection {
    url: String,
    source: String,
}

#[derive(Debug, Clone)]
struct ConnectionTestReport {
    command: String,
    success: bool,
    database_type: String,
    connection_source: String,
    database_name: Option<String>,
    database_user: Option<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CliResult {
    pub format: OutputFormat,
    pub output: CommandOutput,
    pub exit_code: u8,
}

#[derive(Debug, Clone)]
pub struct ProjectReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub dbstate_project_status: DbStateProjectStatus,
    pub missing_paths: Vec<String>,
    pub existing_paths: Vec<String>,
    pub planned_creates: Vec<String>,
    pub created_paths: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaInfo {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInfo {
    pub schema_name: String,
    pub table_name: String,
    pub table_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfo {
    pub schema_name: String,
    pub table_name: String,
    pub column_name: String,
    pub ordinal_position: i32,
    pub data_type: String,
    pub is_nullable: bool,
    pub has_default: bool,
    pub default_expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInfo {
    pub extension_name: String,
    pub schema_name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumInfo {
    pub schema_name: String,
    pub enum_name: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceInfo {
    pub schema_name: String,
    pub sequence_name: String,
    pub data_type: Option<String>,
    pub start_value: Option<i64>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub increment_by: Option<i64>,
    pub cycle: bool,
    pub cache_size: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexInfo {
    pub schema_name: String,
    pub table_name: String,
    pub index_name: String,
    pub is_unique: bool,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInfo {
    pub schema_name: String,
    pub view_name: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionCounts {
    pub schemas: usize,
    pub tables: usize,
    pub columns: usize,
    pub extensions: usize,
    pub enums: usize,
    pub sequences: usize,
    pub indexes: usize,
    pub views: usize,
}

#[derive(Debug, Clone)]
pub struct InspectionReport {
    pub command: CommandKind,
    pub success: bool,
    pub database_type: String,
    pub inspection_scope: Vec<String>,
    pub schemas: Vec<SchemaInfo>,
    pub tables: Vec<TableInfo>,
    pub columns: Vec<ColumnInfo>,
    pub extensions: Vec<ExtensionInfo>,
    pub enums: Vec<EnumInfo>,
    pub sequences: Vec<SequenceInfo>,
    pub indexes: Vec<IndexInfo>,
    pub views: Vec<ViewInfo>,
    pub counts: InspectionCounts,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExportReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub export_scope: String,
    pub dry_run: bool,
    pub selected_schemas: Vec<String>,
    pub selected_tables: Vec<String>,
    pub planned_files: Vec<String>,
    pub created_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SyncReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub sync_scope: String,
    pub dry_run: bool,
    pub selected_schemas: Vec<String>,
    pub selected_tables: Vec<String>,
    pub added_files: Vec<String>,
    pub changed_files: Vec<String>,
    pub unchanged_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub planned_creates: Vec<String>,
    pub planned_updates: Vec<String>,
    pub created_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompareReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub compare_scope: String,
    pub selected_schemas: Vec<String>,
    pub selected_tables: Vec<String>,
    pub in_sync: Vec<String>,
    pub repo_different: Vec<String>,
    pub repo_only: Vec<String>,
    pub database_only: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PlanReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub plan_scope: String,
    pub selected_schemas: Vec<String>,
    pub selected_tables: Vec<String>,
    pub included_objects: Vec<String>,
    pub excluded_objects: Vec<String>,
    pub plan_items: Vec<PlanItem>,
    pub blocked_items: Vec<PlanItem>,
    pub dependency_warnings: Vec<DependencyWarning>,
    pub compare_summary: CompareSummary,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReleaseReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub release_name: String,
    pub release_scope: String,
    pub dry_run: bool,
    pub selected_schemas: Vec<String>,
    pub selected_tables: Vec<String>,
    pub included_objects: Vec<String>,
    pub excluded_objects: Vec<String>,
    pub plan_items: Vec<PlanItem>,
    pub blocked_items: Vec<PlanItem>,
    pub dependency_warnings: Vec<DependencyWarning>,
    pub planned_artifacts: Vec<String>,
    pub created_artifacts: Vec<String>,
    pub risk_level: String,
    pub risk_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataCompareReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub compare_scope: String,
    pub selected_tables: Vec<String>,
    pub table_results: Vec<ReferenceTableCompareResult>,
    pub counts: ReferenceDataCompareCounts,
    pub in_sync: Vec<String>,
    pub repo_different: Vec<String>,
    pub repo_only: Vec<String>,
    pub database_only: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceTableCompareResult {
    pub table_name: String,
    pub key_columns: Vec<String>,
    pub compared_columns: Vec<String>,
    pub ignored_columns: Vec<String>,
    pub masked_columns: Vec<String>,
    pub row_counts: ReferenceDataCompareCounts,
    pub row_results: Vec<ReferenceRowCompareResult>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceRowCompareResult {
    pub table_name: String,
    pub row_key: String,
    pub classification: String,
    pub changed_columns: Vec<String>,
    pub masked_columns: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceDataCompareCounts {
    pub in_sync: usize,
    pub repo_different: usize,
    pub repo_only: usize,
    pub database_only: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone)]
pub struct PlanItem {
    pub object_ref: String,
    pub object_type: String,
    pub relative_path: String,
    pub compare_classification: String,
    pub plan_intent: String,
    pub selected: bool,
    pub blocked: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DependencyWarning {
    pub warning_type: String,
    pub object_ref: String,
    pub required_object_ref: Option<String>,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone)]
pub struct CompareSummary {
    pub in_sync: usize,
    pub repo_different: usize,
    pub repo_only: usize,
    pub database_only: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Copy)]
struct ExpectedPath {
    relative: &'static str,
    kind: PathKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathKind {
    Directory,
    File,
}

const EXPECTED_PATHS: &[ExpectedPath] = &[
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
];

pub fn run_cli(
    args: &[String],
    current_dir: Result<&Path, &std::io::Error>,
) -> Result<CliResult, String> {
    let cwd = current_dir.map_err(|error| format!("Could not read current directory: {error}"))?;
    let parsed = ParsedArgs::parse(args)?;

    match parsed.command {
        CommandKind::RepoStatus => {
            let report = status_report(cwd, CommandKind::RepoStatus);
            let exit_code = if report.is_git_repository { 0 } else { 2 };
            Ok(CliResult {
                format: parsed.format,
                output: CommandOutput::Project(report),
                exit_code,
            })
        }
        CommandKind::Init => {
            let report = init_project(cwd, parsed.dry_run)?;
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format: parsed.format,
                output: CommandOutput::Project(report),
                exit_code,
            })
        }
        CommandKind::InspectPostgres => {
            let report = inspect_postgres_scoped_command(
                parsed.url,
                env::var("DBSTATE_POSTGRES_URL").ok(),
                parsed.schema,
                parsed.table,
            );
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format: parsed.format,
                output: CommandOutput::Inspection(report),
                exit_code,
            })
        }
        CommandKind::ExportPostgres => {
            let format = parsed.format;
            let report = export_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Export(report),
                exit_code,
            })
        }
        CommandKind::SyncPostgres => {
            let format = parsed.format;
            let report = sync_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Sync(report),
                exit_code,
            })
        }
        CommandKind::ComparePostgres => {
            let format = parsed.format;
            let report = compare_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Compare(report),
                exit_code,
            })
        }
        CommandKind::PlanPostgres => {
            let format = parsed.format;
            let report = plan_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Plan(report),
                exit_code,
            })
        }
        CommandKind::ReleasePostgres => {
            let format = parsed.format;
            let report = release_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Release(report),
                exit_code,
            })
        }
        CommandKind::DataComparePostgres => {
            let format = parsed.format;
            let report = data_compare_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::DataCompare(report),
                exit_code,
            })
        }
    }
}

mod service;
mod ui;

pub use service::{
    load_connection_profiles, parse_service_args, run_service, service_response,
    service_route_definitions, service_usage, ServiceConfig, ServiceHttpResponse,
};
pub use ui::{app_css, app_js, index_html, ui_css, ui_html, ui_js};

#[cfg(test)]
use service::{
    load_connection_profiles_from_path, normalize_local_path_input, parse_connection_profile,
    resolve_service_postgres_connection, save_connection_profiles, validate_connection_profile,
    validate_unique_profile_names,
};
#[derive(Debug, Clone)]
struct ParsedArgs {
    command: CommandKind,
    format: OutputFormat,
    dry_run: bool,
    url: Option<String>,
    schema: Option<String>,
    table: Option<String>,
    all: bool,
    includes: Vec<String>,
    excludes: Vec<String>,
    release_name: Option<String>,
}

impl ParsedArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        if args.is_empty() {
            return Err(usage());
        }

        let mut format = OutputFormat::Text;
        let mut dry_run = false;
        let mut url = None;
        let mut schema = None;
        let mut table = None;
        let mut all = false;
        let mut includes = Vec::new();
        let mut excludes = Vec::new();
        let mut release_name = None;
        let mut positional = Vec::new();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--format" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--format requires a value".to_string())?;
                    format = match value.as_str() {
                        "json" => OutputFormat::Json,
                        "text" => OutputFormat::Text,
                        _ => return Err("--format must be either json or text".to_string()),
                    };
                    index += 2;
                }
                "--json" => {
                    format = OutputFormat::Json;
                    index += 1;
                }
                "--dry-run" => {
                    dry_run = true;
                    index += 1;
                }
                "--url" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--url requires a value".to_string())?;
                    url = Some(value.to_string());
                    index += 2;
                }
                "--schema" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--schema requires a value".to_string())?;
                    schema = Some(value.to_string());
                    index += 2;
                }
                "--table" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--table requires a value".to_string())?;
                    table = Some(value.to_string());
                    index += 2;
                }
                "--all" => {
                    all = true;
                    index += 1;
                }
                "--include" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--include requires a value".to_string())?;
                    includes.push(value.to_string());
                    index += 2;
                }
                "--exclude" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--exclude requires a value".to_string())?;
                    excludes.push(value.to_string());
                    index += 2;
                }
                "--name" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--name requires a value".to_string())?;
                    release_name = Some(value.to_string());
                    index += 2;
                }
                "--help" | "-h" => return Err(usage()),
                value if value.starts_with('-') => {
                    return Err(format!("Unknown option: {value}"));
                }
                value => {
                    positional.push(value.to_string());
                    index += 1;
                }
            }
        }

        let command = match positional.as_slice() {
            [command] if command == "init" => CommandKind::Init,
            [repo, command] if repo == "repo" && command == "status" => CommandKind::RepoStatus,
            [inspect, database] if inspect == "inspect" && database == "postgres" => {
                CommandKind::InspectPostgres
            }
            [export, database] if export == "export" && database == "postgres" => {
                CommandKind::ExportPostgres
            }
            [sync, database] if sync == "sync" && database == "postgres" => {
                CommandKind::SyncPostgres
            }
            [compare, database] if compare == "compare" && database == "postgres" => {
                CommandKind::ComparePostgres
            }
            [plan, database] if plan == "plan" && database == "postgres" => {
                CommandKind::PlanPostgres
            }
            [release, database] if release == "release" && database == "postgres" => {
                CommandKind::ReleasePostgres
            }
            [data_compare, database]
                if data_compare == "data-compare" && database == "postgres" =>
            {
                CommandKind::DataComparePostgres
            }
            _ => return Err(usage()),
        };

        if dry_run
            && command != CommandKind::Init
            && command != CommandKind::ExportPostgres
            && command != CommandKind::SyncPostgres
            && command != CommandKind::ReleasePostgres
        {
            return Err(
                "--dry-run is only supported for dbstate init, dbstate export postgres, dbstate sync postgres, and dbstate release postgres"
                    .to_string(),
            );
        }

        if url.is_some()
            && command != CommandKind::InspectPostgres
            && command != CommandKind::ExportPostgres
            && command != CommandKind::SyncPostgres
            && command != CommandKind::ComparePostgres
            && command != CommandKind::PlanPostgres
            && command != CommandKind::ReleasePostgres
            && command != CommandKind::DataComparePostgres
        {
            return Err(
                "--url is only supported for dbstate inspect postgres, dbstate export postgres, dbstate sync postgres, dbstate compare postgres, dbstate plan postgres, dbstate release postgres, and dbstate data-compare postgres"
                    .to_string(),
            );
        }

        if (schema.is_some() || table.is_some() || all)
            && command != CommandKind::InspectPostgres
            && command != CommandKind::ExportPostgres
            && command != CommandKind::SyncPostgres
            && command != CommandKind::ComparePostgres
            && command != CommandKind::PlanPostgres
            && command != CommandKind::ReleasePostgres
            && command != CommandKind::DataComparePostgres
        {
            return Err(
                "--schema, --table, and --all are only supported for dbstate inspect postgres, dbstate export postgres, dbstate sync postgres, dbstate compare postgres, dbstate plan postgres, dbstate release postgres, and dbstate data-compare postgres"
                    .to_string(),
            );
        }

        if schema.is_some() && command == CommandKind::DataComparePostgres {
            return Err(
                "--schema is not supported for dbstate data-compare postgres. Use --table <schema.table> or --all."
                    .to_string(),
            );
        }

        if (!includes.is_empty() || !excludes.is_empty())
            && command != CommandKind::PlanPostgres
            && command != CommandKind::ReleasePostgres
        {
            return Err(
                "--include and --exclude are only supported for dbstate plan postgres and dbstate release postgres".to_string(),
            );
        }

        if release_name.is_some() && command != CommandKind::ReleasePostgres {
            return Err("--name is only supported for dbstate release postgres".to_string());
        }

        Ok(Self {
            command,
            format,
            dry_run,
            url,
            schema,
            table,
            all,
            includes,
            excludes,
            release_name,
        })
    }
}

pub fn usage() -> String {
    "Usage:\n  dbstate repo status [--format json|--json]\n  dbstate init [--dry-run] [--format json|--json]\n  dbstate inspect postgres [--url <postgres-url>] [--all | --schema <schema> | --table <schema.table>] [--format json|--json]\n  dbstate export postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--dry-run] [--format json|--json]\n  dbstate sync postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--dry-run] [--format json|--json]\n  dbstate compare postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--format json|--json]\n  dbstate plan postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--include <object-ref>] [--exclude <object-ref>] [--format json|--json]\n  dbstate release postgres (--all | --schema <schema> | --table <schema.table>) --name <release-name> [--url <postgres-url>] [--include <object-ref>] [--exclude <object-ref>] [--dry-run] [--format json|--json]\n  dbstate data-compare postgres (--all | --table <schema.table>) [--url <postgres-url>] [--format json|--json]\n  dbstate serve [--host <host>] [--port <port>] [--format json|--json]".to_string()
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
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
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
    report.working_tree_status = git_working_tree_status(&root);
    report.is_dirty = report.working_tree_status == WorkingTreeStatus::Dirty;
    if report.is_dirty {
        report
            .warnings
            .push("Working tree has changes. Read-only status is allowed, but initialization is blocked until the working tree is clean.".to_string());
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

    if report.is_dirty && !dry_run {
        report.success = false;
        report.errors.push(
            "Initialization is blocked because the working tree has changes. Run dbstate repo status, commit/stash changes, or use --dry-run.".to_string(),
        );
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

pub fn inspect_postgres_command(
    cli_url: Option<String>,
    env_url: Option<String>,
) -> InspectionReport {
    inspect_postgres_scoped_command(cli_url, env_url, None, None)
}

fn inspect_postgres_scoped_command(
    cli_url: Option<String>,
    env_url: Option<String>,
    schema: Option<String>,
    table: Option<String>,
) -> InspectionReport {
    let mut report = empty_inspection_report(CommandKind::InspectPostgres);

    let Some(connection_url) = resolve_postgres_url(cli_url, env_url) else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };
    if !is_postgres_connection_url(&connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match inspect_postgres(&connection_url) {
        Ok(inventory) => {
            report.schemas = inventory.schemas;
            report.tables = inventory.tables;
            report.columns = inventory.columns;
            report.extensions = inventory.extensions;
            report.enums = inventory.enums;
            report.sequences = inventory.sequences;
            report.indexes = inventory.indexes;
            report.views = inventory.views;
            if let Err(error) = apply_inspection_scope(&mut report, schema, table) {
                report.success = false;
                report.errors.push(error);
                return report;
            }
            report.counts = InspectionCounts {
                schemas: report.schemas.len(),
                tables: report.tables.len(),
                columns: report.columns.len(),
                extensions: report.extensions.len(),
                enums: report.enums.len(),
                sequences: report.sequences.len(),
                indexes: report.indexes.len(),
                views: report.views.len(),
            };
            report.success = true;
        }
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
        }
    }

    report
}

fn apply_inspection_scope(
    report: &mut InspectionReport,
    schema: Option<String>,
    table: Option<String>,
) -> Result<(), String> {
    if let Some(schema) = schema {
        if schema.trim().is_empty() {
            return Err("--schema cannot be empty.".to_string());
        }
        if !report.schemas.iter().any(|item| item.name == schema) {
            return Err(format!(
                "Selected schema '{schema}' was not found in the PostgreSQL inventory."
            ));
        }
        report.schemas.retain(|item| item.name == schema);
        report.tables.retain(|item| item.schema_name == schema);
        report.columns.retain(|item| item.schema_name == schema);
        report.enums.retain(|item| item.schema_name == schema);
        report.sequences.retain(|item| item.schema_name == schema);
        report.indexes.retain(|item| item.schema_name == schema);
        report.views.retain(|item| item.schema_name == schema);
        report.inspection_scope = vec![format!("schema:{schema}")];
        return Ok(());
    }

    if let Some(table) = table {
        let Some((schema, table_name)) = table.split_once('.') else {
            return Err(
                "--table must use schema-qualified form such as public.example_table.".to_string(),
            );
        };
        if schema.trim().is_empty() || table_name.trim().is_empty() {
            return Err(
                "--table must use schema-qualified form such as public.example_table.".to_string(),
            );
        }
        if !report
            .tables
            .iter()
            .any(|item| item.schema_name == schema && item.table_name == table_name)
        {
            return Err(format!(
                "Selected table '{schema}.{table_name}' was not found in the PostgreSQL inventory."
            ));
        }
        report.schemas.retain(|item| item.name == schema);
        report
            .tables
            .retain(|item| item.schema_name == schema && item.table_name == table_name);
        report
            .columns
            .retain(|item| item.schema_name == schema && item.table_name == table_name);
        report.enums.clear();
        report.sequences.clear();
        report.views.clear();
        report
            .indexes
            .retain(|item| item.schema_name == schema && item.table_name == table_name);
        report.inspection_scope = vec![format!("table:{schema}.{table_name}")];
    }

    Ok(())
}

fn resolve_postgres_url(cli_url: Option<String>, env_url: Option<String>) -> Option<String> {
    cli_url
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env_url.filter(|value| !value.trim().is_empty()))
}

fn is_postgres_connection_url(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://")
}

fn invalid_postgres_url_message() -> String {
    "Invalid PostgreSQL connection URL. Provide a postgres:// or postgresql:// URL.".to_string()
}

#[derive(Debug, Clone)]
pub struct PostgresInventory {
    pub schemas: Vec<SchemaInfo>,
    pub tables: Vec<TableInfo>,
    pub columns: Vec<ColumnInfo>,
    pub extensions: Vec<ExtensionInfo>,
    pub enums: Vec<EnumInfo>,
    pub sequences: Vec<SequenceInfo>,
    pub indexes: Vec<IndexInfo>,
    pub views: Vec<ViewInfo>,
}

pub fn inspect_postgres(connection_url: &str) -> Result<PostgresInventory, String> {
    let mut client = Client::connect(connection_url, NoTls).map_err(|_| {
        "PostgreSQL connection failed. Verify the session-only connection URL, credentials, network, and database availability.".to_string()
    })?;

    let schema_rows = client
        .query(
            "SELECT nspname
             FROM pg_catalog.pg_namespace
             WHERE nspname <> 'pg_catalog'
               AND nspname <> 'information_schema'
               AND nspname NOT LIKE 'pg_toast%'
               AND nspname NOT LIKE 'pg_%'
             ORDER BY nspname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading schemas.".to_string())?;

    let table_rows = client
        .query(
            "SELECT n.nspname,
                    c.relname,
                    CASE c.relkind
                        WHEN 'r' THEN 'BASE TABLE'
                        WHEN 'p' THEN 'PARTITIONED TABLE'
                        ELSE c.relkind::text
                    END
             FROM pg_catalog.pg_class c
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relkind IN ('r', 'p')
               AND n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, c.relname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading tables.".to_string())?;

    let column_rows = client
        .query(
            "SELECT n.nspname,
                    c.relname,
                    a.attname,
                    a.attnum::int4,
                    pg_catalog.format_type(a.atttypid, a.atttypmod),
                    NOT a.attnotnull,
                    pg_catalog.pg_get_expr(ad.adbin, ad.adrelid) IS NOT NULL,
                    pg_catalog.pg_get_expr(ad.adbin, ad.adrelid)
             FROM pg_catalog.pg_attribute a
             JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
             LEFT JOIN pg_catalog.pg_attrdef ad
               ON ad.adrelid = a.attrelid
              AND ad.adnum = a.attnum
             WHERE a.attnum > 0
               AND NOT a.attisdropped
               AND c.relkind IN ('r', 'p')
               AND n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, c.relname, a.attnum",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading columns.".to_string())?;

    let extension_rows = client
        .query(
            "SELECT e.extname::text, n.nspname::text, e.extversion::text
             FROM pg_catalog.pg_extension e
             LEFT JOIN pg_catalog.pg_namespace n ON n.oid = e.extnamespace
             WHERE e.extname <> 'plpgsql'
             ORDER BY e.extname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading extensions.".to_string())?;

    let enum_rows = client
        .query(
            "SELECT n.nspname::text, t.typname::text, e.enumlabel::text
             FROM pg_catalog.pg_type t
             JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
             JOIN pg_catalog.pg_enum e ON e.enumtypid = t.oid
             WHERE n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, t.typname, e.enumsortorder",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading enums.".to_string())?;

    let sequence_rows = client
        .query(
            "SELECT schemaname::text,
                    sequencename::text,
                    data_type::text,
                    start_value,
                    min_value,
                    max_value,
                    increment_by,
                    cycle,
                    cache_size
             FROM pg_catalog.pg_sequences
             WHERE schemaname <> 'pg_catalog'
               AND schemaname <> 'information_schema'
               AND schemaname NOT LIKE 'pg_toast%'
               AND schemaname NOT LIKE 'pg_%'
             ORDER BY schemaname, sequencename",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading sequences.".to_string())?;

    let index_rows = client
        .query(
            "SELECT ns.nspname::text,
                    tbl.relname::text,
                    idx.relname::text,
                    i.indisunique,
                    pg_catalog.pg_get_indexdef(idx.oid)
             FROM pg_catalog.pg_index i
             JOIN pg_catalog.pg_class idx ON idx.oid = i.indexrelid
             JOIN pg_catalog.pg_class tbl ON tbl.oid = i.indrelid
             JOIN pg_catalog.pg_namespace ns ON ns.oid = tbl.relnamespace
             WHERE tbl.relkind IN ('r', 'p')
               AND ns.nspname <> 'pg_catalog'
               AND ns.nspname <> 'information_schema'
               AND ns.nspname NOT LIKE 'pg_toast%'
               AND ns.nspname NOT LIKE 'pg_%'
               AND NOT EXISTS (
                   SELECT 1 FROM pg_catalog.pg_constraint c
                   WHERE c.conindid = idx.oid
               )
             ORDER BY ns.nspname, tbl.relname, idx.relname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading indexes.".to_string())?;

    let view_rows = client
        .query(
            "SELECT n.nspname::text,
                    c.relname::text,
                    pg_catalog.pg_get_viewdef(c.oid, true)::text
             FROM pg_catalog.pg_class c
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relkind = 'v'
               AND n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, c.relname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading views.".to_string())?;

    let schemas = schema_rows
        .into_iter()
        .map(|row| SchemaInfo { name: row.get(0) })
        .collect();

    let tables = table_rows
        .into_iter()
        .map(|row| TableInfo {
            schema_name: row.get(0),
            table_name: row.get(1),
            table_type: row.get(2),
        })
        .collect();

    let columns = column_rows
        .into_iter()
        .map(|row| ColumnInfo {
            schema_name: row.get(0),
            table_name: row.get(1),
            column_name: row.get(2),
            ordinal_position: row.get(3),
            data_type: row.get(4),
            is_nullable: row.get(5),
            has_default: row.get(6),
            default_expression: row.get(7),
        })
        .collect();

    let mut extensions = Vec::new();
    for row in extension_rows {
        extensions.push(ExtensionInfo {
            extension_name: try_get_catalog_string(&row, 0, "extension name")?,
            schema_name: try_get_catalog_optional_string(&row, 1, "extension schema")?,
            version: try_get_catalog_optional_string(&row, 2, "extension version")?,
        });
    }

    let mut enum_map: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for row in enum_rows {
        enum_map
            .entry((
                try_get_catalog_string(&row, 0, "enum schema")?,
                try_get_catalog_string(&row, 1, "enum name")?,
            ))
            .or_default()
            .push(try_get_catalog_string(&row, 2, "enum label")?);
    }
    let enums = enum_map
        .into_iter()
        .map(|((schema_name, enum_name), labels)| EnumInfo {
            schema_name,
            enum_name,
            labels,
        })
        .collect();

    let mut sequences = Vec::new();
    for row in sequence_rows {
        sequences.push(SequenceInfo {
            schema_name: try_get_catalog_string(&row, 0, "sequence schema")?,
            sequence_name: try_get_catalog_string(&row, 1, "sequence name")?,
            data_type: try_get_catalog_optional_string(&row, 2, "sequence data type")?,
            start_value: try_get_catalog_optional_i64(&row, 3, "sequence start value")?,
            min_value: try_get_catalog_optional_i64(&row, 4, "sequence min value")?,
            max_value: try_get_catalog_optional_i64(&row, 5, "sequence max value")?,
            increment_by: try_get_catalog_optional_i64(&row, 6, "sequence increment")?,
            cycle: try_get_catalog_bool(&row, 7, "sequence cycle")?,
            cache_size: try_get_catalog_optional_i64(&row, 8, "sequence cache size")?,
        });
    }

    let mut indexes = Vec::new();
    for row in index_rows {
        indexes.push(IndexInfo {
            schema_name: try_get_catalog_string(&row, 0, "index schema")?,
            table_name: try_get_catalog_string(&row, 1, "index table")?,
            index_name: try_get_catalog_string(&row, 2, "index name")?,
            is_unique: try_get_catalog_bool(&row, 3, "index uniqueness")?,
            definition: try_get_catalog_string(&row, 4, "index definition")?,
        });
    }

    let mut views = Vec::new();
    for row in view_rows {
        views.push(ViewInfo {
            schema_name: try_get_catalog_string(&row, 0, "view schema")?,
            view_name: try_get_catalog_string(&row, 1, "view name")?,
            definition: try_get_catalog_string(&row, 2, "view definition")?,
        });
    }

    Ok(PostgresInventory {
        schemas,
        tables,
        columns,
        extensions,
        enums,
        sequences,
        indexes,
        views,
    })
}

fn try_get_catalog_string(
    row: &postgres::Row,
    index: usize,
    field: &str,
) -> Result<String, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn try_get_catalog_optional_string(
    row: &postgres::Row,
    index: usize,
    field: &str,
) -> Result<Option<String>, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn try_get_catalog_optional_i64(
    row: &postgres::Row,
    index: usize,
    field: &str,
) -> Result<Option<i64>, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn try_get_catalog_bool(row: &postgres::Row, index: usize, field: &str) -> Result<bool, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn export_postgres_command(cwd: &Path, parsed: ParsedArgs) -> ExportReport {
    let mut report = empty_export_report(parsed.dry_run);
    let selection = match ExportSelection::from_options(
        parsed.all,
        parsed.schema.clone(),
        parsed.table.clone(),
    ) {
        Ok(selection) => selection,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    report.export_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();

    let Some(connection_url) =
        resolve_postgres_url(parsed.url, env::var("DBSTATE_POSTGRES_URL").ok())
    else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };
    if !is_postgres_connection_url(&connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match inspect_postgres(&connection_url) {
        Ok(inventory) => {
            export_postgres_with_inventory(cwd, &inventory, &selection, parsed.dry_run)
        }
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
            report
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportSelection {
    All,
    Schema(String),
    Table { schema: String, table: String },
}

impl ExportSelection {
    fn from_options(
        all: bool,
        schema: Option<String>,
        table: Option<String>,
    ) -> Result<Self, String> {
        let selected = if all { 1 } else { 0 }
            + if schema.is_some() { 1 } else { 0 }
            + if table.is_some() { 1 } else { 0 };
        if selected == 0 {
            return Err("Selection is required. Provide --schema, --table, or --all.".to_string());
        }
        if selected > 1 {
            return Err(
                "Use only one export selection option: --schema, --table, or --all.".to_string(),
            );
        }
        if all {
            return Ok(Self::All);
        }
        if let Some(schema) = schema {
            if schema.trim().is_empty() {
                return Err("--schema cannot be empty.".to_string());
            }
            return Ok(Self::Schema(schema));
        }

        let table = table.expect("table selection exists");
        let Some((schema, table)) = table.split_once('.') else {
            return Err(
                "--table must use schema-qualified form such as public.example_table.".to_string(),
            );
        };
        if schema.trim().is_empty() || table.trim().is_empty() {
            return Err(
                "--table must use schema-qualified form such as public.example_table.".to_string(),
            );
        }
        Ok(Self::Table {
            schema: schema.to_string(),
            table: table.to_string(),
        })
    }

    fn scope_name(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::Schema(schema) => format!("schema:{schema}"),
            Self::Table { schema, table } => format!("table:{schema}.{table}"),
        }
    }

    fn selected_schemas(&self) -> Vec<String> {
        match self {
            Self::Schema(schema) => vec![schema.clone()],
            _ => Vec::new(),
        }
    }

    fn selected_tables(&self) -> Vec<String> {
        match self {
            Self::Table { schema, table } => vec![format!("{schema}.{table}")],
            _ => Vec::new(),
        }
    }
}

pub fn export_postgres_with_inventory(
    cwd: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
    dry_run: bool,
) -> ExportReport {
    let mut report = empty_export_report(dry_run);
    report.export_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();

    let project = status_report(cwd, CommandKind::ExportPostgres);
    report.repository_path = project.repository_path.clone();
    report.git_root = project.git_root.clone();
    report.is_git_repository = project.is_git_repository;
    report.branch = project.branch.clone();
    report.working_tree_status = project.working_tree_status;
    report.is_dirty = project.is_dirty;
    if !project.is_git_repository {
        report
            .errors
            .push("Current path is not inside a Git repository.".to_string());
        return report;
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return report;
    }
    if project.is_dirty && !dry_run {
        report.errors.push(
            "Export is blocked because the working tree has changes. Commit/stash changes or use --dry-run."
                .to_string(),
        );
        return report;
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    let plan = match plan_export(&root, inventory, selection) {
        Ok(plan) => plan,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    report.warnings = plan.warnings;
    report.planned_files = plan
        .planned_files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect();
    report.skipped_files = plan.skipped_files.clone();

    if dry_run {
        report.success = report.errors.is_empty();
        return report;
    }

    for file in plan.planned_files {
        let target = root.join(&file.relative_path);
        if target.exists() {
            report.skipped_files.push(file.relative_path);
            continue;
        }
        if let Some(parent) = target.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                report.errors.push(format!(
                    "Could not create parent directory for {}: {error}",
                    file.relative_path
                ));
                continue;
            }
        }
        if let Err(error) = fs::write(&target, file.content) {
            report
                .errors
                .push(format!("Could not write {}: {error}", file.relative_path));
            continue;
        }
        report.created_files.push(file.relative_path);
    }

    report.success = report.errors.is_empty();
    report
}

#[derive(Debug, Clone)]
struct ExportPlan {
    planned_files: Vec<PlannedFile>,
    skipped_files: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct PlannedFile {
    relative_path: String,
    content: String,
}

fn plan_export(
    root: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
) -> Result<ExportPlan, String> {
    let mut warnings = Vec::new();
    match selection {
        ExportSelection::All => {}
        ExportSelection::Schema(schema) => {
            if !inventory
                .schemas
                .iter()
                .any(|candidate| candidate.name == *schema)
            {
                return Err(format!(
                    "Selected schema '{schema}' was not found in the PostgreSQL inventory."
                ));
            }
        }
        ExportSelection::Table { schema, table } => {
            let Some(selected_table) = inventory.tables.iter().find(|candidate| {
                candidate.schema_name == *schema && candidate.table_name == *table
            }) else {
                return Err(format!(
                    "Selected table '{schema}.{table}' was not found in the PostgreSQL inventory."
                ));
            };
            if selected_table.table_type != "BASE TABLE" {
                return Err(format!(
                    "Selected table '{schema}.{table}' is not an ordinary/base table and is deferred for Slice 3."
                ));
            }
            let schema_path = schema_file_path(schema)?;
            if !root.join(&schema_path).is_file() {
                warnings.push(format!(
                    "Selected table '{schema}.{table}' is exported without its schema object file. Export --schema {schema} or --all if the schema file is needed."
                ));
            }
        }
    }

    let mut planned_files = Vec::new();
    let mut skipped_files = Vec::new();
    let database_objects = render_database_objects_for_selection(root, inventory, selection)?;
    for object in database_objects.values() {
        let relative_path = object.relative_path.clone();
        ensure_database_object_path(&relative_path)?;
        if root.join(&relative_path).exists() {
            skipped_files.push(relative_path);
            continue;
        }
        planned_files.push(PlannedFile {
            relative_path,
            content: object.content.clone(),
        });
    }

    Ok(ExportPlan {
        planned_files,
        skipped_files,
        warnings,
    })
}

fn schema_file_path(schema: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/schemas/{}.sql",
        safe_file_component(schema)?
    ))
}

fn table_file_path(schema: &str, table: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/tables/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(table)?
    ))
}

fn extension_file_path(extension: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/extensions/{}.sql",
        safe_file_component(extension)?
    ))
}

fn enum_file_path(schema: &str, enum_name: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/enums/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(enum_name)?
    ))
}

fn sequence_file_path(schema: &str, sequence: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/sequences/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(sequence)?
    ))
}

fn index_file_path(schema: &str, table: &str, index: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/indexes/{}.{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(table)?,
        safe_file_component(index)?
    ))
}

fn view_file_path(schema: &str, view: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/views/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(view)?
    ))
}

fn safe_file_component(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
    {
        return Err(format!(
            "Unsafe PostgreSQL object name for file path: {trimmed}"
        ));
    }
    Ok(trimmed.to_string())
}

fn ensure_database_object_path(relative_path: &str) -> Result<(), String> {
    if relative_path.starts_with("database/objects/") {
        Ok(())
    } else {
        Err(format!(
            "Refusing to write outside database/objects/: {relative_path}"
        ))
    }
}

pub fn quote_postgres_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub fn render_schema_sql(schema: &str) -> String {
    format!(
        "-- DbState PostgreSQL desired-state object\n-- Object type: schema\n-- Object name: {schema}\n\nCREATE SCHEMA {};\n",
        quote_postgres_identifier(schema)
    )
}

pub fn render_table_sql(schema: &str, table: &str, columns: &[ColumnInfo]) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: table").ok();
    writeln!(sql, "-- Object name: {schema}.{table}").ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE TABLE {}.{} (",
        quote_postgres_identifier(schema),
        quote_postgres_identifier(table)
    )
    .ok();

    let mut sorted_columns = columns.to_vec();
    sorted_columns.sort_by_key(|column| column.ordinal_position);
    for (index, column) in sorted_columns.iter().enumerate() {
        let comma = if index + 1 == sorted_columns.len() {
            ""
        } else {
            ","
        };
        write!(
            sql,
            "    {} {}",
            quote_postgres_identifier(&column.column_name),
            column.data_type
        )
        .ok();
        if column.has_default {
            if let Some(default_expression) = &column.default_expression {
                write!(sql, " DEFAULT {default_expression}").ok();
            }
        }
        if !column.is_nullable {
            write!(sql, " NOT NULL").ok();
        }
        writeln!(sql, "{comma}").ok();
    }
    writeln!(sql, ");").ok();
    sql
}

pub fn render_extension_sql(extension: &ExtensionInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: extension").ok();
    writeln!(sql, "-- Object name: {}", extension.extension_name).ok();
    if let Some(schema) = &extension.schema_name {
        writeln!(sql, "-- Extension schema: {schema}").ok();
    }
    if let Some(version) = &extension.version {
        writeln!(sql, "-- Extension version observed: {version}").ok();
    }
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE EXTENSION IF NOT EXISTS {};",
        quote_postgres_identifier(&extension.extension_name)
    )
    .ok();
    sql
}

pub fn render_enum_sql(enum_info: &EnumInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: enum").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}",
        enum_info.schema_name, enum_info.enum_name
    )
    .ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE TYPE {}.{} AS ENUM (",
        quote_postgres_identifier(&enum_info.schema_name),
        quote_postgres_identifier(&enum_info.enum_name)
    )
    .ok();
    for (index, label) in enum_info.labels.iter().enumerate() {
        let comma = if index + 1 == enum_info.labels.len() {
            ""
        } else {
            ","
        };
        writeln!(sql, "    '{}'{comma}", label.replace('\'', "''")).ok();
    }
    writeln!(sql, ");").ok();
    sql
}

pub fn render_sequence_sql(sequence: &SequenceInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: sequence").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}",
        sequence.schema_name, sequence.sequence_name
    )
    .ok();
    writeln!(
        sql,
        "-- Owned-by relationship not captured in Private Beta."
    )
    .ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE SEQUENCE {}.{}",
        quote_postgres_identifier(&sequence.schema_name),
        quote_postgres_identifier(&sequence.sequence_name)
    )
    .ok();
    if let Some(data_type) = &sequence.data_type {
        writeln!(sql, "    AS {data_type}").ok();
    }
    if let Some(value) = sequence.start_value {
        writeln!(sql, "    START WITH {value}").ok();
    }
    if let Some(value) = sequence.increment_by {
        writeln!(sql, "    INCREMENT BY {value}").ok();
    }
    if let Some(value) = sequence.min_value {
        writeln!(sql, "    MINVALUE {value}").ok();
    }
    if let Some(value) = sequence.max_value {
        writeln!(sql, "    MAXVALUE {value}").ok();
    }
    if let Some(value) = sequence.cache_size {
        writeln!(sql, "    CACHE {value}").ok();
    }
    if sequence.cycle {
        writeln!(sql, "    CYCLE").ok();
    } else {
        writeln!(sql, "    NO CYCLE").ok();
    }
    sql.push_str(";\n");
    sql
}

pub fn render_index_sql(index: &IndexInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: index").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}.{}",
        index.schema_name, index.table_name, index.index_name
    )
    .ok();
    writeln!(sql).ok();
    let definition = index.definition.trim().trim_end_matches(';');
    writeln!(sql, "{definition};").ok();
    sql
}

pub fn render_view_sql(view: &ViewInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: view").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}",
        view.schema_name, view.view_name
    )
    .ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE VIEW {}.{} AS",
        quote_postgres_identifier(&view.schema_name),
        quote_postgres_identifier(&view.view_name)
    )
    .ok();
    writeln!(sql, "{};", view.definition.trim().trim_end_matches(';')).ok();
    sql
}

fn sync_postgres_command(cwd: &Path, parsed: ParsedArgs) -> SyncReport {
    let mut report = empty_sync_report(parsed.dry_run);
    let selection = match ExportSelection::from_options(
        parsed.all,
        parsed.schema.clone(),
        parsed.table.clone(),
    ) {
        Ok(selection) => selection,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    report.sync_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();

    let Some(connection_url) =
        resolve_postgres_url(parsed.url, env::var("DBSTATE_POSTGRES_URL").ok())
    else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };
    if !is_postgres_connection_url(&connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match inspect_postgres(&connection_url) {
        Ok(inventory) => sync_postgres_with_inventory(cwd, &inventory, &selection, parsed.dry_run),
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
            report
        }
    }
}

pub fn sync_postgres_with_inventory(
    cwd: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
    dry_run: bool,
) -> SyncReport {
    let mut report = empty_sync_report(dry_run);
    report.sync_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();

    let project = status_report(cwd, CommandKind::SyncPostgres);
    report.repository_path = project.repository_path.clone();
    report.git_root = project.git_root.clone();
    report.is_git_repository = project.is_git_repository;
    report.branch = project.branch.clone();
    report.working_tree_status = project.working_tree_status;
    report.is_dirty = project.is_dirty;
    if !project.is_git_repository {
        report
            .errors
            .push("Current path is not inside a Git repository.".to_string());
        return report;
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return report;
    }
    if project.is_dirty && !dry_run {
        report.errors.push(
            "Synchronization is blocked because the working tree has changes. Commit/stash changes or use --dry-run."
                .to_string(),
        );
        return report;
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    let plan = match plan_sync(&root, inventory, selection) {
        Ok(plan) => plan,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    report.added_files = plan.added_files.clone();
    report.changed_files = plan.changed_files.clone();
    report.unchanged_files = plan.unchanged_files.clone();
    report.skipped_files = plan.skipped_files.clone();
    report.planned_creates = plan.planned_creates.clone();
    report.planned_updates = plan.planned_updates.clone();
    report.warnings = plan.warnings.clone();

    if dry_run {
        report.success = report.errors.is_empty();
        return report;
    }

    for write in plan.writes {
        let target = root.join(&write.relative_path);
        if let Some(parent) = target.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                report.errors.push(format!(
                    "Could not create parent directory for {}: {error}",
                    write.relative_path
                ));
                continue;
            }
        }
        if let Err(error) = fs::write(&target, write.content) {
            report
                .errors
                .push(format!("Could not write {}: {error}", write.relative_path));
            continue;
        }
        match write.action {
            SyncWriteAction::Create => report.created_files.push(write.relative_path),
            SyncWriteAction::Update => report.updated_files.push(write.relative_path),
        }
    }

    report.success = report.errors.is_empty();
    report
}

#[derive(Debug, Clone)]
struct SyncPlan {
    added_files: Vec<String>,
    changed_files: Vec<String>,
    unchanged_files: Vec<String>,
    skipped_files: Vec<String>,
    planned_creates: Vec<String>,
    planned_updates: Vec<String>,
    warnings: Vec<String>,
    writes: Vec<SyncWrite>,
}

#[derive(Debug, Clone)]
struct SyncWrite {
    relative_path: String,
    content: String,
    action: SyncWriteAction,
}

#[derive(Debug, Clone, Copy)]
enum SyncWriteAction {
    Create,
    Update,
}

fn plan_sync(
    root: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
) -> Result<SyncPlan, String> {
    let mut warnings = Vec::new();
    let mut skipped_files = Vec::new();

    match selection {
        ExportSelection::All => {}
        ExportSelection::Schema(schema) => {
            if !inventory
                .schemas
                .iter()
                .any(|candidate| candidate.name == *schema)
            {
                return Err(format!(
                    "Selected schema '{schema}' was not found in the PostgreSQL inventory."
                ));
            }
            for table in inventory
                .tables
                .iter()
                .filter(|table| table.schema_name == *schema)
            {
                if table.table_type != "BASE TABLE" {
                    skipped_files.push(table_file_path(&table.schema_name, &table.table_name)?);
                }
            }
        }
        ExportSelection::Table { schema, table } => {
            let Some(selected_table) = inventory.tables.iter().find(|candidate| {
                candidate.schema_name == *schema && candidate.table_name == *table
            }) else {
                return Err(format!(
                    "Selected table '{schema}.{table}' was not found in the PostgreSQL inventory."
                ));
            };
            if selected_table.table_type != "BASE TABLE" {
                return Err(format!(
                    "Selected table '{schema}.{table}' is not an ordinary/base table and is deferred for Slice 4."
                ));
            }
            let schema_path = schema_file_path(schema)?;
            if !root.join(&schema_path).is_file() {
                warnings.push(format!(
                    "Selected table '{schema}.{table}' has missing local schema file {schema_path}."
                ));
            }
        }
    }

    skipped_files.sort();
    skipped_files.dedup();

    let mut plan = SyncPlan {
        added_files: Vec::new(),
        changed_files: Vec::new(),
        unchanged_files: Vec::new(),
        skipped_files,
        planned_creates: Vec::new(),
        planned_updates: Vec::new(),
        warnings,
        writes: Vec::new(),
    };

    let database_objects = render_database_objects_for_selection(root, inventory, selection)?;
    for object in database_objects.values() {
        let relative_path = object.relative_path.clone();
        ensure_database_object_path(&relative_path)?;
        classify_sync_file(root, &relative_path, object.content.clone(), &mut plan)?;
    }

    Ok(plan)
}

fn classify_sync_file(
    root: &Path,
    relative_path: &str,
    content: String,
    plan: &mut SyncPlan,
) -> Result<(), String> {
    ensure_database_object_path(relative_path)?;
    let target = root.join(relative_path);
    if !target.exists() {
        plan.added_files.push(relative_path.to_string());
        plan.planned_creates.push(relative_path.to_string());
        plan.writes.push(SyncWrite {
            relative_path: relative_path.to_string(),
            content,
            action: SyncWriteAction::Create,
        });
        return Ok(());
    }

    let existing = fs::read_to_string(&target)
        .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
    if existing == content {
        plan.unchanged_files.push(relative_path.to_string());
    } else {
        plan.changed_files.push(relative_path.to_string());
        plan.planned_updates.push(relative_path.to_string());
        plan.writes.push(SyncWrite {
            relative_path: relative_path.to_string(),
            content,
            action: SyncWriteAction::Update,
        });
    }
    Ok(())
}

fn compare_postgres_command(cwd: &Path, parsed: ParsedArgs) -> CompareReport {
    let mut report = empty_compare_report();
    let selection = match ExportSelection::from_options(
        parsed.all,
        parsed.schema.clone(),
        parsed.table.clone(),
    ) {
        Ok(selection) => selection,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    report.compare_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();

    let Some(connection_url) =
        resolve_postgres_url(parsed.url, env::var("DBSTATE_POSTGRES_URL").ok())
    else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };
    if !is_postgres_connection_url(&connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match inspect_postgres(&connection_url) {
        Ok(inventory) => compare_postgres_with_inventory(cwd, &inventory, &selection),
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
            report
        }
    }
}

pub fn compare_postgres_with_inventory(
    cwd: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
) -> CompareReport {
    let mut report = empty_compare_report();
    report.compare_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();

    let project = status_report(cwd, CommandKind::ComparePostgres);
    report.repository_path = project.repository_path.clone();
    report.git_root = project.git_root.clone();
    report.is_git_repository = project.is_git_repository;
    report.branch = project.branch.clone();
    report.working_tree_status = project.working_tree_status;
    report.is_dirty = project.is_dirty;

    if !project.is_git_repository {
        report
            .errors
            .push("Current path is not inside a Git repository.".to_string());
        return report;
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return report;
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    let repo_import = match discover_repository_objects(&root) {
        Ok(import) => import,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    let database_objects = match render_database_objects_for_selection(&root, inventory, selection)
    {
        Ok(objects) => objects,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    let repo_objects = select_repository_objects(&repo_import.objects, selection);

    report.skipped = repo_import.skipped;
    report.warnings = repo_import.warnings;
    report.errors.extend(repo_import.errors);
    if !report.errors.is_empty() {
        return report;
    }

    if let ExportSelection::Table { schema, table } = selection {
        let schema_path = schema_file_path(schema).unwrap_or_else(|_| String::new());
        if !schema_path.is_empty() && !root.join(&schema_path).is_file() {
            report.warnings.push(format!(
                "Selected table '{schema}.{table}' has missing local schema file {schema_path}."
            ));
        }
    }

    let mut keys = BTreeSet::new();
    keys.extend(database_objects.keys().cloned());
    keys.extend(repo_objects.keys().cloned());
    if keys.is_empty() {
        report.errors.push(
            "Selected schema or table was not found in the repository or PostgreSQL inventory."
                .to_string(),
        );
        return report;
    }

    for key in keys {
        match (repo_objects.get(&key), database_objects.get(&key)) {
            (Some(repo_object), Some(database_object)) => {
                if normalize_desired_state_text(&repo_object.content)
                    == normalize_desired_state_text(&database_object.content)
                {
                    report.in_sync.push(repo_object.relative_path.clone());
                } else {
                    report
                        .repo_different
                        .push(repo_object.relative_path.clone());
                }
            }
            (Some(repo_object), None) => {
                report.repo_only.push(repo_object.relative_path.clone());
            }
            (None, Some(database_object)) => {
                report
                    .database_only
                    .push(database_object.relative_path.clone());
            }
            (None, None) => {}
        }
    }

    report.success = true;
    report
}

fn plan_postgres_command(cwd: &Path, parsed: ParsedArgs) -> PlanReport {
    let mut report = empty_plan_report();
    let selection = match ExportSelection::from_options(
        parsed.all,
        parsed.schema.clone(),
        parsed.table.clone(),
    ) {
        Ok(selection) => selection,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    let plan_selection = match PlanSelection::from_options(parsed.includes, parsed.excludes) {
        Ok(selection) => selection,
        Err(error) => {
            report.plan_scope = selection.scope_name();
            report.selected_schemas = selection.selected_schemas();
            report.selected_tables = selection.selected_tables();
            report.errors.push(error);
            return report;
        }
    };

    report.plan_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();
    report.included_objects = plan_selection.included_object_refs();
    report.excluded_objects = plan_selection.excluded_object_refs();

    let Some(connection_url) =
        resolve_postgres_url(parsed.url, env::var("DBSTATE_POSTGRES_URL").ok())
    else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };
    if !is_postgres_connection_url(&connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match inspect_postgres(&connection_url) {
        Ok(inventory) => plan_postgres_with_inventory(cwd, &inventory, &selection, &plan_selection),
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
            report
        }
    }
}

fn release_postgres_command(cwd: &Path, parsed: ParsedArgs) -> ReleaseReport {
    let mut report = empty_release_report(parsed.dry_run);
    let release_name = match parsed.release_name.clone() {
        Some(name) => name,
        None => {
            report
                .errors
                .push("Release name is required. Provide --name <release-name>.".to_string());
            return report;
        }
    };
    report.release_name = release_name.clone();

    if let Err(error) = release_slug(&release_name) {
        report.errors.push(error);
        return report;
    }

    let selection = match ExportSelection::from_options(
        parsed.all,
        parsed.schema.clone(),
        parsed.table.clone(),
    ) {
        Ok(selection) => selection,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    let plan_selection = match PlanSelection::from_options(parsed.includes, parsed.excludes) {
        Ok(selection) => selection,
        Err(error) => {
            report.release_scope = selection.scope_name();
            report.selected_schemas = selection.selected_schemas();
            report.selected_tables = selection.selected_tables();
            report.errors.push(error);
            return report;
        }
    };

    report.release_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();
    report.included_objects = plan_selection.included_object_refs();
    report.excluded_objects = plan_selection.excluded_object_refs();

    let Some(connection_url) =
        resolve_postgres_url(parsed.url, env::var("DBSTATE_POSTGRES_URL").ok())
    else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };
    if !is_postgres_connection_url(&connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match inspect_postgres(&connection_url) {
        Ok(inventory) => release_postgres_with_inventory(
            cwd,
            &inventory,
            &selection,
            &plan_selection,
            &release_name,
            parsed.dry_run,
        ),
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
            report
        }
    }
}

pub fn release_postgres_with_inventory(
    cwd: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
    plan_selection: &PlanSelection,
    release_name: &str,
    dry_run: bool,
) -> ReleaseReport {
    let mut report = empty_release_report(dry_run);
    report.release_name = release_name.to_string();
    report.release_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();
    report.included_objects = plan_selection.included_object_refs();
    report.excluded_objects = plan_selection.excluded_object_refs();

    let slug = match release_slug(release_name) {
        Ok(slug) => slug,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    let project = status_report(cwd, CommandKind::ReleasePostgres);
    report.repository_path = project.repository_path.clone();
    report.git_root = project.git_root.clone();
    report.is_git_repository = project.is_git_repository;
    report.branch = project.branch.clone();
    report.working_tree_status = project.working_tree_status;
    report.is_dirty = project.is_dirty;

    if !project.is_git_repository {
        report
            .errors
            .push("Current path is not inside a Git repository.".to_string());
        return report;
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return report;
    }
    if project.is_dirty && !dry_run {
        report.errors.push(
            "Release artifact generation is blocked because the working tree has changes. Commit/stash changes or use --dry-run."
                .to_string(),
        );
        return report;
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    if !root.join("database/releases").is_dir() {
        report.errors.push(
            "database/releases is missing. Run dbstate init before generating release artifacts."
                .to_string(),
        );
        return report;
    }

    let plan = plan_postgres_with_inventory(cwd, inventory, selection, plan_selection);
    report.included_objects = plan.included_objects.clone();
    report.excluded_objects = plan.excluded_objects.clone();
    report.plan_items = plan.plan_items.clone();
    report.blocked_items = plan.blocked_items.clone();
    report.dependency_warnings = plan.dependency_warnings.clone();
    report.warnings = plan.warnings.clone();
    if report.is_dirty && dry_run {
        report.warnings.push(
            "Release artifact generation will be blocked because the working tree has changes. Commit or stash changes before generating release artifacts."
                .to_string(),
        );
    }
    report.errors = plan.errors.clone();
    report.deferred_object_types = plan.deferred_object_types.clone();
    report.repository_path = plan.repository_path.clone();
    report.git_root = plan.git_root.clone();
    report.is_git_repository = plan.is_git_repository;
    report.branch = plan.branch.clone();
    report.working_tree_status = plan.working_tree_status;
    report.is_dirty = plan.is_dirty;

    if !report.errors.is_empty() {
        update_release_risk(&mut report);
        return report;
    }
    if !report.blocked_items.is_empty() {
        report.errors.push(
            "Release artifact generation is blocked because selected plan items have dependency blockers."
                .to_string(),
        );
        update_release_risk(&mut report);
        return report;
    }

    let artifacts = match plan_release_artifact_paths(&root, &slug) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    report.planned_artifacts = artifacts.relative_paths();
    update_release_risk(&mut report);

    if dry_run {
        report.success = report.errors.is_empty();
        return report;
    }

    let mut artifact_report = report.clone();
    artifact_report.success = true;

    let sql = match render_release_sql(&root, &artifact_report, &artifacts) {
        Ok(sql) => sql,
        Err(error) => {
            report.errors.push(error);
            update_release_risk(&mut report);
            return report;
        }
    };
    let summary = render_release_summary(&artifact_report, &artifacts);
    let risk = render_release_risk_json(&artifact_report, &artifacts);
    let manifest = render_release_manifest_json(
        &artifact_report,
        &artifacts,
        &[
            (&artifacts.sql, &sql),
            (&artifacts.summary, &summary),
            (&artifacts.risk, &risk),
        ],
    );

    let generated = [
        (&artifacts.sql, sql),
        (&artifacts.summary, summary),
        (&artifacts.risk, risk),
        (&artifacts.manifest, manifest),
    ];
    for (relative_path, _) in &generated {
        if let Err(error) = ensure_database_release_path(relative_path) {
            report.errors.push(error);
        }
        if root.join(relative_path).exists() {
            report.errors.push(format!(
                "Refusing to overwrite existing release artifact {relative_path}."
            ));
        }
    }
    if !report.errors.is_empty() {
        update_release_risk(&mut report);
        return report;
    }

    for (relative_path, content) in generated {
        let target = root.join(relative_path);
        if let Err(error) = fs::write(&target, content) {
            report
                .errors
                .push(format!("Could not write {relative_path}: {error}"));
            continue;
        }
        report.created_artifacts.push(relative_path.to_string());
    }

    report.success = report.errors.is_empty();
    update_release_risk(&mut report);
    report
}

#[derive(Debug, Clone)]
struct ReleaseArtifactPaths {
    sql: String,
    summary: String,
    risk: String,
    manifest: String,
}

impl ReleaseArtifactPaths {
    fn relative_paths(&self) -> Vec<String> {
        vec![
            self.sql.clone(),
            self.summary.clone(),
            self.risk.clone(),
            self.manifest.clone(),
        ]
    }

    fn sequence(&self) -> String {
        self.sql
            .rsplit('/')
            .next()
            .and_then(|file| {
                file.split_once('_')
                    .map(|(sequence, _)| sequence.to_string())
            })
            .unwrap_or_else(|| "0000".to_string())
    }
}

fn release_slug(release_name: &str) -> Result<String, String> {
    let trimmed = release_name.trim();
    if trimmed.is_empty() {
        return Err("Release name cannot be empty.".to_string());
    }
    if trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
        || trimmed.contains(' ')
        || trimmed.contains('\t')
    {
        return Err(
            "Release name must use only letters, numbers, hyphen, or underscore.".to_string(),
        );
    }
    let slug = trimmed.to_ascii_lowercase();
    if slug
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        Ok(slug)
    } else {
        Err("Release name must use only letters, numbers, hyphen, or underscore.".to_string())
    }
}

fn plan_release_artifact_paths(root: &Path, slug: &str) -> Result<ReleaseArtifactPaths, String> {
    for sequence in 1..=9999 {
        let prefix = format!("{sequence:04}_{slug}");
        let artifacts = ReleaseArtifactPaths {
            sql: format!("database/releases/{prefix}.sql"),
            summary: format!("database/releases/{prefix}.summary.md"),
            risk: format!("database/releases/{prefix}.risk.json"),
            manifest: format!("database/releases/{prefix}.manifest.json"),
        };
        for relative_path in artifacts.relative_paths() {
            ensure_database_release_path(&relative_path)?;
        }
        if !root.join(&artifacts.sql).exists()
            && !root.join(&artifacts.summary).exists()
            && !root.join(&artifacts.risk).exists()
            && !root.join(&artifacts.manifest).exists()
        {
            return Ok(artifacts);
        }
    }
    Err("Could not choose a release artifact sequence from 0001 to 9999.".to_string())
}

fn ensure_database_release_path(relative_path: &str) -> Result<(), String> {
    if relative_path.starts_with("database/releases/")
        && !relative_path.contains("..")
        && !relative_path.contains('\\')
        && !relative_path.contains(':')
    {
        Ok(())
    } else {
        Err(format!(
            "Refusing to write outside database/releases/: {relative_path}"
        ))
    }
}

#[derive(Debug, Clone, Default)]
struct ReleaseCounts {
    create: usize,
    review_required: usize,
    blocked: usize,
    skipped: usize,
    in_sync: usize,
    database_only: usize,
}

fn update_release_risk(report: &mut ReleaseReport) {
    let (level, reasons) = release_risk(report);
    report.risk_level = level;
    report.risk_reasons = reasons;
}

fn release_risk(report: &ReleaseReport) -> (String, Vec<String>) {
    let counts = release_counts(report);
    let mut reasons = Vec::new();
    if !report.blocked_items.is_empty() || !report.errors.is_empty() {
        if !report.blocked_items.is_empty() {
            reasons.push(format!(
                "{} blocked item(s) prevent release artifact generation.",
                report.blocked_items.len()
            ));
        }
        if !report.errors.is_empty() {
            reasons.push(format!("{} error(s) are present.", report.errors.len()));
        }
        return ("blocked".to_string(), reasons);
    }
    if counts.review_required > 5 || counts.database_only > 0 || counts.skipped > 0 {
        if counts.review_required > 5 {
            reasons.push(format!(
                "{} review-required item(s) need manual assessment.",
                counts.review_required
            ));
        }
        if counts.database_only > 0 {
            reasons.push(format!(
                "{} database-only item(s) are not dropped by DbState.",
                counts.database_only
            ));
        }
        if counts.skipped > 0 {
            reasons.push(format!(
                "{} skipped item(s) require review.",
                counts.skipped
            ));
        }
        return ("high".to_string(), reasons);
    }
    if counts.review_required > 0
        || !report.dependency_warnings.is_empty()
        || !report.warnings.is_empty()
    {
        if counts.review_required > 0 {
            reasons.push(format!(
                "{} review-required item(s) exist.",
                counts.review_required
            ));
        }
        if !report.dependency_warnings.is_empty() {
            reasons.push(format!(
                "{} dependency warning(s) exist.",
                report.dependency_warnings.len()
            ));
        }
        if !report.warnings.is_empty() {
            reasons.push(format!("{} warning(s) exist.", report.warnings.len()));
        }
        return ("medium".to_string(), reasons);
    }
    reasons.push("Only safe create statements are planned.".to_string());
    ("low".to_string(), reasons)
}

fn release_counts(report: &ReleaseReport) -> ReleaseCounts {
    let mut counts = ReleaseCounts {
        blocked: report.blocked_items.len(),
        ..ReleaseCounts::default()
    };
    for item in &report.plan_items {
        match item.plan_intent.as_str() {
            "createInDatabaseLater" => counts.create += 1,
            "updateDatabaseLater" => counts.review_required += 1,
            "reviewDatabaseOnly" => counts.database_only += 1,
            _ => counts.skipped += 1,
        }
        if item.compare_classification == "inSync" {
            counts.in_sync += 1;
        }
        if item.compare_classification == "skipped" {
            counts.skipped += 1;
        }
    }
    counts
}

fn release_object_type_counts(report: &ReleaseReport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for item in &report.plan_items {
        *counts.entry(item.object_type.clone()).or_insert(0) += 1;
    }
    for item in &report.blocked_items {
        *counts.entry(item.object_type.clone()).or_insert(0) += 1;
    }
    counts
}

fn render_release_sql(
    root: &Path,
    report: &ReleaseReport,
    artifacts: &ReleaseArtifactPaths,
) -> Result<String, String> {
    let mut sql = String::new();
    writeln!(sql, "-- DbState Release Artifact").ok();
    writeln!(sql, "-- Release: {}", report.release_name).ok();
    writeln!(sql, "-- Sequence: {}", artifacts.sequence()).ok();
    writeln!(sql, "-- Artifact: {}", artifacts.sql).ok();
    writeln!(sql, "-- Generated by: DbState PostgreSQL v0.1").ok();
    writeln!(
        sql,
        "-- Safety: Review-only. DbState does not execute this SQL."
    )
    .ok();
    writeln!(sql, "-- Source: repository desired state").ok();
    writeln!(sql, "-- Target: PostgreSQL database").ok();
    writeln!(sql, "-- Direct apply: not available through DbState.").ok();
    writeln!(sql, "-- Scope: {}", report.release_scope).ok();
    writeln!(sql, "-- Risk level: {}", report.risk_level).ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Summary").ok();
    writeln!(sql, "-- Selected objects:").ok();
    if report.plan_items.is_empty() {
        writeln!(sql, "--   none").ok();
    } else {
        for item in &report.plan_items {
            writeln!(sql, "--   {}", item.object_ref).ok();
        }
    }
    let counts = release_counts(report);
    writeln!(sql, "-- Counts:").ok();
    writeln!(sql, "--   create: {}", counts.create).ok();
    writeln!(sql, "--   review required: {}", counts.review_required).ok();
    writeln!(sql, "--   database only: {}", counts.database_only).ok();
    writeln!(sql, "--   skipped: {}", counts.skipped).ok();
    writeln!(sql, "--   blocked: {}", counts.blocked).ok();
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Creates").ok();
    writeln!(
        sql,
        "-- WARNING: Full dependency ordering is not implemented in Private Beta."
    )
    .ok();
    let mut creates: Vec<&PlanItem> = report
        .plan_items
        .iter()
        .filter(|item| item.plan_intent == "createInDatabaseLater")
        .collect();
    creates.sort_by_key(release_object_order);
    if creates.is_empty() {
        writeln!(sql, "-- none").ok();
    }
    for item in creates {
        writeln!(sql, "-- Object: {}", item.object_ref).ok();
        sql.push_str(&render_create_later_sql(root, item)?);
        writeln!(sql).ok();
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Review Required").ok();
    let mut review_items: Vec<&PlanItem> = report
        .plan_items
        .iter()
        .filter(|item| item.plan_intent != "createInDatabaseLater")
        .collect();
    review_items.sort_by_key(release_object_order);
    if review_items.is_empty() {
        writeln!(sql, "-- none").ok();
    }
    for item in review_items {
        writeln!(sql, "-- Object: {}", item.object_ref).ok();
        match item.plan_intent.as_str() {
            "updateDatabaseLater" => {
                writeln!(
                    sql,
                    "-- REVIEW REQUIRED: object differs; automatic ALTER is not generated in Private Beta."
                )
                .ok();
            }
            "reviewDatabaseOnly" => {
                writeln!(
                    sql,
                    "-- REVIEW REQUIRED: object exists only in target database. DbState does not generate DROP."
                )
                .ok();
            }
            _ => {
                writeln!(
                    sql,
                    "-- REVIEW REQUIRED: no SQL generated for object with intent {}.",
                    item.plan_intent
                )
                .ok();
            }
        }
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Blocked Items").ok();
    if report.blocked_items.is_empty() {
        writeln!(sql, "-- none").ok();
    } else {
        for item in &report.blocked_items {
            writeln!(
                sql,
                "-- BLOCKED: {} with intent {}.",
                item.object_ref, item.plan_intent
            )
            .ok();
        }
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Deferred Object Types").ok();
    if report.deferred_object_types.is_empty() {
        writeln!(sql, "-- none").ok();
    } else {
        for object_type in &report.deferred_object_types {
            writeln!(sql, "-- DEFERRED: {object_type}").ok();
        }
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    if !report.dependency_warnings.is_empty() {
        writeln!(sql, "-- BEGIN REVIEW SECTION: Warnings").ok();
        writeln!(sql, "-- Dependency warnings:").ok();
        for warning in &report.dependency_warnings {
            writeln!(
                sql,
                "--   [{}] {}: {}",
                warning.severity, warning.warning_type, warning.message
            )
            .ok();
        }
        writeln!(sql, "-- END REVIEW SECTION").ok();
        writeln!(sql).ok();
    }

    Ok(sql)
}

fn release_object_order(item: &&PlanItem) -> (u8, String) {
    let order = match item.object_type.as_str() {
        "schema" => 1,
        "extension" => 2,
        "enum" => 3,
        "sequence" => 4,
        "table" => 5,
        "index" => 6,
        "view" => 7,
        _ => 99,
    };
    (order, item.object_ref.clone())
}

fn render_create_later_sql(root: &Path, item: &PlanItem) -> Result<String, String> {
    let object_ref = ObjectRef::parse(&item.object_ref)?;
    match object_ref {
        ObjectRef::Schema(schema) => Ok(format!(
            "CREATE SCHEMA IF NOT EXISTS {};\n",
            quote_postgres_identifier(&schema)
        )),
        ObjectRef::Table { .. } => {
            ensure_database_object_path(&item.relative_path)?;
            let content = fs::read_to_string(root.join(&item.relative_path))
                .map_err(|error| format!("Could not read {}: {error}", item.relative_path))?;
            Ok(content.replacen("CREATE TABLE ", "CREATE TABLE IF NOT EXISTS ", 1))
        }
        ObjectRef::Extension(_)
        | ObjectRef::Enum { .. }
        | ObjectRef::Sequence { .. }
        | ObjectRef::Index { .. }
        | ObjectRef::View { .. } => {
            ensure_database_object_path(&item.relative_path)?;
            fs::read_to_string(root.join(&item.relative_path))
                .map_err(|error| format!("Could not read {}: {error}", item.relative_path))
        }
    }
}

fn render_release_summary(report: &ReleaseReport, artifacts: &ReleaseArtifactPaths) -> String {
    let mut summary = String::new();
    writeln!(summary, "# DbState PostgreSQL Release Summary").ok();
    writeln!(summary).ok();
    writeln!(summary, "- Release name: {}", report.release_name).ok();
    writeln!(summary, "- Release sequence: {}", artifacts.sequence()).ok();
    writeln!(summary, "- Command: {}", report.command.as_str()).ok();
    writeln!(summary, "- Scope: {}", report.release_scope).ok();
    writeln!(summary, "- Dry run: {}", report.dry_run).ok();
    writeln!(summary, "- Risk level: {}", report.risk_level).ok();
    writeln!(summary, "- Repository path: {}", report.repository_path).ok();
    writeln!(
        summary,
        "- Git root: {}",
        report.git_root.as_deref().unwrap_or("")
    )
    .ok();
    writeln!(
        summary,
        "- Branch: {}",
        report.branch.as_deref().unwrap_or("unknown")
    )
    .ok();
    writeln!(
        summary,
        "- Working tree: {}",
        report.working_tree_status.as_str()
    )
    .ok();
    writeln!(summary).ok();
    writeln!(summary, "## Artifacts").ok();
    for artifact in artifacts.relative_paths() {
        writeln!(summary, "- {artifact}").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Object Counts By Status").ok();
    let counts = release_counts(report);
    writeln!(summary, "- create: {}", counts.create).ok();
    writeln!(summary, "- review required: {}", counts.review_required).ok();
    writeln!(summary, "- blocked: {}", counts.blocked).ok();
    writeln!(summary, "- skipped: {}", counts.skipped).ok();
    writeln!(summary, "- in sync: {}", counts.in_sync).ok();
    writeln!(summary, "- database only: {}", counts.database_only).ok();
    writeln!(summary).ok();
    writeln!(summary, "## Object Counts By Type").ok();
    let type_counts = release_object_type_counts(report);
    if type_counts.is_empty() {
        writeln!(summary, "- none").ok();
    } else {
        for (object_type, count) in type_counts {
            writeln!(summary, "- {object_type}: {count}").ok();
        }
    }
    writeln!(summary).ok();
    writeln!(summary, "## Risk Summary").ok();
    writeln!(summary, "- Risk level: {}", report.risk_level).ok();
    write_markdown_list(&mut summary, &report.risk_reasons);
    writeln!(summary).ok();
    writeln!(summary, "## Included Objects").ok();
    write_markdown_list(&mut summary, &report.included_objects);
    writeln!(summary).ok();
    writeln!(summary, "## Excluded Objects").ok();
    write_markdown_list(&mut summary, &report.excluded_objects);
    writeln!(summary).ok();
    writeln!(summary, "## Plan Items").ok();
    for item in &report.plan_items {
        writeln!(
            summary,
            "- {}: {}, {}, intent {}",
            item.object_ref, item.object_type, item.compare_classification, item.plan_intent
        )
        .ok();
    }
    if report.plan_items.is_empty() {
        writeln!(summary, "- none").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Blocked Items").ok();
    for item in &report.blocked_items {
        writeln!(summary, "- {}: {}", item.object_ref, item.plan_intent).ok();
    }
    if report.blocked_items.is_empty() {
        writeln!(summary, "- none").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Skipped Items").ok();
    let skipped_items: Vec<String> = report
        .plan_items
        .iter()
        .filter(|item| item.compare_classification == "skipped")
        .map(|item| item.object_ref.clone())
        .collect();
    write_markdown_list(&mut summary, &skipped_items);
    writeln!(summary).ok();
    writeln!(summary, "## Dependency Warnings").ok();
    for warning in &report.dependency_warnings {
        writeln!(
            summary,
            "- [{}] {}: {}",
            warning.severity, warning.warning_type, warning.message
        )
        .ok();
    }
    if report.dependency_warnings.is_empty() {
        writeln!(summary, "- none").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Deferred Object Types").ok();
    write_markdown_list(&mut summary, &report.deferred_object_types);
    writeln!(summary).ok();
    writeln!(summary, "## Warnings").ok();
    write_markdown_list(&mut summary, &report.warnings);
    writeln!(summary).ok();
    writeln!(summary, "## Safety").ok();
    writeln!(
        summary,
        "DbState generated these artifacts for review. DbState did not execute SQL, did not apply changes, did not mutate PostgreSQL, and did not stage, commit, push, pull, fetch, or tag Git changes."
    )
    .ok();
    writeln!(summary).ok();
    writeln!(summary, "## Reviewer Checklist").ok();
    writeln!(summary, "1. Review all REVIEW REQUIRED comments.").ok();
    writeln!(summary, "2. Review blocked and skipped items.").ok();
    writeln!(summary, "3. Review deferred object type warnings.").ok();
    writeln!(summary, "4. Confirm no destructive SQL is present.").ok();
    writeln!(summary, "5. Confirm object ordering is acceptable.").ok();
    writeln!(
        summary,
        "6. Have a DBA or responsible engineer review before any manual execution outside DbState."
    )
    .ok();
    writeln!(summary).ok();
    writeln!(summary, "## Current Limitations").ok();
    writeln!(
        summary,
        "- Full dependency ordering is not implemented in Private Beta."
    )
    .ok();
    writeln!(
        summary,
        "- Changed objects produce review-only comments instead of unsafe ALTER statements."
    )
    .ok();
    writeln!(
        summary,
        "- Database-only objects produce review comments instead of DROP statements."
    )
    .ok();
    writeln!(
        summary,
        "- Unsupported and deferred objects remain review-only."
    )
    .ok();
    summary
}

fn write_markdown_list(markdown: &mut String, values: &[String]) {
    if values.is_empty() {
        writeln!(markdown, "- none").ok();
        return;
    }
    for value in values {
        writeln!(markdown, "- {value}").ok();
    }
}

fn render_release_risk_json(report: &ReleaseReport, artifacts: &ReleaseArtifactPaths) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", report.command.as_str(), true);
    write_json_bool_field(&mut json, "success", report.success);
    write_json_string_field(&mut json, "releaseName", &report.release_name, false);
    write_json_string_field(&mut json, "releaseSequence", &artifacts.sequence(), false);
    write_json_string_field(&mut json, "databaseType", &report.database_type, false);
    write_json_array_field(&mut json, "generatedArtifacts", &artifacts.relative_paths());
    write_release_repository_context_field(&mut json, report);
    write_json_string_field(&mut json, "scope", &report.release_scope, false);
    write_release_counts_field(&mut json, "counts", &release_counts(report));
    write_object_type_counts_field(
        &mut json,
        "objectTypeCounts",
        &release_object_type_counts(report),
    );
    write_json_string_field(&mut json, "riskLevel", &report.risk_level, false);
    write_json_array_field(&mut json, "riskReasons", &report.risk_reasons);
    write_json_array_field(&mut json, "selectedObjects", &report.included_objects);
    write_plan_item_array_field(&mut json, "planItems", &report.plan_items);
    write_plan_item_array_field(&mut json, "blockedItems", &report.blocked_items);
    write_plan_item_array_field(&mut json, "skippedItems", &release_skipped_items(report));
    write_dependency_warning_array_field(
        &mut json,
        "dependencyWarnings",
        &report.dependency_warnings,
    );
    write_json_array_field(
        &mut json,
        "deferredObjectTypes",
        &report.deferred_object_types,
    );
    write_json_bool_field(&mut json, "destructiveSqlGenerated", false);
    write_json_bool_field(&mut json, "directApplyAvailable", false);
    write_json_bool_field(&mut json, "generatedSqlExecutionSupported", false);
    write_json_bool_field(&mut json, "databaseMutationPerformed", false);
    write_json_bool_field(&mut json, "gitMutationPerformed", false);
    write_json_bool_field(&mut json, "credentialPersistencePerformed", false);
    write_json_array_field(&mut json, "warnings", &report.warnings);
    write_json_array_field(&mut json, "errors", &report.errors);
    json.push('}');
    json
}

fn render_release_manifest_json(
    report: &ReleaseReport,
    artifacts: &ReleaseArtifactPaths,
    generated: &[(&String, &String)],
) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "releaseName", &report.release_name, true);
    write_json_string_field(&mut json, "releaseSequence", &artifacts.sequence(), false);
    write_json_string_field(
        &mut json,
        "dbstateVersion",
        env!("CARGO_PKG_VERSION"),
        false,
    );
    write_release_manifest_artifacts_field(&mut json, "artifacts", generated, artifacts);
    write_json_bool_field(&mut json, "directApplyAvailable", false);
    write_json_bool_field(&mut json, "generatedSqlExecutionSupported", false);
    json.push('}');
    json
}

fn release_skipped_items(report: &ReleaseReport) -> Vec<PlanItem> {
    report
        .plan_items
        .iter()
        .filter(|item| item.compare_classification == "skipped")
        .cloned()
        .collect()
}

fn write_release_repository_context_field(json: &mut String, report: &ReleaseReport) {
    json.push(',');
    write!(json, "\"repositoryContext\":{{").ok();
    write_json_string_field(json, "repositoryPath", &report.repository_path, true);
    write_json_optional_string_field(json, "gitRoot", report.git_root.as_deref());
    write_json_bool_field(json, "isGitRepository", report.is_git_repository);
    write_json_optional_string_field(json, "branch", report.branch.as_deref());
    write_json_string_field(
        json,
        "workingTreeStatus",
        report.working_tree_status.as_str(),
        false,
    );
    write_json_bool_field(json, "isDirty", report.is_dirty);
    json.push('}');
}

fn write_release_counts_field(json: &mut String, name: &str, counts: &ReleaseCounts) {
    json.push(',');
    write!(
        json,
        "\"{}\":{{\"create\":{},\"reviewRequired\":{},\"blocked\":{},\"skipped\":{},\"inSync\":{},\"databaseOnly\":{}}}",
        escape_json(name),
        counts.create,
        counts.review_required,
        counts.blocked,
        counts.skipped,
        counts.in_sync,
        counts.database_only
    )
    .ok();
}

fn write_object_type_counts_field(json: &mut String, name: &str, counts: &BTreeMap<String, usize>) {
    json.push(',');
    write!(json, "\"{}\":{{", escape_json(name)).ok();
    for (index, (object_type, count)) in counts.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(json, "\"{}\":{}", escape_json(object_type), count).ok();
    }
    json.push('}');
}

fn write_release_manifest_artifacts_field(
    json: &mut String,
    name: &str,
    generated: &[(&String, &String)],
    artifacts: &ReleaseArtifactPaths,
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, (path, content)) in generated.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        let artifact_type = if *path == &artifacts.sql {
            "sql"
        } else if *path == &artifacts.summary {
            "summary"
        } else if *path == &artifacts.risk {
            "risk"
        } else {
            "manifest"
        };
        json.push('{');
        write_json_string_field(json, "artifactType", artifact_type, true);
        write_json_string_field(json, "relativePath", path, false);
        write_json_i64_field(json, "sizeBytes", content.len() as i64, false);
        json.push('}');
    }
    if !generated.is_empty() {
        json.push(',');
    }
    json.push('{');
    write_json_string_field(json, "artifactType", "manifest", true);
    write_json_string_field(json, "relativePath", &artifacts.manifest, false);
    write_json_i64_field(json, "sizeBytes", 0, false);
    json.push('}');
    json.push(']');
}

fn data_compare_postgres_command(cwd: &Path, parsed: ParsedArgs) -> ReferenceDataCompareReport {
    let mut report = empty_reference_data_compare_report();
    let selection = match ReferenceDataSelection::from_options(parsed.all, parsed.table.clone()) {
        Ok(selection) => selection,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    report.compare_scope = selection.scope_name();
    report.selected_tables = selection.selected_tables();

    let Some(connection_url) =
        resolve_postgres_url(parsed.url, env::var("DBSTATE_POSTGRES_URL").ok())
    else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };
    if !is_postgres_connection_url(&connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match data_compare_postgres_with_connection(cwd, &connection_url, &selection) {
        Ok(report) => report,
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
            report
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceDataSelection {
    All,
    Table(String),
}

impl ReferenceDataSelection {
    fn from_options(all: bool, table: Option<String>) -> Result<Self, String> {
        let selected = if all { 1 } else { 0 } + if table.is_some() { 1 } else { 0 };
        if selected == 0 {
            return Err("Selection is required. Provide --table or --all.".to_string());
        }
        if selected > 1 {
            return Err(
                "Use only one data-compare selection option: --table or --all.".to_string(),
            );
        }
        if all {
            return Ok(Self::All);
        }
        let table = table.expect("table selection exists");
        validate_schema_qualified_name(&table)?;
        Ok(Self::Table(table))
    }

    fn scope_name(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::Table(table) => format!("table:{table}"),
        }
    }

    fn selected_tables(&self) -> Vec<String> {
        match self {
            Self::All => Vec::new(),
            Self::Table(table) => vec![table.clone()],
        }
    }

    fn includes_table(&self, table_name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Table(selected) => selected == table_name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceDataRegistry {
    version: i64,
    tables: Vec<ReferenceDataTableConfig>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataTableConfig {
    name: String,
    file: String,
    key_columns: Vec<String>,
    ignore_columns: Vec<String>,
    masked_columns: Vec<String>,
    allow_deletes: bool,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataTableState {
    table_name: String,
    key_columns: Vec<String>,
    rows: Vec<ReferenceDataRow>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataRow {
    values: BTreeMap<String, Option<String>>,
}

pub fn data_compare_postgres_with_connection(
    cwd: &Path,
    connection_url: &str,
    selection: &ReferenceDataSelection,
) -> Result<ReferenceDataCompareReport, String> {
    let mut report = empty_reference_data_compare_report();
    report.compare_scope = selection.scope_name();
    report.selected_tables = selection.selected_tables();

    let project = status_report(cwd, CommandKind::DataComparePostgres);
    report.repository_path = project.repository_path.clone();
    report.git_root = project.git_root.clone();
    report.is_git_repository = project.is_git_repository;
    report.branch = project.branch.clone();
    report.working_tree_status = project.working_tree_status;
    report.is_dirty = project.is_dirty;
    if !project.is_git_repository {
        report
            .errors
            .push("Current path is not inside a Git repository.".to_string());
        return Ok(report);
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return Ok(report);
    }
    if !is_postgres_connection_url(connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return Ok(report);
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    let registry = read_reference_data_registry(&root)?;
    report.warnings.push(format!(
        "Reference-data registry version {} loaded.",
        registry.version
    ));

    let selected_configs: Vec<ReferenceDataTableConfig> = registry
        .tables
        .iter()
        .filter(|config| selection.includes_table(&config.name))
        .cloned()
        .collect();
    if matches!(selection, ReferenceDataSelection::Table(_)) && selected_configs.is_empty() {
        report.errors.push(
            "Selected reference-data table is not configured in database/reference-data/dbstate.reference-data.yml."
                .to_string(),
        );
        return Ok(report);
    }
    report.selected_tables = selected_configs
        .iter()
        .map(|config| config.name.clone())
        .collect();
    if selected_configs.is_empty() {
        report.success = true;
        return Ok(report);
    }

    let mut client = Client::connect(connection_url, NoTls).map_err(|_| {
        "PostgreSQL connection failed. Verify the session-only connection URL, credentials, network, and database availability.".to_string()
    })?;

    for config in selected_configs {
        let state = match read_reference_data_table_state(&root, &config) {
            Ok(state) => state,
            Err(error) => {
                report.errors.push(error);
                continue;
            }
        };
        let (database_rows, database_columns) =
            match read_reference_data_rows_from_postgres(&mut client, &config, &state) {
                Ok(rows) => rows,
                Err(error) => {
                    report.errors.push(error);
                    continue;
                }
            };
        let table_result =
            compare_reference_data_table(&config, &state, &database_rows, &database_columns);
        append_reference_table_result(&mut report, table_result);
    }

    report.success = report.errors.is_empty();
    Ok(report)
}

fn read_reference_data_registry(root: &Path) -> Result<ReferenceDataRegistry, String> {
    let path = root.join("database/reference-data/dbstate.reference-data.yml");
    let content = fs::read_to_string(&path).map_err(|error| {
        format!("Could not read database/reference-data/dbstate.reference-data.yml: {error}")
    })?;
    parse_reference_data_registry(&content)
}

fn parse_reference_data_registry(content: &str) -> Result<ReferenceDataRegistry, String> {
    let value: Value = serde_yaml::from_str(content)
        .map_err(|error| format!("Invalid reference-data registry YAML: {error}"))?;
    let mapping = expect_mapping(&value, "reference-data registry")?;
    let version = required_i64(mapping, "version", "reference-data registry")?;
    let tables_value = required_value(mapping, "tables", "reference-data registry")?;
    let table_values = match tables_value {
        Value::Sequence(values) => values,
        _ => return Err("reference-data registry tables must be a list.".to_string()),
    };

    let mut tables = Vec::new();
    for table_value in table_values {
        let table_mapping = expect_mapping(table_value, "reference-data registry table")?;
        let name = required_string(table_mapping, "name", "reference-data registry table")?;
        validate_schema_qualified_name(&name)?;
        let file = required_string(table_mapping, "file", "reference-data registry table")?;
        ensure_reference_data_relative_path(&file)?;
        let key_columns =
            required_string_list(table_mapping, "key", "reference-data registry table")?;
        if key_columns.is_empty() {
            return Err(format!(
                "Reference-data table '{name}' must define at least one key column."
            ));
        }
        let ignore_columns = optional_string_list(
            table_mapping,
            "ignoreColumns",
            "reference-data registry table",
        )?;
        let masked_columns = optional_string_list(
            table_mapping,
            "maskedColumns",
            "reference-data registry table",
        )?;
        for key in &key_columns {
            if ignore_columns.contains(key) {
                return Err(format!(
                    "Reference-data table '{name}' key column '{key}' cannot be ignored."
                ));
            }
            if masked_columns.contains(key) {
                return Err(format!(
                    "Reference-data table '{name}' key column '{key}' cannot be masked."
                ));
            }
        }
        let allow_deletes = optional_bool(
            table_mapping,
            "allowDeletes",
            "reference-data registry table",
        )?
        .unwrap_or(false);
        tables.push(ReferenceDataTableConfig {
            name,
            file,
            key_columns,
            ignore_columns,
            masked_columns,
            allow_deletes,
        });
    }
    Ok(ReferenceDataRegistry { version, tables })
}

fn read_reference_data_table_state(
    root: &Path,
    config: &ReferenceDataTableConfig,
) -> Result<ReferenceDataTableState, String> {
    ensure_reference_data_relative_path(&config.file)?;
    let path = root.join("database/reference-data").join(&config.file);
    let content = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Could not read database/reference-data/{}: {error}",
            config.file
        )
    })?;
    parse_reference_data_table_state(&content, config)
}

fn parse_reference_data_table_state(
    content: &str,
    config: &ReferenceDataTableConfig,
) -> Result<ReferenceDataTableState, String> {
    let value: Value = serde_yaml::from_str(content)
        .map_err(|error| format!("Invalid reference-data table YAML: {error}"))?;
    let mapping = expect_mapping(&value, "reference-data table file")?;
    let table_name = required_string(mapping, "table", "reference-data table file")?;
    if table_name != config.name {
        return Err(format!(
            "Reference-data table file declares '{table_name}' but registry expects '{}'.",
            config.name
        ));
    }
    let key_columns = required_string_list(mapping, "key", "reference-data table file")?;
    if key_columns != config.key_columns {
        return Err(format!(
            "Reference-data table file key for '{}' does not match registry key.",
            config.name
        ));
    }
    let rows_value = required_value(mapping, "rows", "reference-data table file")?;
    let row_values = match rows_value {
        Value::Sequence(values) => values,
        _ => return Err("reference-data table rows must be a list.".to_string()),
    };

    let mut rows = Vec::new();
    let mut keys = BTreeSet::new();
    for row_value in row_values {
        let row_mapping = expect_mapping(row_value, "reference-data row")?;
        let mut values = BTreeMap::new();
        for (key, value) in row_mapping {
            let Some(column) = key.as_str() else {
                return Err("Reference-data row column names must be strings.".to_string());
            };
            values.insert(column.to_string(), yaml_value_to_reference_string(value)?);
        }
        let row = ReferenceDataRow { values };
        let row_key = reference_row_key(&row, &config.key_columns)?;
        if !keys.insert(row_key.clone()) {
            return Err(format!(
                "Reference-data table '{}' has duplicate row key '{}'.",
                config.name, row_key
            ));
        }
        rows.push(row);
    }

    Ok(ReferenceDataTableState {
        table_name,
        key_columns,
        rows,
    })
}

fn read_reference_data_rows_from_postgres(
    client: &mut Client,
    config: &ReferenceDataTableConfig,
    state: &ReferenceDataTableState,
) -> Result<(Vec<ReferenceDataRow>, BTreeSet<String>), String> {
    let (schema, table) = split_schema_qualified_name(&config.name)?;
    let column_rows = client
        .query(
            "SELECT column_name
             FROM information_schema.columns
             WHERE table_schema = $1
               AND table_name = $2
             ORDER BY ordinal_position",
            &[&schema, &table],
        )
        .map_err(|_| {
            format!(
                "PostgreSQL reference-data catalog query failed for '{}'.",
                config.name
            )
        })?;
    if column_rows.is_empty() {
        return Err(format!(
            "Configured reference-data table '{}' was not found in PostgreSQL.",
            config.name
        ));
    }
    let database_columns: BTreeSet<String> = column_rows
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();
    for key in &config.key_columns {
        if !database_columns.contains(key) {
            return Err(format!(
                "Configured reference-data table '{}' key column '{}' was not found in PostgreSQL.",
                config.name, key
            ));
        }
    }

    let mut selected_columns: BTreeSet<String> = config.key_columns.iter().cloned().collect();
    for row in &state.rows {
        for column in row.values.keys() {
            if !config.ignore_columns.contains(column)
                && !config.masked_columns.contains(column)
                && database_columns.contains(column)
            {
                selected_columns.insert(column.clone());
            }
        }
    }
    let selected_columns: Vec<String> = selected_columns.into_iter().collect();
    let select_list = selected_columns
        .iter()
        .map(|column| {
            format!(
                "{}::text AS {}",
                quote_postgres_identifier(column),
                quote_postgres_identifier(column)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let order_by = config
        .key_columns
        .iter()
        .map(|column| quote_postgres_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT {select_list} FROM {}.{} ORDER BY {order_by}",
        quote_postgres_identifier(&schema),
        quote_postgres_identifier(&table)
    );
    let rows = client.query(&sql, &[]).map_err(|_| {
        format!(
            "PostgreSQL reference-data SELECT failed for '{}'.",
            config.name
        )
    })?;
    let mut database_rows = Vec::new();
    for row in rows {
        let mut values = BTreeMap::new();
        for column in &selected_columns {
            let value: Option<String> = row
                .try_get(column.as_str())
                .map_err(|_| format!("Could not read column '{column}' from '{}'.", config.name))?;
            values.insert(column.clone(), value);
        }
        database_rows.push(ReferenceDataRow { values });
    }
    Ok((database_rows, database_columns))
}

pub fn compare_reference_data_table(
    config: &ReferenceDataTableConfig,
    state: &ReferenceDataTableState,
    database_rows: &[ReferenceDataRow],
    database_columns: &BTreeSet<String>,
) -> ReferenceTableCompareResult {
    let mut result = ReferenceTableCompareResult {
        table_name: config.name.clone(),
        key_columns: config.key_columns.clone(),
        compared_columns: comparable_reference_columns(config, state, database_columns),
        ignored_columns: config.ignore_columns.clone(),
        masked_columns: config.masked_columns.clone(),
        row_counts: ReferenceDataCompareCounts::default(),
        row_results: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };
    if config.allow_deletes {
        result.warnings.push(
            "allowDeletes is recorded but DML generation is not implemented in Slice 8."
                .to_string(),
        );
    }
    if state.table_name != config.name || state.key_columns != config.key_columns {
        result
            .errors
            .push("Reference-data table state does not match registry configuration.".to_string());
        return result;
    }
    for row in &state.rows {
        for column in row.values.keys() {
            if !database_columns.contains(column) {
                result.warnings.push(format!(
                    "Repository row contains column '{}' that does not exist in PostgreSQL table '{}'.",
                    column, config.name
                ));
            }
        }
    }

    let mut repo_by_key = BTreeMap::new();
    for row in &state.rows {
        match reference_row_key(row, &config.key_columns) {
            Ok(key) => {
                repo_by_key.insert(key, row.clone());
            }
            Err(error) => {
                result.errors.push(error);
            }
        }
    }
    let mut database_by_key = BTreeMap::new();
    for row in database_rows {
        match reference_row_key(row, &config.key_columns) {
            Ok(key) => {
                database_by_key.insert(key, row.clone());
            }
            Err(error) => {
                result.errors.push(error);
            }
        }
    }
    if !result.errors.is_empty() {
        return result;
    }

    let mut keys = BTreeSet::new();
    keys.extend(repo_by_key.keys().cloned());
    keys.extend(database_by_key.keys().cloned());
    for key in keys {
        match (repo_by_key.get(&key), database_by_key.get(&key)) {
            (Some(repo_row), Some(database_row)) => {
                let changed_columns =
                    changed_reference_columns(config, repo_row, database_row, database_columns);
                if changed_columns.is_empty() {
                    push_reference_row_result(&mut result, &key, "inSync", Vec::new());
                } else {
                    push_reference_row_result(&mut result, &key, "repoDifferent", changed_columns);
                }
            }
            (Some(_), None) => {
                push_reference_row_result(&mut result, &key, "repoOnly", Vec::new());
            }
            (None, Some(_)) => {
                push_reference_row_result(&mut result, &key, "databaseOnly", Vec::new());
            }
            (None, None) => {}
        }
    }
    result
}

fn push_reference_row_result(
    table_result: &mut ReferenceTableCompareResult,
    row_key: &str,
    classification: &str,
    changed_columns: Vec<String>,
) {
    match classification {
        "inSync" => table_result.row_counts.in_sync += 1,
        "repoDifferent" => table_result.row_counts.repo_different += 1,
        "repoOnly" => table_result.row_counts.repo_only += 1,
        "databaseOnly" => table_result.row_counts.database_only += 1,
        "skipped" => table_result.row_counts.skipped += 1,
        _ => {}
    }
    table_result.row_results.push(ReferenceRowCompareResult {
        table_name: table_result.table_name.clone(),
        row_key: row_key.to_string(),
        classification: classification.to_string(),
        changed_columns,
        masked_columns: table_result.masked_columns.clone(),
        warnings: Vec::new(),
    });
}

fn append_reference_table_result(
    report: &mut ReferenceDataCompareReport,
    table_result: ReferenceTableCompareResult,
) {
    report.counts.in_sync += table_result.row_counts.in_sync;
    report.counts.repo_different += table_result.row_counts.repo_different;
    report.counts.repo_only += table_result.row_counts.repo_only;
    report.counts.database_only += table_result.row_counts.database_only;
    report.counts.skipped += table_result.row_counts.skipped;

    for row in &table_result.row_results {
        let row_ref = format!("{}:{}", row.table_name, row.row_key);
        match row.classification.as_str() {
            "inSync" => report.in_sync.push(row_ref),
            "repoDifferent" => report.repo_different.push(row_ref),
            "repoOnly" => report.repo_only.push(row_ref),
            "databaseOnly" => report.database_only.push(row_ref),
            "skipped" => report.skipped.push(row_ref),
            _ => {}
        }
    }
    report.warnings.extend(table_result.warnings.clone());
    report.errors.extend(table_result.errors.clone());
    report.table_results.push(table_result);
}

fn comparable_reference_columns(
    config: &ReferenceDataTableConfig,
    state: &ReferenceDataTableState,
    database_columns: &BTreeSet<String>,
) -> Vec<String> {
    let mut columns = BTreeSet::new();
    for row in &state.rows {
        for column in row.values.keys() {
            if !config.key_columns.contains(column)
                && !config.ignore_columns.contains(column)
                && !config.masked_columns.contains(column)
                && database_columns.contains(column)
            {
                columns.insert(column.clone());
            }
        }
    }
    columns.into_iter().collect()
}

fn changed_reference_columns(
    config: &ReferenceDataTableConfig,
    repo_row: &ReferenceDataRow,
    database_row: &ReferenceDataRow,
    database_columns: &BTreeSet<String>,
) -> Vec<String> {
    let mut columns = BTreeSet::new();
    for column in repo_row.values.keys() {
        if config.key_columns.contains(column)
            || config.ignore_columns.contains(column)
            || config.masked_columns.contains(column)
            || !database_columns.contains(column)
        {
            continue;
        }
        if repo_row.values.get(column) != database_row.values.get(column) {
            columns.insert(column.clone());
        }
    }
    columns.into_iter().collect()
}

fn reference_row_key(row: &ReferenceDataRow, key_columns: &[String]) -> Result<String, String> {
    let mut parts = Vec::new();
    for key in key_columns {
        let Some(value) = row.values.get(key) else {
            return Err(format!("Reference-data row is missing key column '{key}'."));
        };
        let Some(value) = value else {
            return Err(format!(
                "Reference-data row key column '{key}' cannot be null."
            ));
        };
        parts.push(format!("{key}={value}"));
    }
    Ok(parts.join(","))
}

fn validate_schema_qualified_name(name: &str) -> Result<(), String> {
    let (schema, table) = split_schema_qualified_name(name)?;
    safe_file_component(&schema)?;
    safe_file_component(&table)?;
    Ok(())
}

fn split_schema_qualified_name(name: &str) -> Result<(String, String), String> {
    let Some((schema, table)) = name.split_once('.') else {
        return Err(format!(
            "Reference-data table name '{name}' must use schema-qualified form such as public.payment_methods."
        ));
    };
    if schema.trim().is_empty() || table.trim().is_empty() {
        return Err(format!(
            "Reference-data table name '{name}' must use schema-qualified form such as public.payment_methods."
        ));
    }
    Ok((schema.to_string(), table.to_string()))
}

fn ensure_reference_data_relative_path(relative_path: &str) -> Result<(), String> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains("..")
        || trimmed.contains(':')
        || trimmed.contains('\\')
        || !trimmed.starts_with("tables/")
    {
        return Err(format!(
            "Reference-data file path must stay under database/reference-data/tables/: {trimmed}"
        ));
    }
    Ok(())
}

fn yaml_value_to_reference_string(value: &Value) -> Result<Option<String>, String> {
    match value {
        Value::Null => Ok(None),
        Value::Bool(value) => Ok(Some(value.to_string())),
        Value::Number(value) => Ok(Some(value.to_string())),
        Value::String(value) => Ok(Some(value.clone())),
        _ => Err("Reference-data row values must be scalar YAML values.".to_string()),
    }
}

fn expect_mapping<'a>(value: &'a Value, context: &str) -> Result<&'a Mapping, String> {
    match value {
        Value::Mapping(mapping) => Ok(mapping),
        _ => Err(format!("{context} must be a YAML mapping.")),
    }
}

fn required_value<'a>(mapping: &'a Mapping, key: &str, context: &str) -> Result<&'a Value, String> {
    mapping
        .get(Value::String(key.to_string()))
        .ok_or_else(|| format!("{context} is missing required field '{key}'."))
}

fn required_string(mapping: &Mapping, key: &str, context: &str) -> Result<String, String> {
    match required_value(mapping, key, context)? {
        Value::String(value) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(format!(
            "{context} field '{key}' must be a non-empty string."
        )),
    }
}

fn required_i64(mapping: &Mapping, key: &str, context: &str) -> Result<i64, String> {
    match required_value(mapping, key, context)? {
        Value::Number(value) => value
            .as_i64()
            .ok_or_else(|| format!("{context} field '{key}' must be an integer.")),
        _ => Err(format!("{context} field '{key}' must be an integer.")),
    }
}

fn required_string_list(
    mapping: &Mapping,
    key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    match required_value(mapping, key, context)? {
        Value::Sequence(values) => yaml_sequence_to_strings(values, key, context),
        _ => Err(format!(
            "{context} field '{key}' must be a list of strings."
        )),
    }
}

fn optional_string_list(
    mapping: &Mapping,
    key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    match mapping.get(Value::String(key.to_string())) {
        Some(Value::Sequence(values)) => yaml_sequence_to_strings(values, key, context),
        Some(_) => Err(format!(
            "{context} field '{key}' must be a list of strings."
        )),
        None => Ok(Vec::new()),
    }
}

fn yaml_sequence_to_strings(
    values: &[Value],
    key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    for value in values {
        match value {
            Value::String(value) if !value.trim().is_empty() => output.push(value.clone()),
            _ => {
                return Err(format!(
                    "{context} field '{key}' must be a list of strings."
                ))
            }
        }
    }
    output.sort();
    output.dedup();
    Ok(output)
}

fn optional_bool(mapping: &Mapping, key: &str, context: &str) -> Result<Option<bool>, String> {
    match mapping.get(Value::String(key.to_string())) {
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("{context} field '{key}' must be true or false.")),
        None => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ObjectRef {
    Schema(String),
    Table {
        schema: String,
        table: String,
    },
    Extension(String),
    Enum {
        schema: String,
        enum_name: String,
    },
    Sequence {
        schema: String,
        sequence: String,
    },
    Index {
        schema: String,
        table: String,
        index: String,
    },
    View {
        schema: String,
        view: String,
    },
}

impl ObjectRef {
    fn parse(value: &str) -> Result<Self, String> {
        let Some((object_type, identity)) = value.split_once(':') else {
            return Err(format!(
                "Invalid object reference '{value}'. Use schema:<schema>, table:<schema>.<table>, extension:<name>, enum:<schema>.<name>, sequence:<schema>.<name>, index:<schema>.<table>.<name>, or view:<schema>.<name>."
            ));
        };
        match object_type {
            "schema" => {
                if identity.trim().is_empty() {
                    return Err("Schema object reference cannot be empty.".to_string());
                }
                safe_file_component(identity)?;
                Ok(Self::Schema(identity.to_string()))
            }
            "table" => {
                let Some((schema, table)) = identity.split_once('.') else {
                    return Err(format!(
                        "Invalid table object reference '{value}'. Use table:<schema>.<table>."
                    ));
                };
                if schema.trim().is_empty() || table.trim().is_empty() {
                    return Err(format!(
                        "Invalid table object reference '{value}'. Use table:<schema>.<table>."
                    ));
                }
                safe_file_component(schema)?;
                safe_file_component(table)?;
                Ok(Self::Table {
                    schema: schema.to_string(),
                    table: table.to_string(),
                })
            }
            "extension" => {
                if identity.trim().is_empty() {
                    return Err("Extension object reference cannot be empty.".to_string());
                }
                safe_file_component(identity)?;
                Ok(Self::Extension(identity.to_string()))
            }
            "enum" => {
                let (schema, enum_name) = parse_two_part_object_ref(value, identity, "enum")?;
                Ok(Self::Enum { schema, enum_name })
            }
            "sequence" => {
                let (schema, sequence) = parse_two_part_object_ref(value, identity, "sequence")?;
                Ok(Self::Sequence { schema, sequence })
            }
            "index" => {
                let parts: Vec<&str> = identity.split('.').collect();
                if parts.len() != 3 || parts.iter().any(|part| part.trim().is_empty()) {
                    return Err(format!(
                        "Invalid index object reference '{value}'. Use index:<schema>.<table>.<index>."
                    ));
                }
                safe_file_component(parts[0])?;
                safe_file_component(parts[1])?;
                safe_file_component(parts[2])?;
                Ok(Self::Index {
                    schema: parts[0].to_string(),
                    table: parts[1].to_string(),
                    index: parts[2].to_string(),
                })
            }
            "view" => {
                let (schema, view) = parse_two_part_object_ref(value, identity, "view")?;
                Ok(Self::View { schema, view })
            }
            _ => Err(format!(
                "Invalid object reference '{value}'. Use schema:<schema>, table:<schema>.<table>, extension:<name>, enum:<schema>.<name>, sequence:<schema>.<name>, index:<schema>.<table>.<name>, or view:<schema>.<name>."
            )),
        }
    }

    fn as_str(&self) -> String {
        match self {
            Self::Schema(schema) => format!("schema:{schema}"),
            Self::Table { schema, table } => format!("table:{schema}.{table}"),
            Self::Extension(extension) => format!("extension:{extension}"),
            Self::Enum { schema, enum_name } => format!("enum:{schema}.{enum_name}"),
            Self::Sequence { schema, sequence } => format!("sequence:{schema}.{sequence}"),
            Self::Index {
                schema,
                table,
                index,
            } => format!("index:{schema}.{table}.{index}"),
            Self::View { schema, view } => format!("view:{schema}.{view}"),
        }
    }

    fn object_type(&self) -> &'static str {
        match self {
            Self::Schema(_) => "schema",
            Self::Table { .. } => "table",
            Self::Extension(_) => "extension",
            Self::Enum { .. } => "enum",
            Self::Sequence { .. } => "sequence",
            Self::Index { .. } => "index",
            Self::View { .. } => "view",
        }
    }

    fn required_schema_ref(&self) -> Option<Self> {
        match self {
            Self::Schema(_) | Self::Extension(_) => None,
            Self::Table { schema, .. } => Some(Self::Schema(schema.clone())),
            Self::Enum { schema, .. }
            | Self::Sequence { schema, .. }
            | Self::View { schema, .. } => Some(Self::Schema(schema.clone())),
            Self::Index { schema, .. } => Some(Self::Schema(schema.clone())),
        }
    }
}

fn parse_two_part_object_ref(
    value: &str,
    identity: &str,
    object_type: &str,
) -> Result<(String, String), String> {
    let Some((schema, name)) = identity.split_once('.') else {
        return Err(format!(
            "Invalid {object_type} object reference '{value}'. Use {object_type}:<schema>.<name>."
        ));
    };
    if schema.trim().is_empty() || name.trim().is_empty() {
        return Err(format!(
            "Invalid {object_type} object reference '{value}'. Use {object_type}:<schema>.<name>."
        ));
    }
    safe_file_component(schema)?;
    safe_file_component(name)?;
    Ok((schema.to_string(), name.to_string()))
}

#[derive(Debug, Clone)]
pub struct PlanSelection {
    includes: Vec<ObjectRef>,
    excludes: Vec<ObjectRef>,
}

impl PlanSelection {
    pub fn from_options(includes: Vec<String>, excludes: Vec<String>) -> Result<Self, String> {
        let mut parsed_includes = Vec::new();
        for include in includes {
            parsed_includes.push(ObjectRef::parse(&include)?);
        }
        parsed_includes.sort();
        parsed_includes.dedup();

        let mut parsed_excludes = Vec::new();
        for exclude in excludes {
            parsed_excludes.push(ObjectRef::parse(&exclude)?);
        }
        parsed_excludes.sort();
        parsed_excludes.dedup();

        Ok(Self {
            includes: parsed_includes,
            excludes: parsed_excludes,
        })
    }

    pub fn include_all() -> Self {
        Self {
            includes: Vec::new(),
            excludes: Vec::new(),
        }
    }

    fn included_object_refs(&self) -> Vec<String> {
        self.includes.iter().map(ObjectRef::as_str).collect()
    }

    fn excluded_object_refs(&self) -> Vec<String> {
        self.excludes.iter().map(ObjectRef::as_str).collect()
    }

    fn is_included(&self, object_ref: &ObjectRef) -> bool {
        self.includes.is_empty() || self.includes.contains(object_ref)
    }

    fn is_excluded(&self, object_ref: &ObjectRef) -> bool {
        self.excludes.contains(object_ref)
    }
}

pub fn plan_postgres_with_inventory(
    cwd: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
    plan_selection: &PlanSelection,
) -> PlanReport {
    let compare = compare_postgres_with_inventory(cwd, inventory, selection);
    let mut report = empty_plan_report();
    report.plan_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();
    report.included_objects = plan_selection.included_object_refs();
    report.excluded_objects = plan_selection.excluded_object_refs();
    report.warnings = compare.warnings.clone();
    report.errors = compare.errors.clone();
    report.deferred_object_types = compare.deferred_object_types.clone();
    report.repository_path = compare.repository_path.clone();
    report.git_root = compare.git_root.clone();
    report.is_git_repository = compare.is_git_repository;
    report.branch = compare.branch.clone();
    report.working_tree_status = compare.working_tree_status;
    report.is_dirty = compare.is_dirty;
    report.compare_summary = CompareSummary {
        in_sync: compare.in_sync.len(),
        repo_different: compare.repo_different.len(),
        repo_only: compare.repo_only.len(),
        database_only: compare.database_only.len(),
        skipped: compare.skipped.len(),
    };

    if !compare.success {
        return report;
    }

    let root = match git_root(cwd) {
        Some(root) => root,
        None => {
            report
                .errors
                .push("Current path is not inside a Git repository.".to_string());
            return report;
        }
    };

    let mut candidate_refs = BTreeMap::new();
    add_plan_candidates(
        &mut candidate_refs,
        &compare.repo_different,
        "repoDifferent",
        "updateDatabaseLater",
    );
    add_plan_candidates(
        &mut candidate_refs,
        &compare.repo_only,
        "repoOnly",
        "createInDatabaseLater",
    );
    add_plan_candidates(
        &mut candidate_refs,
        &compare.database_only,
        "databaseOnly",
        "reviewDatabaseOnly",
    );

    let in_sync_refs: BTreeSet<ObjectRef> = compare
        .in_sync
        .iter()
        .filter_map(|path| object_ref_from_relative_path(path).ok())
        .collect();

    for included in &plan_selection.includes {
        if !candidate_refs.contains_key(included) {
            if in_sync_refs.contains(included) {
                report.warnings.push(format!(
                    "Included object '{}' has no actionable difference.",
                    included.as_str()
                ));
            } else {
                report.errors.push(format!(
                    "Included object '{}' was not found in the selected compare scope.",
                    included.as_str()
                ));
            }
        }
    }
    for excluded in &plan_selection.excludes {
        if !candidate_refs.contains_key(excluded) && !in_sync_refs.contains(excluded) {
            report.warnings.push(format!(
                "Excluded object '{}' was not found in the selected compare scope.",
                excluded.as_str()
            ));
        }
    }
    if !report.errors.is_empty() {
        return report;
    }

    let mut selected_refs = BTreeSet::new();
    for object_ref in candidate_refs.keys() {
        if plan_selection.is_excluded(object_ref) {
            continue;
        }
        if plan_selection.is_included(object_ref) {
            selected_refs.insert(object_ref.clone());
        }
    }
    if plan_selection.includes.is_empty() {
        report.included_objects = selected_refs.iter().map(ObjectRef::as_str).collect();
    }

    for object_ref in &selected_refs {
        let Some((relative_path, classification, intent)) = candidate_refs.get(object_ref) else {
            continue;
        };
        let mut item_warnings = Vec::new();
        let mut blocked = false;

        if let Some(required_schema) = object_ref.required_schema_ref() {
            let required_schema_path = match &required_schema {
                ObjectRef::Schema(schema) => schema_file_path(schema).unwrap_or_default(),
                ObjectRef::Table { .. }
                | ObjectRef::Extension(_)
                | ObjectRef::Enum { .. }
                | ObjectRef::Sequence { .. }
                | ObjectRef::Index { .. }
                | ObjectRef::View { .. } => String::new(),
            };
            if !required_schema_path.is_empty() && !root.join(&required_schema_path).is_file() {
                let warning = DependencyWarning {
                    warning_type: "missingDependency".to_string(),
                    object_ref: object_ref.as_str(),
                    required_object_ref: Some(required_schema.as_str()),
                    message: format!(
                        "Selected table '{}' requires schema desired-state file '{}'.",
                        object_ref.as_str(),
                        required_schema_path
                    ),
                    severity: "blocked".to_string(),
                };
                item_warnings.push(warning.message.clone());
                report.dependency_warnings.push(warning);
                blocked = true;
            }
            if plan_selection.is_excluded(&required_schema) {
                let warning = DependencyWarning {
                    warning_type: "excludedRequiredDependency".to_string(),
                    object_ref: object_ref.as_str(),
                    required_object_ref: Some(required_schema.as_str()),
                    message: format!(
                        "Selected table '{}' depends on excluded schema '{}'.",
                        object_ref.as_str(),
                        required_schema.as_str()
                    ),
                    severity: "blocked".to_string(),
                };
                item_warnings.push(warning.message.clone());
                report.dependency_warnings.push(warning);
                report.dependency_warnings.push(DependencyWarning {
                    warning_type: "dependentObjectImpacted".to_string(),
                    object_ref: object_ref.as_str(),
                    required_object_ref: Some(required_schema.as_str()),
                    message: format!(
                        "Excluded schema '{}' impacts selected table '{}'.",
                        required_schema.as_str(),
                        object_ref.as_str()
                    ),
                    severity: "blocked".to_string(),
                });
                blocked = true;
            }
        }

        let item = PlanItem {
            object_ref: object_ref.as_str(),
            object_type: object_ref.object_type().to_string(),
            relative_path: relative_path.clone(),
            compare_classification: classification.to_string(),
            plan_intent: if blocked {
                "blocked".to_string()
            } else {
                intent.to_string()
            },
            selected: true,
            blocked,
            warnings: item_warnings,
        };

        if item.blocked {
            report.blocked_items.push(item);
        } else {
            report.plan_items.push(item);
        }
    }

    report.plan_items.sort_by(|left, right| {
        left.object_ref
            .cmp(&right.object_ref)
            .then(left.relative_path.cmp(&right.relative_path))
    });
    report.blocked_items.sort_by(|left, right| {
        left.object_ref
            .cmp(&right.object_ref)
            .then(left.relative_path.cmp(&right.relative_path))
    });
    report
        .dependency_warnings
        .sort_by(|left, right| left.object_ref.cmp(&right.object_ref));
    report.success = report.errors.is_empty();
    report
}

fn add_plan_candidates(
    candidates: &mut BTreeMap<ObjectRef, (String, &'static str, &'static str)>,
    paths: &[String],
    classification: &'static str,
    intent: &'static str,
) {
    for path in paths {
        if let Ok(object_ref) = object_ref_from_relative_path(path) {
            candidates.insert(object_ref, (path.clone(), classification, intent));
        }
    }
}

fn object_ref_from_relative_path(relative_path: &str) -> Result<ObjectRef, String> {
    ensure_database_object_path(relative_path)?;
    if let Some(file_name) = relative_path.strip_prefix("database/objects/schemas/") {
        let Some(schema) = schema_name_from_file(file_name) else {
            return Err(format!(
                "Invalid schema desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Schema(schema));
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/tables/") {
        let Some((schema, table)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid table desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Table { schema, table });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/extensions/") {
        let Some(extension) = schema_name_from_file(file_name) else {
            return Err(format!(
                "Invalid extension desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Extension(extension));
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/enums/") {
        let Some((schema, enum_name)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid enum desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Enum { schema, enum_name });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/sequences/") {
        let Some((schema, sequence)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid sequence desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Sequence { schema, sequence });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/indexes/") {
        let Some((schema, table, index)) = three_part_name_from_file(file_name) else {
            return Err(format!(
                "Invalid index desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Index {
            schema,
            table,
            index,
        });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/views/") {
        let Some((schema, view)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid view desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::View { schema, view });
    }
    Err(format!(
        "Unsupported desired-state file path: {relative_path}"
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepositoryObjectType {
    Schema,
    Table,
    Extension,
    Enum,
    Sequence,
    Index,
    View,
}

#[derive(Debug, Clone)]
struct DesiredStateObject {
    object_type: RepositoryObjectType,
    schema_name: String,
    table_name: Option<String>,
    object_name: String,
    parent_name: Option<String>,
    relative_path: String,
    content: String,
}

#[derive(Debug, Clone)]
struct RepositoryImport {
    objects: BTreeMap<String, DesiredStateObject>,
    skipped: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

fn discover_repository_objects(root: &Path) -> Result<RepositoryImport, String> {
    let mut import = RepositoryImport {
        objects: BTreeMap::new(),
        skipped: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    discover_schema_files(root, &mut import)?;
    discover_table_files(root, &mut import)?;
    discover_extension_files(root, &mut import)?;
    discover_enum_files(root, &mut import)?;
    discover_sequence_files(root, &mut import)?;
    discover_index_files(root, &mut import)?;
    discover_view_files(root, &mut import)?;
    import.skipped.sort();
    import.skipped.dedup();
    Ok(import)
}

fn discover_schema_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/schemas");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/schemas: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read schema file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/schemas/{file_name}");
        let Some(schema) = schema_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            schema_key(&schema),
            DesiredStateObject {
                object_type: RepositoryObjectType::Schema,
                schema_name: schema.clone(),
                table_name: None,
                object_name: schema,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_table_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/tables");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/tables: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read table file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/tables/{file_name}");
        let Some((schema, table)) = table_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err() || safe_file_component(&table).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            table_key(&schema, &table),
            DesiredStateObject {
                object_type: RepositoryObjectType::Table,
                schema_name: schema,
                table_name: Some(table.clone()),
                object_name: table,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_extension_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_one_part_object_files(
        root,
        import,
        "database/objects/extensions",
        RepositoryObjectType::Extension,
        extension_key,
    )
}

fn discover_enum_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_two_part_object_files(
        root,
        import,
        "database/objects/enums",
        RepositoryObjectType::Enum,
        enum_key,
    )
}

fn discover_sequence_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_two_part_object_files(
        root,
        import,
        "database/objects/sequences",
        RepositoryObjectType::Sequence,
        sequence_key,
    )
}

fn discover_view_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_two_part_object_files(
        root,
        import,
        "database/objects/views",
        RepositoryObjectType::View,
        view_key,
    )
}

fn discover_index_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/indexes");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/indexes: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read index file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/indexes/{file_name}");
        let Some((schema, table, index)) = three_part_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err()
            || safe_file_component(&table).is_err()
            || safe_file_component(&index).is_err()
        {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            index_key(&schema, &table, &index),
            DesiredStateObject {
                object_type: RepositoryObjectType::Index,
                schema_name: schema,
                table_name: Some(table.clone()),
                object_name: index,
                parent_name: Some(table),
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_one_part_object_files(
    root: &Path,
    import: &mut RepositoryImport,
    folder: &str,
    object_type: RepositoryObjectType,
    key_fn: fn(&str) -> String,
) -> Result<(), String> {
    let dir = root.join(folder);
    for entry in fs::read_dir(&dir).map_err(|error| format!("Could not read {folder}: {error}"))? {
        let entry =
            entry.map_err(|error| format!("Could not read desired-state file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("{folder}/{file_name}");
        let Some(name) = schema_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&name).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            key_fn(&name),
            DesiredStateObject {
                object_type: object_type.clone(),
                schema_name: String::new(),
                table_name: None,
                object_name: name,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_two_part_object_files(
    root: &Path,
    import: &mut RepositoryImport,
    folder: &str,
    object_type: RepositoryObjectType,
    key_fn: fn(&str, &str) -> String,
) -> Result<(), String> {
    let dir = root.join(folder);
    for entry in fs::read_dir(&dir).map_err(|error| format!("Could not read {folder}: {error}"))? {
        let entry =
            entry.map_err(|error| format!("Could not read desired-state file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("{folder}/{file_name}");
        let Some((schema, name)) = table_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err() || safe_file_component(&name).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            key_fn(&schema, &name),
            DesiredStateObject {
                object_type: object_type.clone(),
                schema_name: schema,
                table_name: None,
                object_name: name,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn schema_name_from_file(file_name: &str) -> Option<String> {
    file_name
        .strip_suffix(".sql")
        .filter(|stem| !stem.is_empty())
        .map(|stem| stem.to_string())
}

fn table_name_from_file(file_name: &str) -> Option<(String, String)> {
    let stem = file_name.strip_suffix(".sql")?;
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some((parts[0].to_string(), parts[1].to_string()))
}

fn three_part_name_from_file(file_name: &str) -> Option<(String, String, String)> {
    let stem = file_name.strip_suffix(".sql")?;
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}

fn render_database_objects_for_selection(
    _root: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
) -> Result<BTreeMap<String, DesiredStateObject>, String> {
    let mut objects = BTreeMap::new();

    let mut schema_names = Vec::new();
    let mut table_names = Vec::new();
    let mut include_extensions = false;
    let mut enum_names = Vec::new();
    let mut sequence_names = Vec::new();
    let mut index_names = Vec::new();
    let mut view_names = Vec::new();
    match selection {
        ExportSelection::All => {
            include_extensions = true;
            schema_names.extend(inventory.schemas.iter().map(|schema| schema.name.clone()));
            table_names.extend(
                inventory
                    .tables
                    .iter()
                    .filter(|table| table.table_type == "BASE TABLE")
                    .map(|table| (table.schema_name.clone(), table.table_name.clone())),
            );
            enum_names.extend(
                inventory
                    .enums
                    .iter()
                    .map(|item| (item.schema_name.clone(), item.enum_name.clone())),
            );
            sequence_names.extend(
                inventory
                    .sequences
                    .iter()
                    .map(|item| (item.schema_name.clone(), item.sequence_name.clone())),
            );
            index_names.extend(inventory.indexes.iter().map(|item| {
                (
                    item.schema_name.clone(),
                    item.table_name.clone(),
                    item.index_name.clone(),
                )
            }));
            view_names.extend(
                inventory
                    .views
                    .iter()
                    .map(|item| (item.schema_name.clone(), item.view_name.clone())),
            );
        }
        ExportSelection::Schema(schema) => {
            if inventory
                .schemas
                .iter()
                .any(|candidate| candidate.name == *schema)
            {
                schema_names.push(schema.clone());
            }
            table_names.extend(
                inventory
                    .tables
                    .iter()
                    .filter(|table| {
                        table.schema_name == *schema && table.table_type == "BASE TABLE"
                    })
                    .map(|table| (table.schema_name.clone(), table.table_name.clone())),
            );
            enum_names.extend(
                inventory
                    .enums
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| (item.schema_name.clone(), item.enum_name.clone())),
            );
            sequence_names.extend(
                inventory
                    .sequences
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| (item.schema_name.clone(), item.sequence_name.clone())),
            );
            index_names.extend(
                inventory
                    .indexes
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.index_name.clone(),
                        )
                    }),
            );
            view_names.extend(
                inventory
                    .views
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| (item.schema_name.clone(), item.view_name.clone())),
            );
        }
        ExportSelection::Table { schema, table } => {
            if inventory.tables.iter().any(|candidate| {
                candidate.schema_name == *schema
                    && candidate.table_name == *table
                    && candidate.table_type == "BASE TABLE"
            }) {
                table_names.push((schema.clone(), table.clone()));
            }
            index_names.extend(
                inventory
                    .indexes
                    .iter()
                    .filter(|item| item.schema_name == *schema && item.table_name == *table)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.index_name.clone(),
                        )
                    }),
            );
        }
    }

    schema_names.sort();
    schema_names.dedup();
    table_names.sort();
    table_names.dedup();
    enum_names.sort();
    enum_names.dedup();
    sequence_names.sort();
    sequence_names.dedup();
    index_names.sort();
    index_names.dedup();
    view_names.sort();
    view_names.dedup();

    for schema in schema_names {
        let relative_path = schema_file_path(&schema)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Schema,
            schema_name: schema.clone(),
            table_name: None,
            object_name: schema.clone(),
            parent_name: None,
            relative_path,
            content: render_schema_sql(&schema),
        };
        objects.insert(object_key(&object), object);
    }

    for (schema, table) in table_names {
        let relative_path = table_file_path(&schema, &table)?;
        let columns: Vec<ColumnInfo> = inventory
            .columns
            .iter()
            .filter(|column| column.schema_name == schema && column.table_name == table)
            .cloned()
            .collect();
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Table,
            schema_name: schema.clone(),
            table_name: Some(table.clone()),
            object_name: table.clone(),
            parent_name: None,
            relative_path,
            content: render_table_sql(&schema, &table, &columns),
        };
        objects.insert(object_key(&object), object);
    }
    if include_extensions {
        for extension in &inventory.extensions {
            let relative_path = extension_file_path(&extension.extension_name)?;
            let object = DesiredStateObject {
                object_type: RepositoryObjectType::Extension,
                schema_name: String::new(),
                table_name: None,
                object_name: extension.extension_name.clone(),
                parent_name: None,
                relative_path,
                content: render_extension_sql(extension),
            };
            objects.insert(object_key(&object), object);
        }
    }
    for (schema, enum_name) in enum_names {
        let Some(enum_info) = inventory
            .enums
            .iter()
            .find(|item| item.schema_name == schema && item.enum_name == enum_name)
        else {
            continue;
        };
        let relative_path = enum_file_path(&schema, &enum_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Enum,
            schema_name: schema.clone(),
            table_name: None,
            object_name: enum_name.clone(),
            parent_name: None,
            relative_path,
            content: render_enum_sql(enum_info),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, sequence_name) in sequence_names {
        let Some(sequence) = inventory
            .sequences
            .iter()
            .find(|item| item.schema_name == schema && item.sequence_name == sequence_name)
        else {
            continue;
        };
        let relative_path = sequence_file_path(&schema, &sequence_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Sequence,
            schema_name: schema.clone(),
            table_name: None,
            object_name: sequence_name.clone(),
            parent_name: None,
            relative_path,
            content: render_sequence_sql(sequence),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, table, index_name) in index_names {
        let Some(index) = inventory.indexes.iter().find(|item| {
            item.schema_name == schema && item.table_name == table && item.index_name == index_name
        }) else {
            continue;
        };
        let relative_path = index_file_path(&schema, &table, &index_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Index,
            schema_name: schema.clone(),
            table_name: Some(table.clone()),
            object_name: index_name.clone(),
            parent_name: Some(table),
            relative_path,
            content: render_index_sql(index),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, view_name) in view_names {
        let Some(view) = inventory
            .views
            .iter()
            .find(|item| item.schema_name == schema && item.view_name == view_name)
        else {
            continue;
        };
        let relative_path = view_file_path(&schema, &view_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::View,
            schema_name: schema.clone(),
            table_name: None,
            object_name: view_name.clone(),
            parent_name: None,
            relative_path,
            content: render_view_sql(view),
        };
        objects.insert(object_key(&object), object);
    }
    Ok(objects)
}

fn select_repository_objects(
    objects: &BTreeMap<String, DesiredStateObject>,
    selection: &ExportSelection,
) -> BTreeMap<String, DesiredStateObject> {
    let mut selected = BTreeMap::new();
    for (key, object) in objects {
        let include = match selection {
            ExportSelection::All => true,
            ExportSelection::Schema(schema) => object.schema_name == *schema,
            ExportSelection::Table { schema, table } => {
                object.schema_name == *schema
                    && (object.object_type == RepositoryObjectType::Table
                        && object.table_name.as_deref() == Some(table.as_str())
                        || object.object_type == RepositoryObjectType::Index
                            && object.parent_name.as_deref() == Some(table.as_str()))
            }
        };
        if include {
            selected.insert(key.clone(), object.clone());
        }
    }
    selected
}

fn object_key(object: &DesiredStateObject) -> String {
    match object.object_type {
        RepositoryObjectType::Schema => schema_key(&object.schema_name),
        RepositoryObjectType::Table => table_key(
            &object.schema_name,
            object.table_name.as_deref().unwrap_or(""),
        ),
        RepositoryObjectType::Extension => extension_key(&object.object_name),
        RepositoryObjectType::Enum => enum_key(&object.schema_name, &object.object_name),
        RepositoryObjectType::Sequence => sequence_key(&object.schema_name, &object.object_name),
        RepositoryObjectType::Index => index_key(
            &object.schema_name,
            object.parent_name.as_deref().unwrap_or(""),
            &object.object_name,
        ),
        RepositoryObjectType::View => view_key(&object.schema_name, &object.object_name),
    }
}

fn schema_key(schema: &str) -> String {
    format!("schema:{schema}")
}

fn table_key(schema: &str, table: &str) -> String {
    format!("table:{schema}.{table}")
}

fn extension_key(extension: &str) -> String {
    format!("extension:{extension}")
}

fn enum_key(schema: &str, enum_name: &str) -> String {
    format!("enum:{schema}.{enum_name}")
}

fn sequence_key(schema: &str, sequence: &str) -> String {
    format!("sequence:{schema}.{sequence}")
}

fn index_key(schema: &str, table: &str, index: &str) -> String {
    format!("index:{schema}.{table}.{index}")
}

fn view_key(schema: &str, view: &str) -> String {
    format!("view:{schema}.{view}")
}

pub fn normalize_desired_state_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = normalized
        .split('\n')
        .map(|line| line.trim_end().to_string())
        .collect();
    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn empty_inspection_report(command: CommandKind) -> InspectionReport {
    InspectionReport {
        command,
        success: false,
        database_type: "postgresql".to_string(),
        inspection_scope: vec![
            "schemas".to_string(),
            "tables".to_string(),
            "columns".to_string(),
            "extensions".to_string(),
            "enums".to_string(),
            "sequences".to_string(),
            "indexes".to_string(),
            "views".to_string(),
        ],
        schemas: Vec::new(),
        tables: Vec::new(),
        columns: Vec::new(),
        extensions: Vec::new(),
        enums: Vec::new(),
        sequences: Vec::new(),
        indexes: Vec::new(),
        views: Vec::new(),
        counts: InspectionCounts {
            schemas: 0,
            tables: 0,
            columns: 0,
            extensions: 0,
            enums: 0,
            sequences: 0,
            indexes: 0,
            views: 0,
        },
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn empty_export_report(dry_run: bool) -> ExportReport {
    ExportReport {
        command: CommandKind::ExportPostgres,
        success: false,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        export_scope: "<none>".to_string(),
        dry_run,
        selected_schemas: Vec::new(),
        selected_tables: Vec::new(),
        planned_files: Vec::new(),
        created_files: Vec::new(),
        skipped_files: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn empty_sync_report(dry_run: bool) -> SyncReport {
    SyncReport {
        command: CommandKind::SyncPostgres,
        success: false,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        sync_scope: "<none>".to_string(),
        dry_run,
        selected_schemas: Vec::new(),
        selected_tables: Vec::new(),
        added_files: Vec::new(),
        changed_files: Vec::new(),
        unchanged_files: Vec::new(),
        skipped_files: Vec::new(),
        planned_creates: Vec::new(),
        planned_updates: Vec::new(),
        created_files: Vec::new(),
        updated_files: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn empty_compare_report() -> CompareReport {
    CompareReport {
        command: CommandKind::ComparePostgres,
        success: false,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        compare_scope: "<none>".to_string(),
        selected_schemas: Vec::new(),
        selected_tables: Vec::new(),
        in_sync: Vec::new(),
        repo_different: Vec::new(),
        repo_only: Vec::new(),
        database_only: Vec::new(),
        skipped: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn empty_plan_report() -> PlanReport {
    PlanReport {
        command: CommandKind::PlanPostgres,
        success: false,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        plan_scope: "<none>".to_string(),
        selected_schemas: Vec::new(),
        selected_tables: Vec::new(),
        included_objects: Vec::new(),
        excluded_objects: Vec::new(),
        plan_items: Vec::new(),
        blocked_items: Vec::new(),
        dependency_warnings: Vec::new(),
        compare_summary: CompareSummary {
            in_sync: 0,
            repo_different: 0,
            repo_only: 0,
            database_only: 0,
            skipped: 0,
        },
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn empty_release_report(dry_run: bool) -> ReleaseReport {
    ReleaseReport {
        command: CommandKind::ReleasePostgres,
        success: false,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        release_name: String::new(),
        release_scope: "<none>".to_string(),
        dry_run,
        selected_schemas: Vec::new(),
        selected_tables: Vec::new(),
        included_objects: Vec::new(),
        excluded_objects: Vec::new(),
        plan_items: Vec::new(),
        blocked_items: Vec::new(),
        dependency_warnings: Vec::new(),
        planned_artifacts: Vec::new(),
        created_artifacts: Vec::new(),
        risk_level: "medium".to_string(),
        risk_reasons: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn empty_reference_data_compare_report() -> ReferenceDataCompareReport {
    ReferenceDataCompareReport {
        command: CommandKind::DataComparePostgres,
        success: false,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        compare_scope: "<none>".to_string(),
        selected_tables: Vec::new(),
        table_results: Vec::new(),
        counts: ReferenceDataCompareCounts::default(),
        in_sync: Vec::new(),
        repo_different: Vec::new(),
        repo_only: Vec::new(),
        database_only: Vec::new(),
        skipped: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    }
}

pub fn redact_message(message: &str, secret: &str) -> String {
    let redacted = if secret.is_empty() {
        message.to_string()
    } else {
        message.replace(secret, "<redacted>")
    };
    redact_postgres_url(&redacted)
}

pub fn redact_postgres_url(value: &str) -> String {
    let mut output = String::new();
    for token in value.split_whitespace() {
        if token.starts_with("postgres://") || token.starts_with("postgresql://") {
            output.push_str("<redacted>");
        } else {
            if !output.is_empty() {
                output.push(' ');
            }
            output.push_str(token);
        }
    }
    if output.is_empty() && !value.is_empty() {
        "<redacted>".to_string()
    } else {
        output
    }
}

pub fn is_user_schema(schema_name: &str) -> bool {
    schema_name != "pg_catalog"
        && schema_name != "information_schema"
        && !schema_name.starts_with("pg_toast")
        && !schema_name.starts_with("pg_")
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

fn git_root(cwd: &Path) -> Option<PathBuf> {
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

fn git_branch(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .current_dir(root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}

fn git_working_tree_status(root: &Path) -> WorkingTreeStatus {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
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

fn display_path(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized
        .strip_prefix("///?/")
        .or_else(|| normalized.strip_prefix("//?/"))
        .unwrap_or(&normalized)
        .to_string()
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
        writeln!(text, "Working tree: {}", self.working_tree_status.as_str()).ok();
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
        write_json_string_field(
            &mut json,
            "workingTreeStatus",
            self.working_tree_status.as_str(),
            false,
        );
        write_json_bool_field(&mut json, "isDirty", self.is_dirty);
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

impl InspectionReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(
            text,
            "Inspection scope: {}",
            self.inspection_scope.join(", ")
        )
        .ok();
        writeln!(text, "Schema count: {}", self.counts.schemas).ok();
        writeln!(text, "Table count: {}", self.counts.tables).ok();
        writeln!(text, "Column count: {}", self.counts.columns).ok();
        writeln!(text, "Extension count: {}", self.counts.extensions).ok();
        writeln!(text, "Enum count: {}", self.counts.enums).ok();
        writeln!(text, "Sequence count: {}", self.counts.sequences).ok();
        writeln!(text, "Index count: {}", self.counts.indexes).ok();
        writeln!(text, "View count: {}", self.counts.views).ok();
        writeln!(text, "Schemas:").ok();
        for schema in &self.schemas {
            writeln!(text, "  - {}", schema.name).ok();
        }
        writeln!(text, "Tables:").ok();
        for table in &self.tables {
            writeln!(
                text,
                "  - {}.{} ({})",
                table.schema_name, table.table_name, table.table_type
            )
            .ok();
        }
        writeln!(text, "Extensions:").ok();
        for extension in &self.extensions {
            writeln!(text, "  - {}", extension.extension_name).ok();
        }
        writeln!(text, "Enums:").ok();
        for enum_info in &self.enums {
            writeln!(
                text,
                "  - {}.{}",
                enum_info.schema_name, enum_info.enum_name
            )
            .ok();
        }
        writeln!(text, "Sequences:").ok();
        for sequence in &self.sequences {
            writeln!(
                text,
                "  - {}.{}",
                sequence.schema_name, sequence.sequence_name
            )
            .ok();
        }
        writeln!(text, "Indexes:").ok();
        for index in &self.indexes {
            writeln!(
                text,
                "  - {}.{}.{}",
                index.schema_name, index.table_name, index.index_name
            )
            .ok();
        }
        writeln!(text, "Views:").ok();
        for view in &self.views {
            writeln!(text, "  - {}.{}", view.schema_name, view.view_name).ok();
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
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_array_field(&mut json, "inspectionScope", &self.inspection_scope);
        write_schema_array_field(&mut json, "schemas", &self.schemas);
        write_table_array_field(&mut json, "tables", &self.tables);
        write_column_array_field(&mut json, "columns", &self.columns);
        write_extension_array_field(&mut json, "extensions", &self.extensions);
        write_enum_array_field(&mut json, "enums", &self.enums);
        write_sequence_array_field(&mut json, "sequences", &self.sequences);
        write_index_array_field(&mut json, "indexes", &self.indexes);
        write_view_array_field(&mut json, "views", &self.views);
        write_counts_field(&mut json, "counts", &self.counts);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        write_json_array_field(
            &mut json,
            "deferredObjectTypes",
            &self.deferred_object_types,
        );
        json.push('}');
        json
    }
}

impl ExportReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Database type: {}", self.database_type).ok();
        writeln!(text, "Export scope: {}", self.export_scope).ok();
        writeln!(text, "Dry run: {}", self.dry_run).ok();
        writeln!(text, "Planned files:").ok();
        for path in &self.planned_files {
            writeln!(text, "  - {path}").ok();
        }
        writeln!(text, "Created files:").ok();
        for path in &self.created_files {
            writeln!(text, "  - {path}").ok();
        }
        writeln!(text, "Skipped files:").ok();
        for path in &self.skipped_files {
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
        write_repository_context_fields(
            &mut json,
            &self.repository_path,
            self.git_root.as_deref(),
            self.is_git_repository,
            self.branch.as_deref(),
            self.working_tree_status,
            self.is_dirty,
        );
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_string_field(&mut json, "exportScope", &self.export_scope, false);
        write_json_bool_field(&mut json, "dryRun", self.dry_run);
        write_json_array_field(&mut json, "selectedSchemas", &self.selected_schemas);
        write_json_array_field(&mut json, "selectedTables", &self.selected_tables);
        write_json_array_field(&mut json, "plannedFiles", &self.planned_files);
        write_json_array_field(&mut json, "createdFiles", &self.created_files);
        write_json_array_field(&mut json, "skippedFiles", &self.skipped_files);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        write_json_array_field(
            &mut json,
            "deferredObjectTypes",
            &self.deferred_object_types,
        );
        json.push('}');
        json
    }
}

impl SyncReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Database type: {}", self.database_type).ok();
        writeln!(text, "Sync scope: {}", self.sync_scope).ok();
        writeln!(text, "Dry run: {}", self.dry_run).ok();
        write_path_list(&mut text, "Added files", &self.added_files);
        write_path_list(&mut text, "Changed files", &self.changed_files);
        write_path_list(&mut text, "Unchanged files", &self.unchanged_files);
        write_path_list(&mut text, "Skipped files", &self.skipped_files);
        write_path_list(&mut text, "Planned creates", &self.planned_creates);
        write_path_list(&mut text, "Planned updates", &self.planned_updates);
        write_path_list(&mut text, "Created files", &self.created_files);
        write_path_list(&mut text, "Updated files", &self.updated_files);
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
        write_repository_context_fields(
            &mut json,
            &self.repository_path,
            self.git_root.as_deref(),
            self.is_git_repository,
            self.branch.as_deref(),
            self.working_tree_status,
            self.is_dirty,
        );
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_string_field(&mut json, "syncScope", &self.sync_scope, false);
        write_json_bool_field(&mut json, "dryRun", self.dry_run);
        write_json_array_field(&mut json, "selectedSchemas", &self.selected_schemas);
        write_json_array_field(&mut json, "selectedTables", &self.selected_tables);
        write_json_array_field(&mut json, "addedFiles", &self.added_files);
        write_json_array_field(&mut json, "changedFiles", &self.changed_files);
        write_json_array_field(&mut json, "unchangedFiles", &self.unchanged_files);
        write_json_array_field(&mut json, "skippedFiles", &self.skipped_files);
        write_json_array_field(&mut json, "plannedCreates", &self.planned_creates);
        write_json_array_field(&mut json, "plannedUpdates", &self.planned_updates);
        write_json_array_field(&mut json, "createdFiles", &self.created_files);
        write_json_array_field(&mut json, "updatedFiles", &self.updated_files);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        write_json_array_field(
            &mut json,
            "deferredObjectTypes",
            &self.deferred_object_types,
        );
        json.push('}');
        json
    }
}

impl CompareReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Database type: {}", self.database_type).ok();
        writeln!(text, "Compare scope: {}", self.compare_scope).ok();
        writeln!(text, "Working tree: {}", self.working_tree_status.as_str()).ok();
        write_path_list(&mut text, "In-sync objects", &self.in_sync);
        write_path_list(&mut text, "Repo-different objects", &self.repo_different);
        write_path_list(&mut text, "Repo-only objects", &self.repo_only);
        write_path_list(&mut text, "Database-only objects", &self.database_only);
        write_path_list(&mut text, "Skipped objects", &self.skipped);
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
        write_repository_context_fields(
            &mut json,
            &self.repository_path,
            self.git_root.as_deref(),
            self.is_git_repository,
            self.branch.as_deref(),
            self.working_tree_status,
            self.is_dirty,
        );
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_string_field(&mut json, "compareScope", &self.compare_scope, false);
        write_json_array_field(&mut json, "selectedSchemas", &self.selected_schemas);
        write_json_array_field(&mut json, "selectedTables", &self.selected_tables);
        write_json_array_field(&mut json, "inSync", &self.in_sync);
        write_json_array_field(&mut json, "repoDifferent", &self.repo_different);
        write_json_array_field(&mut json, "repoOnly", &self.repo_only);
        write_json_array_field(&mut json, "databaseOnly", &self.database_only);
        write_json_array_field(&mut json, "skipped", &self.skipped);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        write_json_array_field(
            &mut json,
            "deferredObjectTypes",
            &self.deferred_object_types,
        );
        json.push('}');
        json
    }
}

impl PlanReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Database type: {}", self.database_type).ok();
        writeln!(text, "Plan scope: {}", self.plan_scope).ok();
        writeln!(text, "Working tree: {}", self.working_tree_status.as_str()).ok();
        write_path_list(&mut text, "Included objects", &self.included_objects);
        write_path_list(&mut text, "Excluded objects", &self.excluded_objects);
        write_plan_item_text_list(&mut text, "Ready plan items", &self.plan_items);
        write_plan_item_text_list(&mut text, "Blocked items", &self.blocked_items);
        writeln!(text, "Dependency warnings:").ok();
        for warning in &self.dependency_warnings {
            writeln!(
                text,
                "  - [{}] {}: {}",
                warning.severity, warning.warning_type, warning.message
            )
            .ok();
        }
        writeln!(
            text,
            "Compare summary: inSync={}, repoDifferent={}, repoOnly={}, databaseOnly={}, skipped={}",
            self.compare_summary.in_sync,
            self.compare_summary.repo_different,
            self.compare_summary.repo_only,
            self.compare_summary.database_only,
            self.compare_summary.skipped
        )
        .ok();
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
        write_repository_context_fields(
            &mut json,
            &self.repository_path,
            self.git_root.as_deref(),
            self.is_git_repository,
            self.branch.as_deref(),
            self.working_tree_status,
            self.is_dirty,
        );
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_string_field(&mut json, "planScope", &self.plan_scope, false);
        write_json_array_field(&mut json, "selectedSchemas", &self.selected_schemas);
        write_json_array_field(&mut json, "selectedTables", &self.selected_tables);
        write_json_array_field(&mut json, "includedObjects", &self.included_objects);
        write_json_array_field(&mut json, "excludedObjects", &self.excluded_objects);
        write_plan_item_array_field(&mut json, "planItems", &self.plan_items);
        write_plan_item_array_field(&mut json, "blockedItems", &self.blocked_items);
        write_dependency_warning_array_field(
            &mut json,
            "dependencyWarnings",
            &self.dependency_warnings,
        );
        write_compare_summary_field(&mut json, "compareSummary", &self.compare_summary);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        write_json_array_field(
            &mut json,
            "deferredObjectTypes",
            &self.deferred_object_types,
        );
        json.push('}');
        json
    }
}

impl ReleaseReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Database type: {}", self.database_type).ok();
        writeln!(text, "Release name: {}", self.release_name).ok();
        writeln!(text, "Release scope: {}", self.release_scope).ok();
        writeln!(text, "Dry run: {}", self.dry_run).ok();
        writeln!(text, "Working tree: {}", self.working_tree_status.as_str()).ok();
        writeln!(text, "Risk level: {}", self.risk_level).ok();
        write_path_list(&mut text, "Included objects", &self.included_objects);
        write_path_list(&mut text, "Excluded objects", &self.excluded_objects);
        write_plan_item_text_list(&mut text, "Plan items", &self.plan_items);
        write_plan_item_text_list(&mut text, "Blocked items", &self.blocked_items);
        write_path_list(&mut text, "Planned artifacts", &self.planned_artifacts);
        write_path_list(&mut text, "Created artifacts", &self.created_artifacts);
        writeln!(text, "Dependency warnings:").ok();
        for warning in &self.dependency_warnings {
            writeln!(
                text,
                "  - [{}] {}: {}",
                warning.severity, warning.warning_type, warning.message
            )
            .ok();
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
        write_repository_context_fields(
            &mut json,
            &self.repository_path,
            self.git_root.as_deref(),
            self.is_git_repository,
            self.branch.as_deref(),
            self.working_tree_status,
            self.is_dirty,
        );
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_string_field(&mut json, "releaseName", &self.release_name, false);
        write_json_string_field(&mut json, "releaseScope", &self.release_scope, false);
        write_json_bool_field(&mut json, "dryRun", self.dry_run);
        write_json_array_field(&mut json, "selectedSchemas", &self.selected_schemas);
        write_json_array_field(&mut json, "selectedTables", &self.selected_tables);
        write_json_array_field(&mut json, "includedObjects", &self.included_objects);
        write_json_array_field(&mut json, "excludedObjects", &self.excluded_objects);
        write_plan_item_array_field(&mut json, "planItems", &self.plan_items);
        write_plan_item_array_field(&mut json, "blockedItems", &self.blocked_items);
        write_dependency_warning_array_field(
            &mut json,
            "dependencyWarnings",
            &self.dependency_warnings,
        );
        write_json_array_field(&mut json, "plannedArtifacts", &self.planned_artifacts);
        write_json_array_field(&mut json, "createdArtifacts", &self.created_artifacts);
        write_json_string_field(&mut json, "riskLevel", &self.risk_level, false);
        write_json_array_field(&mut json, "riskReasons", &self.risk_reasons);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        write_json_array_field(
            &mut json,
            "deferredObjectTypes",
            &self.deferred_object_types,
        );
        json.push('}');
        json
    }
}

impl ReferenceDataCompareReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Database type: {}", self.database_type).ok();
        writeln!(text, "Compare scope: {}", self.compare_scope).ok();
        writeln!(text, "Working tree: {}", self.working_tree_status.as_str()).ok();
        writeln!(
            text,
            "Configured selected tables: {}",
            self.selected_tables.len()
        )
        .ok();
        writeln!(
            text,
            "Counts: inSync={}, repoDifferent={}, repoOnly={}, databaseOnly={}, skipped={}",
            self.counts.in_sync,
            self.counts.repo_different,
            self.counts.repo_only,
            self.counts.database_only,
            self.counts.skipped
        )
        .ok();
        for table in &self.table_results {
            writeln!(text, "Table: {}", table.table_name).ok();
            writeln!(
                text,
                "  Counts: inSync={}, repoDifferent={}, repoOnly={}, databaseOnly={}, skipped={}",
                table.row_counts.in_sync,
                table.row_counts.repo_different,
                table.row_counts.repo_only,
                table.row_counts.database_only,
                table.row_counts.skipped
            )
            .ok();
            for row in &table.row_results {
                writeln!(
                    text,
                    "  - {}: {} changedColumns=[{}]",
                    row.row_key,
                    row.classification,
                    row.changed_columns.join(", ")
                )
                .ok();
            }
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
        write_repository_context_fields(
            &mut json,
            &self.repository_path,
            self.git_root.as_deref(),
            self.is_git_repository,
            self.branch.as_deref(),
            self.working_tree_status,
            self.is_dirty,
        );
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_string_field(&mut json, "compareScope", &self.compare_scope, false);
        write_json_string_field(&mut json, "dataCompareScope", &self.compare_scope, false);
        write_json_array_field(&mut json, "selectedTables", &self.selected_tables);
        write_reference_table_result_array_field(&mut json, "tableResults", &self.table_results);
        write_reference_counts_field(&mut json, "counts", &self.counts);
        write_json_array_field(&mut json, "inSync", &self.in_sync);
        write_json_array_field(&mut json, "repoDifferent", &self.repo_different);
        write_json_array_field(&mut json, "repoOnly", &self.repo_only);
        write_json_array_field(&mut json, "databaseOnly", &self.database_only);
        write_json_array_field(&mut json, "skipped", &self.skipped);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        json.push('}');
        json
    }
}

fn write_path_list(text: &mut String, title: &str, paths: &[String]) {
    writeln!(text, "{title}:").ok();
    for path in paths {
        writeln!(text, "  - {path}").ok();
    }
}

fn write_plan_item_text_list(text: &mut String, title: &str, items: &[PlanItem]) {
    writeln!(text, "{title}:").ok();
    for item in items {
        writeln!(
            text,
            "  - {} ({}, {}, intent={}, blocked={})",
            item.object_ref,
            item.object_type,
            item.compare_classification,
            item.plan_intent,
            item.blocked
        )
        .ok();
        for warning in &item.warnings {
            writeln!(text, "    Warning: {warning}").ok();
        }
    }
}

fn write_json_string_field(json: &mut String, name: &str, value: &str, first: bool) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":\"{}\"", escape_json(name), escape_json(value)).ok();
}

fn write_json_optional_string_field(json: &mut String, name: &str, value: Option<&str>) {
    json.push(',');
    match value {
        Some(value) => write!(json, "\"{}\":\"{}\"", escape_json(name), escape_json(value)).ok(),
        None => write!(json, "\"{}\":null", escape_json(name)).ok(),
    };
}

fn write_repository_context_fields(
    json: &mut String,
    repository_path: &str,
    git_root: Option<&str>,
    is_git_repository: bool,
    branch: Option<&str>,
    working_tree_status: WorkingTreeStatus,
    is_dirty: bool,
) {
    write_json_string_field(json, "repositoryPath", repository_path, false);
    write_json_optional_string_field(json, "gitRoot", git_root);
    write_json_bool_field(json, "isGitRepository", is_git_repository);
    write_json_optional_string_field(json, "branch", branch);
    write_json_string_field(
        json,
        "workingTreeStatus",
        working_tree_status.as_str(),
        false,
    );
    write_json_bool_field(json, "isDirty", is_dirty);
}

fn write_json_bool_field(json: &mut String, name: &str, value: bool) {
    json.push(',');
    write!(json, "\"{}\":{}", escape_json(name), value).ok();
}

fn write_json_i32_field(json: &mut String, name: &str, value: i32, first: bool) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":{}", escape_json(name), value).ok();
}

fn write_json_i64_field(json: &mut String, name: &str, value: i64, first: bool) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":{}", escape_json(name), value).ok();
}

fn write_json_array_field(json: &mut String, name: &str, values: &[String]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(json, "\"{}\"", escape_json(value)).ok();
    }
    json.push(']');
}

fn write_schema_array_field(json: &mut String, name: &str, values: &[SchemaInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "name", &value.name, true);
        json.push('}');
    }
    json.push(']');
}

fn write_table_array_field(json: &mut String, name: &str, values: &[TableInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "tableName", &value.table_name, false);
        write_json_string_field(json, "tableType", &value.table_type, false);
        json.push('}');
    }
    json.push(']');
}

fn write_column_array_field(json: &mut String, name: &str, values: &[ColumnInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "tableName", &value.table_name, false);
        write_json_string_field(json, "columnName", &value.column_name, false);
        write_json_i32_field(json, "ordinalPosition", value.ordinal_position, false);
        write_json_string_field(json, "dataType", &value.data_type, false);
        write_json_bool_field(json, "isNullable", value.is_nullable);
        write_json_bool_field(json, "hasDefault", value.has_default);
        json.push('}');
    }
    json.push(']');
}

fn write_extension_array_field(json: &mut String, name: &str, values: &[ExtensionInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "extensionName", &value.extension_name, true);
        if let Some(schema) = &value.schema_name {
            write_json_string_field(json, "schemaName", schema, false);
        }
        if let Some(version) = &value.version {
            write_json_string_field(json, "version", version, false);
        }
        json.push('}');
    }
    json.push(']');
}

fn write_enum_array_field(json: &mut String, name: &str, values: &[EnumInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "enumName", &value.enum_name, false);
        write_json_array_field(json, "labels", &value.labels);
        json.push('}');
    }
    json.push(']');
}

fn write_sequence_array_field(json: &mut String, name: &str, values: &[SequenceInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "sequenceName", &value.sequence_name, false);
        if let Some(data_type) = &value.data_type {
            write_json_string_field(json, "dataType", data_type, false);
        }
        if let Some(start_value) = value.start_value {
            write_json_i64_field(json, "startValue", start_value, false);
        }
        if let Some(min_value) = value.min_value {
            write_json_i64_field(json, "minValue", min_value, false);
        }
        if let Some(max_value) = value.max_value {
            write_json_i64_field(json, "maxValue", max_value, false);
        }
        if let Some(increment_by) = value.increment_by {
            write_json_i64_field(json, "incrementBy", increment_by, false);
        }
        if let Some(cache_size) = value.cache_size {
            write_json_i64_field(json, "cacheSize", cache_size, false);
        }
        write_json_bool_field(json, "cycle", value.cycle);
        json.push('}');
    }
    json.push(']');
}

fn write_index_array_field(json: &mut String, name: &str, values: &[IndexInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "tableName", &value.table_name, false);
        write_json_string_field(json, "indexName", &value.index_name, false);
        write_json_bool_field(json, "isUnique", value.is_unique);
        json.push('}');
    }
    json.push(']');
}

fn write_view_array_field(json: &mut String, name: &str, values: &[ViewInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "viewName", &value.view_name, false);
        json.push('}');
    }
    json.push(']');
}

fn write_counts_field(json: &mut String, name: &str, counts: &InspectionCounts) {
    json.push(',');
    write!(
        json,
        "\"{}\":{{\"schemas\":{},\"tables\":{},\"columns\":{},\"extensions\":{},\"enums\":{},\"sequences\":{},\"indexes\":{},\"views\":{}}}",
        escape_json(name),
        counts.schemas,
        counts.tables,
        counts.columns,
        counts.extensions,
        counts.enums,
        counts.sequences,
        counts.indexes,
        counts.views
    )
    .ok();
}

fn write_plan_item_array_field(json: &mut String, name: &str, values: &[PlanItem]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "objectRef", &value.object_ref, true);
        write_json_string_field(json, "objectType", &value.object_type, false);
        write_json_string_field(json, "relativePath", &value.relative_path, false);
        write_json_string_field(
            json,
            "compareClassification",
            &value.compare_classification,
            false,
        );
        write_json_string_field(json, "planIntent", &value.plan_intent, false);
        write_json_bool_field(json, "selected", value.selected);
        write_json_bool_field(json, "blocked", value.blocked);
        write_json_array_field(json, "warnings", &value.warnings);
        json.push('}');
    }
    json.push(']');
}

fn write_dependency_warning_array_field(
    json: &mut String,
    name: &str,
    values: &[DependencyWarning],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "warningType", &value.warning_type, true);
        write_json_string_field(json, "objectRef", &value.object_ref, false);
        write_json_optional_string_field(
            json,
            "requiredObjectRef",
            value.required_object_ref.as_deref(),
        );
        write_json_string_field(json, "message", &value.message, false);
        write_json_string_field(json, "severity", &value.severity, false);
        json.push('}');
    }
    json.push(']');
}

fn write_compare_summary_field(json: &mut String, name: &str, summary: &CompareSummary) {
    json.push(',');
    write!(
        json,
        "\"{}\":{{\"inSync\":{},\"repoDifferent\":{},\"repoOnly\":{},\"databaseOnly\":{},\"skipped\":{}}}",
        escape_json(name),
        summary.in_sync,
        summary.repo_different,
        summary.repo_only,
        summary.database_only,
        summary.skipped
    )
    .ok();
}

fn write_reference_counts_field(
    json: &mut String,
    name: &str,
    counts: &ReferenceDataCompareCounts,
) {
    json.push(',');
    write!(
        json,
        "\"{}\":{{\"inSync\":{},\"repoDifferent\":{},\"repoOnly\":{},\"databaseOnly\":{},\"skipped\":{}}}",
        escape_json(name),
        counts.in_sync,
        counts.repo_different,
        counts.repo_only,
        counts.database_only,
        counts.skipped
    )
    .ok();
}

fn write_reference_table_result_array_field(
    json: &mut String,
    name: &str,
    values: &[ReferenceTableCompareResult],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "tableName", &value.table_name, true);
        write_json_array_field(json, "keyColumns", &value.key_columns);
        write_json_array_field(json, "comparedColumns", &value.compared_columns);
        write_json_array_field(json, "ignoredColumns", &value.ignored_columns);
        write_json_array_field(json, "maskedColumns", &value.masked_columns);
        write_reference_counts_field(json, "rowCounts", &value.row_counts);
        write_reference_row_result_array_field(json, "rowResults", &value.row_results);
        write_json_array_field(json, "warnings", &value.warnings);
        write_json_array_field(json, "errors", &value.errors);
        json.push('}');
    }
    json.push(']');
}

fn write_reference_row_result_array_field(
    json: &mut String,
    name: &str,
    values: &[ReferenceRowCompareResult],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "tableName", &value.table_name, true);
        write_json_string_field(json, "rowKey", &value.row_key, false);
        write_json_string_field(json, "classification", &value.classification, false);
        write_json_array_field(json, "changedColumns", &value.changed_columns);
        write_json_array_field(json, "maskedColumns", &value.masked_columns);
        write_json_array_field(json, "warnings", &value.warnings);
        json.push('}');
    }
    json.push(']');
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                write!(escaped, "\\u{:04x}", character as u32).ok();
            }
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn temp_path(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("dbstate-{name}-{}-{now}", std::process::id()))
    }

    fn create_temp_dir(name: &str) -> PathBuf {
        let path = temp_path(name);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn init_git_repo(path: &Path) {
        let output = Command::new("git")
            .arg("init")
            .current_dir(path)
            .output()
            .expect("run git init");
        assert!(output.status.success(), "git init failed");
    }

    fn commit_all(path: &Path, message: &str) {
        let add = Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(path)
            .output()
            .expect("run git add");
        assert!(add.status.success(), "git add failed");

        let commit = Command::new("git")
            .arg("-c")
            .arg("user.email=dbstate@example.invalid")
            .arg("-c")
            .arg("user.name=DbState Test")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .current_dir(path)
            .output()
            .expect("run git commit");
        assert!(commit.status.success(), "git commit failed");
    }

    fn create_complete_structure(root: &Path) {
        for expected in EXPECTED_PATHS {
            let target = root.join(expected.relative);
            match expected.kind {
                PathKind::Directory => fs::create_dir_all(target).expect("create directory"),
                PathKind::File => {
                    fs::create_dir_all(target.parent().expect("file has parent"))
                        .expect("create parent");
                    fs::write(target, DEFAULT_REGISTRY).expect("create registry");
                }
            }
        }
    }

    fn placeholder_url(user: &str, credential: &str) -> String {
        format!("{}://{user}:{credential}@example.invalid/db", "postgres")
    }

    fn sample_inventory() -> PostgresInventory {
        PostgresInventory {
            schemas: vec![SchemaInfo {
                name: "dbstate_slice2".to_string(),
            }],
            tables: vec![TableInfo {
                schema_name: "dbstate_slice2".to_string(),
                table_name: "sample_accounts".to_string(),
                table_type: "BASE TABLE".to_string(),
            }],
            columns: vec![
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "account_id".to_string(),
                    ordinal_position: 1,
                    data_type: "integer".to_string(),
                    is_nullable: false,
                    has_default: false,
                    default_expression: None,
                },
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "account_code".to_string(),
                    ordinal_position: 2,
                    data_type: "text".to_string(),
                    is_nullable: false,
                    has_default: false,
                    default_expression: None,
                },
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "display_name".to_string(),
                    ordinal_position: 3,
                    data_type: "text".to_string(),
                    is_nullable: true,
                    has_default: false,
                    default_expression: None,
                },
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "created_at".to_string(),
                    ordinal_position: 4,
                    data_type: "timestamp without time zone".to_string(),
                    is_nullable: false,
                    has_default: true,
                    default_expression: Some("now()".to_string()),
                },
            ],
            extensions: vec![ExtensionInfo {
                extension_name: "pgcrypto".to_string(),
                schema_name: Some("public".to_string()),
                version: Some("1.3".to_string()),
            }],
            enums: vec![EnumInfo {
                schema_name: "dbstate_slice2".to_string(),
                enum_name: "account_status".to_string(),
                labels: vec!["active".to_string(), "closed".to_string()],
            }],
            sequences: vec![SequenceInfo {
                schema_name: "dbstate_slice2".to_string(),
                sequence_name: "account_number_seq".to_string(),
                data_type: Some("bigint".to_string()),
                start_value: Some(1),
                min_value: Some(1),
                max_value: Some(9_223_372_036_854_775_807),
                increment_by: Some(1),
                cycle: false,
                cache_size: Some(1),
            }],
            indexes: vec![IndexInfo {
                schema_name: "dbstate_slice2".to_string(),
                table_name: "sample_accounts".to_string(),
                index_name: "sample_accounts_account_code_idx".to_string(),
                is_unique: true,
                definition:
                    "CREATE UNIQUE INDEX sample_accounts_account_code_idx ON dbstate_slice2.sample_accounts USING btree (account_code)"
                        .to_string(),
            }],
            views: vec![ViewInfo {
                schema_name: "dbstate_slice2".to_string(),
                view_name: "active_accounts".to_string(),
                definition:
                    " SELECT sample_accounts.account_id,\n    sample_accounts.account_code\n   FROM dbstate_slice2.sample_accounts"
                        .to_string(),
            }],
        }
    }

    fn valid_reference_registry_yaml() -> &'static str {
        "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
    key:
      - code
    ignoreColumns:
      - updated_at
    maskedColumns:
      - secret_note
    allowDeletes: false
"
    }

    fn valid_reference_table_yaml() -> &'static str {
        "table: dbstate_ref.payment_methods
key:
  - code
rows:
  - code: CASH
    name: Cash
    is_active: true
    sort_order: 10
    updated_at: repo timestamp ignored
    secret_note: repo-secret-cash
  - code: QRPH
    name: QRPh Desired
    is_active: true
    sort_order: 20
    updated_at: repo timestamp ignored
    secret_note: repo-secret-qrph
  - code: WIRE
    name: Wire
    is_active: false
    sort_order: 40
    updated_at: repo timestamp ignored
    secret_note: repo-secret-wire
"
    }

    fn reference_config_and_state() -> (ReferenceDataTableConfig, ReferenceDataTableState) {
        let registry =
            parse_reference_data_registry(valid_reference_registry_yaml()).expect("registry");
        let config = registry.tables[0].clone();
        let state =
            parse_reference_data_table_state(valid_reference_table_yaml(), &config).expect("state");
        (config, state)
    }

    fn reference_database_columns() -> BTreeSet<String> {
        [
            "code",
            "name",
            "is_active",
            "sort_order",
            "updated_at",
            "secret_note",
        ]
        .into_iter()
        .map(|value| value.to_string())
        .collect()
    }

    fn reference_row(values: &[(&str, Option<&str>)]) -> ReferenceDataRow {
        ReferenceDataRow {
            values: values
                .iter()
                .map(|(key, value)| (key.to_string(), value.map(|value| value.to_string())))
                .collect(),
        }
    }

    #[test]
    fn non_git_folder_returns_clear_status() {
        let dir = create_temp_dir("non-git");
        let report = status_report(&dir, CommandKind::RepoStatus);

        assert!(!report.success);
        assert!(!report.is_git_repository);
        assert_eq!(
            report.dbstate_project_status,
            DbStateProjectStatus::NotGitRepository
        );
        assert!(report
            .errors
            .contains(&"Current path is not inside a Git repository.".to_string()));
    }

    #[test]
    fn git_repo_without_structure_reports_missing_paths() {
        let dir = create_temp_dir("empty-git");
        init_git_repo(&dir);

        let report = status_report(&dir, CommandKind::RepoStatus);

        assert!(report.success);
        assert!(report.is_git_repository);
        assert_eq!(
            report.dbstate_project_status,
            DbStateProjectStatus::GitRepositoryWithoutDbStateStructure
        );
        assert_eq!(report.missing_paths.len(), EXPECTED_PATHS.len());
    }

    #[test]
    fn partial_structure_reports_only_missing_paths() {
        let dir = create_temp_dir("partial-git");
        init_git_repo(&dir);
        fs::create_dir_all(dir.join("database/objects/schemas")).expect("create partial");

        let report = status_report(&dir, CommandKind::RepoStatus);

        assert_eq!(
            report.dbstate_project_status,
            DbStateProjectStatus::PartialDbStateStructure
        );
        assert!(report
            .existing_paths
            .contains(&"database/objects/schemas".to_string()));
        assert!(report
            .missing_paths
            .contains(&"database/releases".to_string()));
    }

    #[test]
    fn complete_structure_reports_complete_status() {
        let dir = create_temp_dir("complete-git");
        init_git_repo(&dir);
        create_complete_structure(&dir);

        let report = status_report(&dir, CommandKind::RepoStatus);

        assert_eq!(
            report.dbstate_project_status,
            DbStateProjectStatus::CompleteDbStateStructure
        );
        assert!(report.missing_paths.is_empty());
    }

    #[test]
    fn dry_run_init_reports_planned_creates_but_creates_nothing() {
        let dir = create_temp_dir("dry-run");
        init_git_repo(&dir);

        let report = init_project(&dir, true).expect("dry-run init");

        assert!(report.success);
        assert!(!report.planned_creates.is_empty());
        assert!(report.created_paths.is_empty());
        assert!(!dir.join("database").exists());
    }

    #[test]
    fn init_creates_only_missing_folders_and_files() {
        let dir = create_temp_dir("init");
        init_git_repo(&dir);

        let report = init_project(&dir, false).expect("init project");

        assert!(report.success);
        assert_eq!(
            report.dbstate_project_status,
            DbStateProjectStatus::CompleteDbStateStructure
        );
        assert!(dir.join("database/objects/tables").is_dir());
        assert!(dir
            .join("database/reference-data/dbstate.reference-data.yml")
            .is_file());
        assert_eq!(
            fs::read_to_string(dir.join("database/reference-data/dbstate.reference-data.yml"))
                .expect("read registry"),
            DEFAULT_REGISTRY
        );
    }

    #[test]
    fn init_does_not_overwrite_existing_registry() {
        let dir = create_temp_dir("no-overwrite");
        init_git_repo(&dir);
        let registry = dir.join("database/reference-data/dbstate.reference-data.yml");
        fs::create_dir_all(registry.parent().expect("registry parent")).expect("create parent");
        fs::write(&registry, "version: 1\ntables:\n  - name: public.keep_me\n")
            .expect("write existing registry");
        commit_all(&dir, "existing registry");

        let report = init_project(&dir, false).expect("init project");

        assert!(report.success);
        assert_eq!(
            fs::read_to_string(registry).expect("read registry"),
            "version: 1\ntables:\n  - name: public.keep_me\n"
        );
    }

    #[test]
    fn init_does_not_create_secret_or_connection_files() {
        let dir = create_temp_dir("no-secrets");
        init_git_repo(&dir);

        init_project(&dir, false).expect("init project");

        let forbidden = [
            "database/connection.yml",
            "database/connections.yml",
            "database/secrets.yml",
            "database/.env",
            "database/reference-data/credentials.yml",
        ];
        for relative in forbidden {
            assert!(!dir.join(relative).exists(), "{relative} should not exist");
        }
    }

    #[test]
    fn dirty_working_tree_warning_is_reported_and_init_is_blocked() {
        let dir = create_temp_dir("dirty");
        init_git_repo(&dir);
        fs::write(dir.join("untracked.txt"), "dirty").expect("write dirty file");

        let status = status_report(&dir, CommandKind::RepoStatus);
        assert!(status.is_dirty);
        assert!(!status.warnings.is_empty());

        let init = init_project(&dir, false).expect("init project");
        assert!(!init.success);
        assert!(init.created_paths.is_empty());
        assert!(!dir.join("database").exists());
    }

    #[test]
    fn project_json_output_includes_expected_fields() {
        let dir = create_temp_dir("json");
        init_git_repo(&dir);

        let report = status_report(&dir, CommandKind::RepoStatus);
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"repositoryPath\"",
            "\"gitRoot\"",
            "\"isGitRepository\"",
            "\"branch\"",
            "\"workingTreeStatus\"",
            "\"isDirty\"",
            "\"dbstateProjectStatus\"",
            "\"missingPaths\"",
            "\"existingPaths\"",
            "\"plannedCreates\"",
            "\"createdPaths\"",
            "\"warnings\"",
            "\"errors\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }
    }

    #[test]
    fn missing_postgres_url_returns_clear_error() {
        let report = inspect_postgres_command(None, None);

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Missing PostgreSQL connection URL")));
    }

    #[test]
    fn url_precedence_prefers_cli_url() {
        let cli_url = placeholder_url("cli-user", "cli-credential");
        let env_url = placeholder_url("env-user", "env-credential");
        let resolved =
            resolve_postgres_url(Some(cli_url.clone()), Some(env_url)).expect("resolved url");

        assert_eq!(resolved, cli_url);
    }

    #[test]
    fn redaction_removes_raw_url_and_credential() {
        let credential = ["sensitive", "marker"].join("-");
        let raw = placeholder_url("user", &credential);
        let redacted = redact_message(&format!("could not connect to {raw}"), &raw);

        assert!(!redacted.contains(&raw));
        assert!(!redacted.contains(&credential));
        assert!(redacted.contains("<redacted>"));
    }

    #[test]
    fn inspection_json_output_includes_expected_fields_and_no_secrets() {
        let mut report = empty_inspection_report(CommandKind::InspectPostgres);
        let raw = format!("{} failed", placeholder_url("user", "sensitive-marker"));
        report.errors.push(redact_postgres_url(&raw));
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"databaseType\"",
            "\"inspectionScope\"",
            "\"schemas\"",
            "\"tables\"",
            "\"columns\"",
            "\"extensions\"",
            "\"enums\"",
            "\"sequences\"",
            "\"indexes\"",
            "\"views\"",
            "\"counts\"",
            "\"warnings\"",
            "\"errors\"",
            "\"deferredObjectTypes\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }

        assert!(!json.contains("sensitive-marker"));
        assert!(!json.contains("postgres://"));
    }

    #[test]
    fn inspect_command_does_not_create_repository_files() {
        let dir = create_temp_dir("inspect-no-files");
        init_git_repo(&dir);

        let before_database_exists = dir.join("database").exists();
        let report = inspect_postgres_command(None, None);

        assert!(!report.success);
        assert_eq!(before_database_exists, dir.join("database").exists());
    }

    #[test]
    fn internal_schema_filtering_excludes_postgresql_schemas() {
        assert!(!is_user_schema("pg_catalog"));
        assert!(!is_user_schema("information_schema"));
        assert!(!is_user_schema("pg_toast"));
        assert!(!is_user_schema("pg_toast_temp_1"));
        assert!(!is_user_schema("pg_temp_1"));
        assert!(is_user_schema("public"));
        assert!(is_user_schema("app_core"));
    }

    #[test]
    fn object_inventory_model_represents_schemas_tables_and_columns() {
        let inventory = PostgresInventory {
            schemas: vec![SchemaInfo {
                name: "app".to_string(),
            }],
            tables: vec![TableInfo {
                schema_name: "app".to_string(),
                table_name: "orders".to_string(),
                table_type: "BASE TABLE".to_string(),
            }],
            columns: vec![ColumnInfo {
                schema_name: "app".to_string(),
                table_name: "orders".to_string(),
                column_name: "id".to_string(),
                ordinal_position: 1,
                data_type: "integer".to_string(),
                is_nullable: false,
                has_default: true,
                default_expression: Some("nextval('orders_id_seq'::regclass)".to_string()),
            }],
            extensions: vec![ExtensionInfo {
                extension_name: "pgcrypto".to_string(),
                schema_name: Some("public".to_string()),
                version: Some("1.3".to_string()),
            }],
            enums: vec![EnumInfo {
                schema_name: "app".to_string(),
                enum_name: "order_status".to_string(),
                labels: vec!["new".to_string(), "paid".to_string()],
            }],
            sequences: vec![SequenceInfo {
                schema_name: "app".to_string(),
                sequence_name: "orders_id_seq".to_string(),
                data_type: Some("integer".to_string()),
                start_value: Some(1),
                min_value: Some(1),
                max_value: Some(2_147_483_647),
                increment_by: Some(1),
                cycle: false,
                cache_size: Some(1),
            }],
            indexes: vec![IndexInfo {
                schema_name: "app".to_string(),
                table_name: "orders".to_string(),
                index_name: "orders_created_at_idx".to_string(),
                is_unique: false,
                definition: "CREATE INDEX orders_created_at_idx ON app.orders USING btree (id)"
                    .to_string(),
            }],
            views: vec![ViewInfo {
                schema_name: "app".to_string(),
                view_name: "open_orders".to_string(),
                definition: " SELECT orders.id FROM app.orders".to_string(),
            }],
        };

        assert_eq!(inventory.schemas[0].name, "app");
        assert_eq!(inventory.tables[0].table_name, "orders");
        assert_eq!(inventory.columns[0].column_name, "id");
        assert_eq!(inventory.extensions[0].extension_name, "pgcrypto");
        assert_eq!(inventory.enums[0].labels, vec!["new", "paid"]);
        assert_eq!(inventory.sequences[0].sequence_name, "orders_id_seq");
        assert_eq!(inventory.indexes[0].index_name, "orders_created_at_idx");
        assert_eq!(inventory.views[0].view_name, "open_orders");
    }

    #[test]
    fn deferred_object_types_are_explicit() {
        let report = empty_inspection_report(CommandKind::InspectPostgres);

        assert!(!report
            .deferred_object_types
            .contains(&"extensions".to_string()));
        assert!(!report.deferred_object_types.contains(&"enums".to_string()));
        assert!(!report
            .deferred_object_types
            .contains(&"sequences".to_string()));
        assert!(!report
            .deferred_object_types
            .contains(&"indexes".to_string()));
        assert!(!report.deferred_object_types.contains(&"views".to_string()));
        assert!(report
            .deferred_object_types
            .contains(&"functions".to_string()));
        assert!(report.deferred_object_types.contains(&"grants".to_string()));
    }

    #[test]
    fn missing_export_selection_returns_clear_error() {
        let error = ExportSelection::from_options(false, None, None).expect_err("selection error");
        assert!(error.contains("Selection is required"));
    }

    #[test]
    fn export_paths_stay_under_database_objects() {
        assert_eq!(
            schema_file_path("core").expect("schema path"),
            "database/objects/schemas/core.sql"
        );
        assert_eq!(
            table_file_path("core", "payment_attempts").expect("table path"),
            "database/objects/tables/core.payment_attempts.sql"
        );
        assert_eq!(
            extension_file_path("pgcrypto").expect("extension path"),
            "database/objects/extensions/pgcrypto.sql"
        );
        assert_eq!(
            enum_file_path("core", "payment_status").expect("enum path"),
            "database/objects/enums/core.payment_status.sql"
        );
        assert_eq!(
            sequence_file_path("core", "payment_id_seq").expect("sequence path"),
            "database/objects/sequences/core.payment_id_seq.sql"
        );
        assert_eq!(
            index_file_path("core", "payments", "payments_code_idx").expect("index path"),
            "database/objects/indexes/core.payments.payments_code_idx.sql"
        );
        assert_eq!(
            view_file_path("core", "active_payments").expect("view path"),
            "database/objects/views/core.active_payments.sql"
        );
        assert!(schema_file_path("../evil").is_err());
        assert!(table_file_path("core", "bad/name").is_err());
        assert!(ensure_database_object_path("database/releases/bad.sql").is_err());
    }

    #[test]
    fn identifier_quoting_handles_required_cases() {
        assert_eq!(quote_postgres_identifier("normal"), "\"normal\"");
        assert_eq!(quote_postgres_identifier("MixedCase"), "\"MixedCase\"");
        assert_eq!(quote_postgres_identifier("select"), "\"select\"");
        assert_eq!(quote_postgres_identifier("has\"quote"), "\"has\"\"quote\"");
    }

    #[test]
    fn generated_schema_sql_matches_golden_expectation() {
        let expected = "-- DbState PostgreSQL desired-state object\n-- Object type: schema\n-- Object name: dbstate_slice2\n\nCREATE SCHEMA \"dbstate_slice2\";\n";
        assert_eq!(render_schema_sql("dbstate_slice2"), expected);
    }

    #[test]
    fn generated_table_sql_matches_golden_expectation() {
        let expected = "-- DbState PostgreSQL desired-state object\n-- Object type: table\n-- Object name: dbstate_slice2.sample_accounts\n\nCREATE TABLE \"dbstate_slice2\".\"sample_accounts\" (\n    \"account_id\" integer NOT NULL,\n    \"account_code\" text NOT NULL,\n    \"display_name\" text,\n    \"created_at\" timestamp without time zone DEFAULT now() NOT NULL\n);\n";
        assert_eq!(
            render_table_sql(
                "dbstate_slice2",
                "sample_accounts",
                &sample_inventory().columns
            ),
            expected
        );
    }

    #[test]
    fn generated_slice16_object_sql_is_deterministic() {
        let inventory = sample_inventory();

        assert!(render_extension_sql(&inventory.extensions[0])
            .contains("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\";"));
        assert!(render_enum_sql(&inventory.enums[0])
            .contains("CREATE TYPE \"dbstate_slice2\".\"account_status\" AS ENUM"));
        assert!(render_enum_sql(&inventory.enums[0]).contains("'active',"));
        assert!(render_sequence_sql(&inventory.sequences[0])
            .contains("CREATE SEQUENCE \"dbstate_slice2\".\"account_number_seq\""));
        assert!(render_index_sql(&inventory.indexes[0])
            .contains("CREATE UNIQUE INDEX sample_accounts_account_code_idx"));
        assert!(render_view_sql(&inventory.views[0])
            .contains("CREATE VIEW \"dbstate_slice2\".\"active_accounts\" AS"));
    }

    #[test]
    fn export_dry_run_creates_no_files() {
        let dir = create_temp_dir("export-dry-run");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);

        assert!(report.success);
        assert!(!report.planned_files.is_empty());
        assert!(report.created_files.is_empty());
        assert!(!dir
            .join("database/objects/schemas/dbstate_slice2.sql")
            .exists());
    }

    #[test]
    fn export_requires_git_repository_and_dbstate_structure() {
        let non_git = create_temp_dir("export-non-git");
        let report = export_postgres_with_inventory(
            &non_git,
            &sample_inventory(),
            &ExportSelection::All,
            false,
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("not inside a Git repository"));

        let no_structure = create_temp_dir("export-no-structure");
        init_git_repo(&no_structure);
        let report = export_postgres_with_inventory(
            &no_structure,
            &sample_inventory(),
            &ExportSelection::All,
            false,
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("Run dbstate init first"));
    }

    #[test]
    fn export_write_is_blocked_when_working_tree_is_dirty() {
        let dir = create_temp_dir("export-dirty");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

        let report =
            export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

        assert!(!report.success);
        assert!(report.errors[0].contains("working tree has changes"));
        assert!(report.created_files.is_empty());
    }

    #[test]
    fn export_does_not_overwrite_existing_files_by_default() {
        let dir = create_temp_dir("export-no-overwrite");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        let existing = dir.join("database/objects/schemas/dbstate_slice2.sql");
        fs::write(&existing, "-- keep me\n").expect("write existing schema file");
        commit_all(&dir, "complete structure");

        let report = export_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Schema("dbstate_slice2".to_string()),
            true,
        );

        assert!(report.success);
        assert!(report
            .skipped_files
            .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
        assert_eq!(
            fs::read_to_string(existing).expect("read existing"),
            "-- keep me\n"
        );
    }

    #[test]
    fn table_export_warns_when_schema_file_is_missing() {
        let dir = create_temp_dir("export-table-warning");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report = export_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
            true,
        );

        assert!(report.success);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("without its schema object file")));
    }

    #[test]
    fn export_json_includes_expected_fields_and_no_secrets() {
        let dir = create_temp_dir("export-json");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"databaseType\"",
            "\"exportScope\"",
            "\"dryRun\"",
            "\"selectedSchemas\"",
            "\"selectedTables\"",
            "\"plannedFiles\"",
            "\"createdFiles\"",
            "\"skippedFiles\"",
            "\"warnings\"",
            "\"errors\"",
            "\"deferredObjectTypes\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }
        assert!(!json.contains("postgres://"));
        assert!(!json.contains("sensitive-marker"));
    }

    #[test]
    fn actual_export_creates_schema_and_table_files() {
        let dir = create_temp_dir("export-write");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

        assert!(report.success);
        assert!(dir
            .join("database/objects/schemas/dbstate_slice2.sql")
            .is_file());
        assert!(dir
            .join("database/objects/tables/dbstate_slice2.sample_accounts.sql")
            .is_file());
    }

    #[test]
    fn sync_dry_run_creates_or_updates_no_files() {
        let dir = create_temp_dir("sync-dry-run");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);

        assert!(report.success);
        assert!(report
            .planned_creates
            .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
        assert!(report.created_files.is_empty());
        assert!(report.updated_files.is_empty());
        assert!(!dir
            .join("database/objects/schemas/dbstate_slice2.sql")
            .exists());
    }

    #[test]
    fn sync_requires_git_repository_and_dbstate_structure() {
        let non_git = create_temp_dir("sync-non-git");
        let report = sync_postgres_with_inventory(
            &non_git,
            &sample_inventory(),
            &ExportSelection::All,
            false,
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("not inside a Git repository"));

        let no_structure = create_temp_dir("sync-no-structure");
        init_git_repo(&no_structure);
        let report = sync_postgres_with_inventory(
            &no_structure,
            &sample_inventory(),
            &ExportSelection::All,
            false,
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("Run dbstate init first"));
    }

    #[test]
    fn sync_write_is_blocked_when_working_tree_is_dirty() {
        let dir = create_temp_dir("sync-dirty");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

        let report =
            sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

        assert!(!report.success);
        assert!(report.errors[0].contains("working tree has changes"));
        assert!(report.created_files.is_empty());
        assert!(report.updated_files.is_empty());
    }

    #[test]
    fn sync_creates_added_object_files() {
        let dir = create_temp_dir("sync-create");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

        assert!(report.success);
        assert!(report
            .created_files
            .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
        assert!(report
            .created_files
            .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    }

    #[test]
    fn sync_updates_changed_object_files() {
        let dir = create_temp_dir("sync-update");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        let table_path = dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
        fs::write(&table_path, "-- stale table definition\n").expect("write stale table file");
        commit_all(&dir, "stale table");

        let report = sync_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
            false,
        );

        assert!(report.success);
        assert!(report
            .updated_files
            .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
        assert_eq!(
            fs::read_to_string(table_path).expect("read updated table"),
            render_table_sql(
                "dbstate_slice2",
                "sample_accounts",
                &sample_inventory().columns
            )
        );
    }

    #[test]
    fn sync_leaves_unchanged_files_untouched() {
        let dir = create_temp_dir("sync-unchanged");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        let schema_path = dir.join("database/objects/schemas/dbstate_slice2.sql");
        fs::write(&schema_path, render_schema_sql("dbstate_slice2")).expect("write schema file");
        commit_all(&dir, "schema file");

        let report = sync_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Schema("dbstate_slice2".to_string()),
            true,
        );

        assert!(report.success);
        assert!(report
            .unchanged_files
            .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    }

    #[test]
    fn sync_never_writes_under_releases() {
        let dir = create_temp_dir("sync-no-releases");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

        assert!(report.success);
        assert!(report
            .created_files
            .iter()
            .all(|path| path.starts_with("database/objects/")));
        assert!(fs::read_dir(dir.join("database/releases"))
            .expect("read releases")
            .next()
            .is_none());
    }

    #[test]
    fn sync_json_includes_expected_fields_and_no_secrets() {
        let dir = create_temp_dir("sync-json");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"databaseType\"",
            "\"syncScope\"",
            "\"dryRun\"",
            "\"selectedSchemas\"",
            "\"selectedTables\"",
            "\"addedFiles\"",
            "\"changedFiles\"",
            "\"unchangedFiles\"",
            "\"skippedFiles\"",
            "\"plannedCreates\"",
            "\"plannedUpdates\"",
            "\"createdFiles\"",
            "\"updatedFiles\"",
            "\"warnings\"",
            "\"errors\"",
            "\"deferredObjectTypes\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }
        assert!(!json.contains("postgres://"));
        assert!(!json.contains("sensitive-marker"));
    }

    #[test]
    fn compare_missing_selection_returns_clear_error() {
        let dir = create_temp_dir("compare-missing-selection");
        let parsed =
            ParsedArgs::parse(&["compare".to_string(), "postgres".to_string()]).expect("parse");

        let report = compare_postgres_command(&dir, parsed);

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Selection is required")));
    }

    #[test]
    fn compare_requires_git_repository_and_dbstate_structure() {
        let non_git = create_temp_dir("compare-non-git");
        let report =
            compare_postgres_with_inventory(&non_git, &sample_inventory(), &ExportSelection::All);
        assert!(!report.success);
        assert!(report.errors[0].contains("not inside a Git repository"));

        let no_structure = create_temp_dir("compare-no-structure");
        init_git_repo(&no_structure);
        let report = compare_postgres_with_inventory(
            &no_structure,
            &sample_inventory(),
            &ExportSelection::All,
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("Run dbstate init first"));
    }

    #[test]
    fn compare_is_read_only_and_can_run_with_dirty_working_tree() {
        let dir = create_temp_dir("compare-dirty-readonly");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

        let report =
            compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

        assert!(report.success);
        assert!(report.is_dirty);
        assert!(report.in_sync.is_empty());
        assert!(report.repo_different.is_empty());
        assert!(report.repo_only.is_empty());
        assert!(!report.database_only.is_empty());
        assert!(!dir
            .join("database/objects/schemas/dbstate_slice2.sql")
            .exists());
    }

    #[test]
    fn repository_schema_and_table_files_are_discovered() {
        let dir = create_temp_dir("compare-discover");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/dbstate_slice2.sql"),
            render_schema_sql("dbstate_slice2"),
        )
        .expect("write schema");
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            render_table_sql(
                "dbstate_slice2",
                "sample_accounts",
                &sample_inventory().columns,
            ),
        )
        .expect("write table");

        let import = discover_repository_objects(&dir).expect("discover objects");

        assert!(import.objects.contains_key("schema:dbstate_slice2"));
        assert!(import
            .objects
            .contains_key("table:dbstate_slice2.sample_accounts"));
        assert!(import.skipped.is_empty());
    }

    #[test]
    fn repository_invalid_file_names_are_reported_as_skipped() {
        let dir = create_temp_dir("compare-invalid-names");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(dir.join("database/objects/tables/bad.txt"), "-- bad\n")
            .expect("write bad table file");
        fs::write(dir.join("database/objects/tables/a.b.c.sql"), "-- bad\n")
            .expect("write bad table file");

        let import = discover_repository_objects(&dir).expect("discover objects");

        assert!(import
            .skipped
            .contains(&"database/objects/tables/bad.txt".to_string()));
        assert!(import
            .skipped
            .contains(&"database/objects/tables/a.b.c.sql".to_string()));
    }

    #[test]
    fn compare_text_normalization_is_deterministic() {
        assert_eq!(
            normalize_desired_state_text("line one  \r\nline two\r\n\r\n"),
            "line one\nline two\n"
        );
        assert_eq!(normalize_desired_state_text(""), "");
    }

    #[test]
    fn compare_classifies_in_sync_objects() {
        let dir = create_temp_dir("compare-in-sync");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/dbstate_slice2.sql"),
            render_schema_sql("dbstate_slice2"),
        )
        .expect("write schema");
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            render_table_sql(
                "dbstate_slice2",
                "sample_accounts",
                &sample_inventory().columns,
            ),
        )
        .expect("write table");
        commit_all(&dir, "desired state files");

        let report =
            compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

        assert!(report.success);
        assert!(report
            .in_sync
            .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
        assert!(report
            .in_sync
            .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    }

    #[test]
    fn compare_classifies_repo_different_objects() {
        let dir = create_temp_dir("compare-different");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            "-- stale table\n",
        )
        .expect("write stale table");
        commit_all(&dir, "stale desired state");

        let report = compare_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
        );

        assert!(report.success);
        assert!(report
            .repo_different
            .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    }

    #[test]
    fn compare_classifies_repo_only_objects() {
        let dir = create_temp_dir("compare-repo-only");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.local_only.sql"),
            "-- local only\n",
        )
        .expect("write local only table");
        commit_all(&dir, "local only desired state");

        let report =
            compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

        assert!(report.success);
        assert!(report
            .repo_only
            .contains(&"database/objects/tables/dbstate_slice2.local_only.sql".to_string()));
    }

    #[test]
    fn compare_classifies_database_only_objects() {
        let dir = create_temp_dir("compare-database-only");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report =
            compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

        assert!(report.success);
        assert!(report
            .database_only
            .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
        assert!(report
            .database_only
            .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    }

    #[test]
    fn compare_json_includes_expected_fields_and_no_secrets() {
        let dir = create_temp_dir("compare-json");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let mut report =
            compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);
        report.warnings.push(redact_postgres_url(&placeholder_url(
            "user",
            "sensitive-marker",
        )));
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"databaseType\"",
            "\"compareScope\"",
            "\"selectedSchemas\"",
            "\"selectedTables\"",
            "\"inSync\"",
            "\"repoDifferent\"",
            "\"repoOnly\"",
            "\"databaseOnly\"",
            "\"skipped\"",
            "\"warnings\"",
            "\"errors\"",
            "\"deferredObjectTypes\"",
            "\"workingTreeStatus\"",
            "\"isDirty\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }
        assert!(!json.contains("postgres://"));
        assert!(!json.contains("sensitive-marker"));
    }

    #[test]
    fn plan_missing_scope_selection_returns_clear_error() {
        let dir = create_temp_dir("plan-missing-selection");
        let parsed =
            ParsedArgs::parse(&["plan".to_string(), "postgres".to_string()]).expect("parse");

        let report = plan_postgres_command(&dir, parsed);

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Selection is required")));
    }

    #[test]
    fn plan_invalid_object_ref_returns_clear_error() {
        let error = PlanSelection::from_options(vec!["bad-ref".to_string()], Vec::new())
            .expect_err("invalid object ref");

        assert!(error.contains("Invalid object reference"));
    }

    #[test]
    fn plan_requires_git_repository_and_dbstate_structure() {
        let non_git = create_temp_dir("plan-non-git");
        let report = plan_postgres_with_inventory(
            &non_git,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("not inside a Git repository"));

        let no_structure = create_temp_dir("plan-no-structure");
        init_git_repo(&no_structure);
        let report = plan_postgres_with_inventory(
            &no_structure,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("Run dbstate init first"));
    }

    #[test]
    fn plan_is_read_only_and_can_run_with_dirty_working_tree() {
        let dir = create_temp_dir("plan-dirty-readonly");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        );

        assert!(report.success);
        assert!(report.is_dirty);
        assert!(!dir
            .join("database/objects/schemas/dbstate_slice2.sql")
            .exists());
        assert!(report
            .plan_items
            .iter()
            .any(|item| item.plan_intent == "reviewDatabaseOnly"));
    }

    #[test]
    fn plan_builds_item_from_repo_different_object() {
        let dir = create_temp_dir("plan-repo-different");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/dbstate_slice2.sql"),
            render_schema_sql("dbstate_slice2"),
        )
        .expect("write schema");
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            "-- stale table\n",
        )
        .expect("write stale table");
        commit_all(&dir, "stale table");

        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        );

        assert!(report.success);
        assert!(report.plan_items.iter().any(|item| {
            item.object_ref == "table:dbstate_slice2.sample_accounts"
                && item.compare_classification == "repoDifferent"
                && item.plan_intent == "updateDatabaseLater"
        }));
    }

    #[test]
    fn plan_builds_item_from_repo_only_object() {
        let dir = create_temp_dir("plan-repo-only");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/local_only.sql"),
            render_schema_sql("local_only"),
        )
        .expect("write repo-only schema");
        commit_all(&dir, "repo-only schema");

        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        );

        assert!(report.success);
        assert!(report.plan_items.iter().any(|item| {
            item.object_ref == "schema:local_only"
                && item.compare_classification == "repoOnly"
                && item.plan_intent == "createInDatabaseLater"
        }));
    }

    #[test]
    fn plan_builds_review_item_from_database_only_object() {
        let dir = create_temp_dir("plan-database-only");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        );

        assert!(report.success);
        assert!(report.plan_items.iter().any(|item| {
            item.object_ref == "schema:dbstate_slice2"
                && item.compare_classification == "databaseOnly"
                && item.plan_intent == "reviewDatabaseOnly"
        }));
    }

    #[test]
    fn plan_include_limits_selected_plan_items() {
        let dir = create_temp_dir("plan-include");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/dbstate_slice2.sql"),
            "-- stale schema\n",
        )
        .expect("write stale schema");
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            "-- stale table\n",
        )
        .expect("write stale table");
        commit_all(&dir, "stale files");

        let selection = PlanSelection::from_options(
            vec!["table:dbstate_slice2.sample_accounts".to_string()],
            Vec::new(),
        )
        .expect("plan selection");
        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &selection,
        );

        assert!(report.success);
        assert_eq!(report.plan_items.len(), 1);
        assert_eq!(
            report.plan_items[0].object_ref,
            "table:dbstate_slice2.sample_accounts"
        );
    }

    #[test]
    fn plan_exclude_excludes_selected_objects() {
        let dir = create_temp_dir("plan-exclude");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/dbstate_slice2.sql"),
            "-- stale schema\n",
        )
        .expect("write stale schema");
        commit_all(&dir, "stale schema");

        let selection =
            PlanSelection::from_options(Vec::new(), vec!["schema:dbstate_slice2".to_string()])
                .expect("plan selection");
        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &selection,
        );

        assert!(report.success);
        assert!(!report
            .plan_items
            .iter()
            .any(|item| item.object_ref == "schema:dbstate_slice2"));
        assert!(report
            .excluded_objects
            .contains(&"schema:dbstate_slice2".to_string()));
    }

    #[test]
    fn plan_blocks_table_when_required_schema_file_is_missing() {
        let dir = create_temp_dir("plan-missing-schema");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            "-- stale table\n",
        )
        .expect("write stale table");
        commit_all(&dir, "table without schema");

        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
            &PlanSelection::include_all(),
        );

        assert!(report.success);
        assert!(report.blocked_items.iter().any(|item| {
            item.object_ref == "table:dbstate_slice2.sample_accounts"
                && item.plan_intent == "blocked"
        }));
        assert!(report.dependency_warnings.iter().any(|warning| {
            warning.warning_type == "missingDependency" && warning.severity == "blocked"
        }));
    }

    #[test]
    fn plan_warns_when_schema_is_excluded_for_selected_table() {
        let dir = create_temp_dir("plan-excluded-schema");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/dbstate_slice2.sql"),
            "-- stale schema\n",
        )
        .expect("write stale schema");
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            "-- stale table\n",
        )
        .expect("write stale table");
        commit_all(&dir, "stale files");

        let selection = PlanSelection::from_options(
            vec!["table:dbstate_slice2.sample_accounts".to_string()],
            vec!["schema:dbstate_slice2".to_string()],
        )
        .expect("plan selection");
        let report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &selection,
        );

        assert!(report.success);
        assert!(report
            .blocked_items
            .iter()
            .any(|item| { item.object_ref == "table:dbstate_slice2.sample_accounts" }));
        assert!(report.dependency_warnings.iter().any(|warning| {
            warning.warning_type == "dependentObjectImpacted"
                && warning.object_ref == "table:dbstate_slice2.sample_accounts"
        }));
    }

    #[test]
    fn plan_json_includes_expected_fields_and_no_secrets() {
        let dir = create_temp_dir("plan-json");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let mut report = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        );
        report.warnings.push(redact_postgres_url(&placeholder_url(
            "user",
            "sensitive-marker",
        )));
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"databaseType\"",
            "\"planScope\"",
            "\"selectedSchemas\"",
            "\"selectedTables\"",
            "\"includedObjects\"",
            "\"excludedObjects\"",
            "\"planItems\"",
            "\"blockedItems\"",
            "\"dependencyWarnings\"",
            "\"compareSummary\"",
            "\"warnings\"",
            "\"errors\"",
            "\"deferredObjectTypes\"",
            "\"workingTreeStatus\"",
            "\"isDirty\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }
        assert!(!json.contains("postgres://"));
        assert!(!json.contains("sensitive-marker"));
    }

    #[test]
    fn release_missing_name_returns_clear_error() {
        let dir = create_temp_dir("release-missing-name");
        let parsed = ParsedArgs::parse(&[
            "release".to_string(),
            "postgres".to_string(),
            "--all".to_string(),
        ])
        .expect("parse release");

        let report = release_postgres_command(&dir, parsed);

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Release name is required")));
    }

    #[test]
    fn release_invalid_name_returns_clear_error() {
        let dir = create_temp_dir("release-invalid-name");
        let parsed = ParsedArgs::parse(&[
            "release".to_string(),
            "postgres".to_string(),
            "--all".to_string(),
            "--name".to_string(),
            "../bad".to_string(),
        ])
        .expect("parse release");

        let report = release_postgres_command(&dir, parsed);

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Release name must use only")));
    }

    #[test]
    fn release_missing_scope_selection_returns_clear_error() {
        let dir = create_temp_dir("release-missing-scope");
        let parsed = ParsedArgs::parse(&[
            "release".to_string(),
            "postgres".to_string(),
            "--name".to_string(),
            "slice7".to_string(),
        ])
        .expect("parse release");

        let report = release_postgres_command(&dir, parsed);

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Selection is required")));
    }

    #[test]
    fn release_requires_git_repository_and_dbstate_structure() {
        let non_git = create_temp_dir("release-non-git");
        let report = release_postgres_with_inventory(
            &non_git,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
            "slice7",
            false,
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("not inside a Git repository"));

        let no_structure = create_temp_dir("release-no-structure");
        init_git_repo(&no_structure);
        let report = release_postgres_with_inventory(
            &no_structure,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
            "slice7",
            false,
        );
        assert!(!report.success);
        assert!(report.errors[0].contains("Run dbstate init first"));
    }

    #[test]
    fn release_write_is_blocked_when_working_tree_is_dirty() {
        let dir = create_temp_dir("release-dirty");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

        let report = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
            "slice7",
            false,
        );

        assert!(!report.success);
        assert!(report.errors[0].contains("working tree has changes"));
        assert!(report.created_artifacts.is_empty());
    }

    #[test]
    fn release_dry_run_writes_no_files_and_plans_artifacts() {
        let dir = create_temp_dir("release-dry-run");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/local_only.sql"),
            render_schema_sql("local_only"),
        )
        .expect("write repo-only schema");
        commit_all(&dir, "repo-only schema");

        let report = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::from_options(vec!["schema:local_only".to_string()], Vec::new())
                .expect("plan selection"),
            "slice7",
            true,
        );

        assert!(report.success);
        assert!(report
            .planned_artifacts
            .contains(&"database/releases/0001_slice7.sql".to_string()));
        assert!(report.created_artifacts.is_empty());
        assert!(!dir.join("database/releases/0001_slice7.sql").exists());
    }

    #[test]
    fn release_generates_sql_summary_and_risk_json_under_releases() {
        let dir = create_temp_dir("release-write");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/local_only.sql"),
            render_schema_sql("local_only"),
        )
        .expect("write repo-only schema");
        fs::write(
            dir.join("database/objects/tables/local_only.accounts.sql"),
            render_table_sql("local_only", "accounts", &sample_inventory().columns),
        )
        .expect("write repo-only table");
        commit_all(&dir, "repo-only objects");

        let report = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::from_options(
                vec![
                    "schema:local_only".to_string(),
                    "table:local_only.accounts".to_string(),
                ],
                Vec::new(),
            )
            .expect("plan selection"),
            "Slice7_Test",
            false,
        );

        assert!(report.success, "{:?}", report.errors);
        assert_eq!(
            report.created_artifacts,
            vec![
                "database/releases/0001_slice7_test.sql".to_string(),
                "database/releases/0001_slice7_test.summary.md".to_string(),
                "database/releases/0001_slice7_test.risk.json".to_string(),
                "database/releases/0001_slice7_test.manifest.json".to_string(),
            ]
        );
        let sql =
            fs::read_to_string(dir.join("database/releases/0001_slice7_test.sql")).expect("sql");
        let summary = fs::read_to_string(dir.join("database/releases/0001_slice7_test.summary.md"))
            .expect("summary");
        let risk = fs::read_to_string(dir.join("database/releases/0001_slice7_test.risk.json"))
            .expect("risk");
        let manifest =
            fs::read_to_string(dir.join("database/releases/0001_slice7_test.manifest.json"))
                .expect("manifest");

        assert!(sql.contains("-- DbState Release Artifact"));
        assert!(sql.contains("-- Safety: Review-only. DbState does not execute this SQL."));
        assert!(sql.contains("BEGIN REVIEW SECTION: Summary"));
        assert!(sql.contains("BEGIN REVIEW SECTION: Creates"));
        assert!(sql.contains("BEGIN REVIEW SECTION: Review Required"));
        assert!(sql.contains("BEGIN REVIEW SECTION: Blocked Items"));
        assert!(sql.contains("BEGIN REVIEW SECTION: Deferred Object Types"));
        assert!(sql.contains("CREATE SCHEMA IF NOT EXISTS \"local_only\";"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"local_only\".\"accounts\""));
        for forbidden in [
            "DROP TABLE",
            "DROP SCHEMA",
            "ALTER TABLE DROP",
            "TRUNCATE",
            "DELETE FROM",
            "INSERT INTO",
        ] {
            assert!(!sql.contains(forbidden), "forbidden SQL found: {forbidden}");
        }
        assert!(summary.contains("## Reviewer Checklist"));
        assert!(summary.contains("Review all REVIEW REQUIRED comments"));
        assert!(summary.contains("Confirm no destructive SQL is present"));
        assert!(summary.contains("DbState did not execute SQL"));
        assert!(summary.contains("## Object Counts By Status"));
        assert!(summary.contains("## Risk Summary"));
        assert!(risk.contains("\"command\":\"release postgres\""));
        assert!(risk.contains("\"releaseSequence\":\"0001\""));
        assert!(risk.contains("\"generatedArtifacts\""));
        assert!(risk.contains("\"repositoryContext\""));
        assert!(risk.contains("\"counts\""));
        assert!(risk.contains("\"objectTypeCounts\""));
        assert!(risk.contains("\"riskLevel\":\"low\""));
        assert!(risk.contains("\"riskReasons\""));
        assert!(risk.contains("\"destructiveSqlGenerated\":false"));
        assert!(risk.contains("\"directApplyAvailable\":false"));
        assert!(risk.contains("\"generatedSqlExecutionSupported\":false"));
        assert!(risk.contains("\"databaseMutationPerformed\":false"));
        assert!(risk.contains("\"gitMutationPerformed\":false"));
        assert!(risk.contains("\"credentialPersistencePerformed\":false"));
        assert!(manifest.contains("\"releaseName\":\"Slice7_Test\""));
        assert!(manifest.contains("\"artifactType\":\"sql\""));
        assert!(manifest.contains("\"artifactType\":\"manifest\""));
        assert!(!sql.contains("postgres://"));
        assert!(!summary.contains("postgres://"));
        assert!(!risk.contains("postgres://"));
        assert!(!manifest.contains("postgres://"));
    }

    #[test]
    fn release_does_not_overwrite_existing_artifacts() {
        let dir = create_temp_dir("release-sequence");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/releases/0001_slice7.sql"),
            "-- existing\n",
        )
        .expect("write existing sql");
        fs::write(
            dir.join("database/releases/0001_slice7.summary.md"),
            "existing\n",
        )
        .expect("write existing summary");
        fs::write(dir.join("database/releases/0001_slice7.risk.json"), "{}\n")
            .expect("write existing risk");
        fs::write(
            dir.join("database/objects/schemas/local_only.sql"),
            render_schema_sql("local_only"),
        )
        .expect("write repo-only schema");
        commit_all(&dir, "existing artifacts and repo-only schema");

        let report = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::from_options(vec!["schema:local_only".to_string()], Vec::new())
                .expect("plan selection"),
            "slice7",
            true,
        );

        assert!(report.success);
        assert!(report
            .planned_artifacts
            .contains(&"database/releases/0002_slice7.sql".to_string()));
    }

    #[test]
    fn release_blocks_when_selected_plan_items_are_blocked() {
        let dir = create_temp_dir("release-blocked");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            "-- local drift\n",
        )
        .expect("write table without schema file");
        commit_all(&dir, "table without schema");

        let report = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
            &PlanSelection::include_all(),
            "slice7",
            false,
        );

        assert!(!report.success);
        assert_eq!(report.risk_level, "blocked");
        assert!(report.created_artifacts.is_empty());
        assert!(report
            .blocked_items
            .iter()
            .any(|item| item.object_ref == "table:dbstate_slice2.sample_accounts"));
    }

    #[test]
    fn changed_table_release_uses_review_only_comment_not_alter() {
        let dir = create_temp_dir("release-changed-table");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::write(
            dir.join("database/objects/schemas/dbstate_slice2.sql"),
            render_schema_sql("dbstate_slice2"),
        )
        .expect("write schema");
        fs::write(
            dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
            "-- local drift\n",
        )
        .expect("write changed table");
        commit_all(&dir, "changed table");

        let report = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
            &PlanSelection::include_all(),
            "slice7",
            false,
        );

        assert!(report.success, "{:?}", report.errors);
        let sql =
            fs::read_to_string(dir.join("database/releases/0001_slice7.sql")).expect("read sql");
        assert!(sql.contains("REVIEW REQUIRED: object differs"));
        assert!(!sql.contains("\nALTER TABLE "));
    }

    #[test]
    fn release_json_includes_expected_fields_and_no_secrets() {
        let mut report = empty_release_report(true);
        report.release_name = "slice7".to_string();
        report.release_scope = "all".to_string();
        report
            .planned_artifacts
            .push("database/releases/0001_slice7.sql".to_string());
        report.warnings.push(redact_postgres_url(&placeholder_url(
            "user",
            "sensitive-marker",
        )));
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"databaseType\"",
            "\"releaseName\"",
            "\"releaseScope\"",
            "\"dryRun\"",
            "\"selectedSchemas\"",
            "\"selectedTables\"",
            "\"includedObjects\"",
            "\"excludedObjects\"",
            "\"planItems\"",
            "\"blockedItems\"",
            "\"dependencyWarnings\"",
            "\"plannedArtifacts\"",
            "\"createdArtifacts\"",
            "\"riskLevel\"",
            "\"warnings\"",
            "\"errors\"",
            "\"deferredObjectTypes\"",
            "\"workingTreeStatus\"",
            "\"isDirty\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }
        assert!(!json.contains("postgres://"));
        assert!(!json.contains("sensitive-marker"));
    }

    #[test]
    fn data_compare_missing_scope_selection_returns_clear_error() {
        let dir = create_temp_dir("data-compare-missing-scope");
        let parsed = ParsedArgs::parse(&["data-compare".to_string(), "postgres".to_string()])
            .expect("parse");

        let report = data_compare_postgres_command(&dir, parsed);

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Selection is required")));
    }

    #[test]
    fn data_compare_requires_git_repository_and_dbstate_structure() {
        let non_git = create_temp_dir("data-compare-non-git");
        let report = data_compare_postgres_with_connection(
            &non_git,
            &placeholder_url("user", "secret"),
            &ReferenceDataSelection::All,
        )
        .expect("report");
        assert!(!report.success);
        assert!(report.errors[0].contains("not inside a Git repository"));

        let no_structure = create_temp_dir("data-compare-no-structure");
        init_git_repo(&no_structure);
        let report = data_compare_postgres_with_connection(
            &no_structure,
            &placeholder_url("user", "secret"),
            &ReferenceDataSelection::All,
        )
        .expect("report");
        assert!(!report.success);
        assert!(report.errors[0].contains("Run dbstate init first"));
    }

    #[test]
    fn data_compare_empty_registry_is_valid_and_read_only_when_dirty() {
        let dir = create_temp_dir("data-compare-empty-registry");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

        let report = data_compare_postgres_with_connection(
            &dir,
            &placeholder_url("user", "secret"),
            &ReferenceDataSelection::All,
        )
        .expect("report");

        assert!(report.success, "{:?}", report.errors);
        assert_eq!(report.selected_tables.len(), 0);
        assert!(report.is_dirty);
        assert!(!dir.join("database/releases/0001_data.sql").exists());
    }

    #[test]
    fn data_compare_invalid_postgres_url_returns_clear_error() {
        let dir = create_temp_dir("data-compare-invalid-url");
        init_git_repo(&dir);
        create_complete_structure(&dir);

        let report = data_compare_postgres_with_connection(
            &dir,
            "not-a-postgres-url",
            &ReferenceDataSelection::All,
        )
        .expect("report");

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("Invalid PostgreSQL connection URL")));
        assert!(!report
            .errors
            .iter()
            .any(|error| error.contains("not-a-postgres-url")));
    }

    #[test]
    fn reference_registry_parser_accepts_valid_configured_table() {
        let registry =
            parse_reference_data_registry(valid_reference_registry_yaml()).expect("registry");

        assert_eq!(registry.version, 1);
        assert_eq!(registry.tables.len(), 1);
        assert_eq!(registry.tables[0].name, "dbstate_ref.payment_methods");
        assert_eq!(
            registry.tables[0].file,
            "tables/dbstate_ref.payment_methods.yml"
        );
        assert_eq!(registry.tables[0].key_columns, vec!["code".to_string()]);
        assert_eq!(
            registry.tables[0].ignore_columns,
            vec!["updated_at".to_string()]
        );
        assert_eq!(
            registry.tables[0].masked_columns,
            vec!["secret_note".to_string()]
        );
    }

    #[test]
    fn reference_registry_parser_rejects_missing_key() {
        let yaml = "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
";
        let error = parse_reference_data_registry(yaml).expect_err("missing key");

        assert!(error.contains("missing required field 'key'"));
    }

    #[test]
    fn reference_registry_parser_rejects_masked_or_ignored_key() {
        let masked = "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
    key: [code]
    maskedColumns: [code]
";
        let ignored = "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
    key: [code]
    ignoreColumns: [code]
";

        assert!(parse_reference_data_registry(masked)
            .expect_err("masked key")
            .contains("cannot be masked"));
        assert!(parse_reference_data_registry(ignored)
            .expect_err("ignored key")
            .contains("cannot be ignored"));
    }

    #[test]
    fn reference_table_file_parser_accepts_valid_rows() {
        let (config, state) = reference_config_and_state();

        assert_eq!(state.table_name, config.name);
        assert_eq!(state.rows.len(), 3);
        assert_eq!(
            reference_row_key(&state.rows[0], &config.key_columns).expect("row key"),
            "code=CASH"
        );
    }

    #[test]
    fn reference_table_file_parser_rejects_duplicate_row_keys() {
        let (config, _) = reference_config_and_state();
        let yaml = "table: dbstate_ref.payment_methods
key: [code]
rows:
  - code: CASH
    name: Cash
  - code: CASH
    name: Cash Duplicate
";
        let error = parse_reference_data_table_state(yaml, &config).expect_err("duplicate key");

        assert!(error.contains("duplicate row key"));
    }

    #[test]
    fn reference_table_file_parser_rejects_missing_key_column() {
        let (config, _) = reference_config_and_state();
        let yaml = "table: dbstate_ref.payment_methods
key: [code]
rows:
  - name: Missing Code
";
        let error = parse_reference_data_table_state(yaml, &config).expect_err("missing key");

        assert!(error.contains("missing key column"));
    }

    #[test]
    fn data_compare_selected_unconfigured_table_returns_clear_error() {
        let dir = create_temp_dir("data-compare-unconfigured");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let report = data_compare_postgres_with_connection(
            &dir,
            &placeholder_url("user", "secret"),
            &ReferenceDataSelection::Table("dbstate_ref.missing".to_string()),
        )
        .expect("report");

        assert!(!report.success);
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("not configured")));
    }

    #[test]
    fn reference_data_compare_classifies_rows_and_ignores_columns() {
        let (config, state) = reference_config_and_state();
        let database_rows = vec![
            reference_row(&[
                ("code", Some("CASH")),
                ("name", Some("Cash")),
                ("is_active", Some("true")),
                ("sort_order", Some("10")),
                ("updated_at", Some("different ignored value")),
            ]),
            reference_row(&[
                ("code", Some("QRPH")),
                ("name", Some("QRPh Live")),
                ("is_active", Some("true")),
                ("sort_order", Some("20")),
                ("updated_at", Some("different ignored value")),
            ]),
            reference_row(&[
                ("code", Some("CARD")),
                ("name", Some("Card")),
                ("is_active", Some("true")),
                ("sort_order", Some("30")),
            ]),
        ];
        let result = compare_reference_data_table(
            &config,
            &state,
            &database_rows,
            &reference_database_columns(),
        );

        assert_eq!(result.row_counts.in_sync, 1);
        assert_eq!(result.row_counts.repo_different, 1);
        assert_eq!(result.row_counts.repo_only, 1);
        assert_eq!(result.row_counts.database_only, 1);
        let different = result
            .row_results
            .iter()
            .find(|row| row.row_key == "code=QRPH")
            .expect("different row");
        assert_eq!(different.classification, "repoDifferent");
        assert_eq!(different.changed_columns, vec!["name".to_string()]);
        assert!(!different
            .changed_columns
            .contains(&"updated_at".to_string()));
    }

    #[test]
    fn reference_data_compare_masks_columns_without_exposing_values() {
        let (config, state) = reference_config_and_state();
        let database_rows = vec![reference_row(&[
            ("code", Some("CASH")),
            ("name", Some("Cash")),
            ("is_active", Some("true")),
            ("sort_order", Some("10")),
            ("secret_note", Some("database-secret-value")),
        ])];
        let result = compare_reference_data_table(
            &config,
            &state,
            &database_rows,
            &reference_database_columns(),
        );
        let mut report = empty_reference_data_compare_report();
        append_reference_table_result(&mut report, result);
        report.success = true;
        let json = report.to_json();
        let text = report.to_text();

        assert!(json.contains("secret_note"));
        assert!(!json.contains("repo-secret-cash"));
        assert!(!json.contains("database-secret-value"));
        assert!(!text.contains("repo-secret-cash"));
        assert!(!text.contains("database-secret-value"));
    }

    #[test]
    fn data_compare_json_includes_expected_fields_and_no_secrets() {
        let mut report = empty_reference_data_compare_report();
        report.compare_scope = "all".to_string();
        report.warnings.push(redact_postgres_url(&placeholder_url(
            "user",
            "sensitive-marker",
        )));
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"repositoryPath\"",
            "\"gitRoot\"",
            "\"isGitRepository\"",
            "\"branch\"",
            "\"workingTreeStatus\"",
            "\"isDirty\"",
            "\"databaseType\"",
            "\"compareScope\"",
            "\"dataCompareScope\"",
            "\"selectedTables\"",
            "\"tableResults\"",
            "\"counts\"",
            "\"inSync\"",
            "\"repoDifferent\"",
            "\"repoOnly\"",
            "\"databaseOnly\"",
            "\"skipped\"",
            "\"warnings\"",
            "\"errors\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }
        assert!(!json.contains("postgres://"));
        assert!(!json.contains("sensitive-marker"));
    }

    fn assert_common_json_contract(json: &str) {
        for field in ["\"command\"", "\"success\"", "\"warnings\"", "\"errors\""] {
            assert!(json.contains(field), "missing common JSON field {field}");
        }
    }

    fn assert_repository_json_contract(json: &str) {
        for field in [
            "\"repositoryPath\"",
            "\"gitRoot\"",
            "\"isGitRepository\"",
            "\"branch\"",
            "\"workingTreeStatus\"",
            "\"isDirty\"",
        ] {
            assert!(
                json.contains(field),
                "missing repository JSON field {field}"
            );
        }
    }

    fn assert_postgres_json_contract(json: &str) {
        assert!(json.contains("\"databaseType\":\"postgresql\""));
    }

    #[test]
    fn slice9_common_json_contract_fields_are_present() {
        let dir = create_temp_dir("slice9-json-contract");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let project_json = status_report(&dir, CommandKind::RepoStatus).to_json();
        assert_common_json_contract(&project_json);
        assert_repository_json_contract(&project_json);

        let inspection_json = empty_inspection_report(CommandKind::InspectPostgres).to_json();
        assert_common_json_contract(&inspection_json);
        assert_postgres_json_contract(&inspection_json);

        let export_json =
            export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true)
                .to_json();
        assert_common_json_contract(&export_json);
        assert_repository_json_contract(&export_json);
        assert_postgres_json_contract(&export_json);

        let sync_json =
            sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true)
                .to_json();
        assert_common_json_contract(&sync_json);
        assert_repository_json_contract(&sync_json);
        assert_postgres_json_contract(&sync_json);

        let compare_json =
            compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All)
                .to_json();
        assert_common_json_contract(&compare_json);
        assert_repository_json_contract(&compare_json);
        assert_postgres_json_contract(&compare_json);

        let plan_json = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
        )
        .to_json();
        assert_common_json_contract(&plan_json);
        assert_repository_json_contract(&plan_json);
        assert_postgres_json_contract(&plan_json);

        let release_json = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            &PlanSelection::include_all(),
            "slice9_contract",
            true,
        )
        .to_json();
        assert_common_json_contract(&release_json);
        assert_repository_json_contract(&release_json);
        assert_postgres_json_contract(&release_json);

        let data_compare_json = data_compare_postgres_with_connection(
            &dir,
            &placeholder_url("user", "contract-secret"),
            &ReferenceDataSelection::All,
        )
        .expect("data compare report")
        .to_json();
        assert_common_json_contract(&data_compare_json);
        assert_repository_json_contract(&data_compare_json);
        assert_postgres_json_contract(&data_compare_json);
        assert!(data_compare_json.contains("\"dataCompareScope\""));
        assert!(!data_compare_json.contains("contract-secret"));
        assert!(!data_compare_json.contains("postgres://"));
    }

    #[test]
    fn slice9_usage_includes_all_current_commands_and_common_options() {
        let usage = usage();

        for expected in [
            "dbstate repo status",
            "dbstate init",
            "dbstate inspect postgres",
            "dbstate export postgres",
            "dbstate sync postgres",
            "dbstate compare postgres",
            "dbstate plan postgres",
            "dbstate release postgres",
            "dbstate data-compare postgres",
            "--format json",
            "--json",
            "--url <postgres-url>",
            "--dry-run",
            "--all",
            "--schema",
            "--table",
            "--include",
            "--exclude",
            "--name",
            "dbstate serve",
            "--host <host>",
            "--port <port>",
        ] {
            assert!(usage.contains(expected), "usage missing {expected}");
        }
    }

    #[test]
    fn slice11_service_routes_include_only_approved_endpoints() {
        let routes = service_route_definitions();

        for expected in [
            ("GET", "/"),
            ("GET", "/ui"),
            ("GET", "/ui/"),
            ("GET", "/ui/app.css"),
            ("GET", "/ui/app.js"),
            ("GET", "/health"),
            ("GET", "/api/v1/health"),
            ("GET", "/api/v1/workspace/roots"),
            ("POST", "/api/v1/workspace/list-directories"),
            ("POST", "/api/v1/workspace/validate"),
            ("GET", "/api/v1/connections/profiles"),
            ("POST", "/api/v1/connections/profiles"),
            ("PUT", "/api/v1/connections/profiles/{name}"),
            ("DELETE", "/api/v1/connections/profiles/{name}"),
            ("POST", "/api/v1/connections/test"),
            ("POST", "/api/v1/repo/status"),
            ("POST", "/api/v1/init/plan"),
            ("POST", "/api/v1/init/write"),
            ("POST", "/api/v1/postgres/inspect"),
            ("POST", "/api/v1/postgres/compare"),
            ("POST", "/api/v1/postgres/plan"),
            ("POST", "/api/v1/postgres/data-compare"),
            ("POST", "/api/v1/postgres/object-ddl"),
            ("POST", "/api/v1/postgres/repository-sync/preview"),
            ("POST", "/api/v1/postgres/repository-sync/write"),
            ("POST", "/api/v1/postgres/release/preview"),
            ("POST", "/api/v1/postgres/release/write"),
        ] {
            assert!(routes.contains(&expected), "missing route {expected:?}");
        }

        for (_, path) in routes {
            assert!(!path.contains("export"));
            assert!(!path.contains("/api/v1/postgres/sync"));
            assert!(!path.contains("apply"));
        }
    }

    #[test]
    fn slice12_ui_routes_serve_static_assets() {
        let dir = create_temp_dir("slice12-ui-routes");

        let root = service_response("GET", "/", "", &dir);
        assert_eq!(root.status_code, 200);
        assert!(root.content_type.contains("text/html"));
        assert!(root.body.contains("DbState PostgreSQL v0.1"));

        let ui = service_response("GET", "/ui", "", &dir);
        assert_eq!(ui.status_code, 200);
        assert!(ui.content_type.contains("text/html"));

        let ui_slash = service_response("GET", "/ui/", "", &dir);
        assert_eq!(ui_slash.status_code, 200);
        assert!(ui_slash.content_type.contains("text/html"));

        let css = service_response("GET", "/ui/app.css", "", &dir);
        assert_eq!(css.status_code, 200);
        assert!(css.content_type.contains("text/css"));
        assert!(css.body.contains(".workflow-panel"));

        let js = service_response("GET", "/ui/app.js", "", &dir);
        assert_eq!(js.status_code, 200);
        assert!(js.content_type.contains("application/javascript"));
        assert!(js.body.contains("/api/v1/health"));

        let versioned_js = service_response("GET", "/ui/app.js?v=slice22-beta-ui", "", &dir);
        assert_eq!(versioned_js.status_code, 200);
        assert!(versioned_js.body.contains("directionForWorkflowMode"));
    }

    #[test]
    fn slice12_ui_html_contains_safety_messages_and_no_external_assets() {
        let html = ui_html();

        assert!(html.contains("Local only"));
        assert!(html.contains("No SQL execution"));
        assert!(html.contains("direct database apply"));
        assert!(html.contains("Controlled local repository/release file writes"));
        assert!(html.contains("Schema compare workflow shell"));
        assert!(html.contains("Init Plan"));
        assert!(html.contains("Initialize DbState Project"));
        assert!(html.contains("INITIALIZE DBSTATE PROJECT"));
        assert!(html.contains("Workspace"));
        assert!(html.contains("Source &amp; Target"));
        assert!(html.contains("Compare Options"));
        assert!(html.contains("Results"));
        assert!(html.contains("Object Diff"));
        assert!(html.contains("Warnings"));
        assert!(html.contains("Release Plan"));
        assert!(html.contains("Reports / Raw JSON"));
        assert!(html.contains("About / Safety"));
        assert!(html.contains("workspace-path"));
        assert!(html.contains("workspace-browse"));
        assert!(html.contains("directory-picker"));
        assert!(html.contains("directory-picker-path"));
        assert!(html.contains("directory-list"));
        assert!(html.contains("Session-only"));
        assert!(html.contains("does not clone or fetch repositories"));
        assert!(html.contains("results-grid"));
        assert!(html.contains("selected-json"));
        assert!(html.contains("warnings-list"));
        assert!(html.contains("object-type-filter"));
        assert!(html.contains("status-filter"));
        assert!(html.contains("status-legend"));
        assert!(html.contains("results-source"));
        assert!(html.contains("results-target"));
        assert!(!html.contains("<th>Source</th>"));
        assert!(!html.contains("<th>Target</th>"));
        assert!(html.contains("release-name"));
        assert!(html.contains("Reviewer Checklist"));
        assert!(html.contains("dbstate release postgres --all --name"));
        assert!(html.contains("Raw JSON"));
        assert!(html.contains("/ui/app.css?v=slice22-beta-ui"));
        assert!(html.contains("/ui/app.js?v=slice22-beta-ui"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("cdn"));
        assert!(!html.contains("unpkg"));
        assert!(!html.contains("jsdelivr"));
    }

    #[test]
    fn slice12_ui_javascript_calls_only_approved_endpoints() {
        let js = ui_js();
        let approved = [
            "/api/v1/health",
            "/api/v1/connections/profiles",
            "/api/v1/connections/test",
            "/api/v1/repo/status",
            "/api/v1/init/plan",
            "/api/v1/init/write",
            "/api/v1/postgres/inspect",
            "/api/v1/postgres/compare",
            "/api/v1/postgres/plan",
            "/api/v1/postgres/data-compare",
            "/api/v1/postgres/object-ddl",
            "/api/v1/postgres/repository-sync/preview",
            "/api/v1/postgres/repository-sync/write",
            "/api/v1/postgres/release/preview",
            "/api/v1/postgres/release/write",
            "/api/v1/workspace/roots",
            "/api/v1/workspace/list-directories",
            "/api/v1/workspace/validate",
        ];

        for endpoint in approved {
            assert!(
                js.contains(endpoint),
                "missing approved endpoint {endpoint}"
            );
        }

        for forbidden in [
            "/api/v1/postgres/export",
            "/api/v1/release",
            "/api/v1/postgres/apply",
            "localStorage",
            "sessionStorage",
            "showDirectoryPicker",
            "console.log",
            "execute generated SQL",
            "execute SQL",
            "sync to database",
            "directApply",
            "mutateDatabase",
            "clone",
            "git fetch",
            "git pull",
            "git push",
            "git add",
            "git commit",
        ] {
            assert!(
                !js.contains(forbidden),
                "UI JavaScript contains forbidden pattern {forbidden}"
            );
        }
    }

    #[test]
    fn ui_contains_stable_playwright_demo_selectors() {
        let html = ui_html();
        let js = ui_js();
        let expected_html_selectors = [
            "data-testid=\"tab-workspace\"",
            "data-testid=\"workspace-browse\"",
            "data-testid=\"workspace-health\"",
            "data-testid=\"workspace-check\"",
            "data-testid=\"workspace-repo-status\"",
            "data-testid=\"workspace-init-plan\"",
            "data-testid=\"workspace-initialize-project\"",
            "data-testid=\"tab-source-target\"",
            "data-testid=\"workflow-mode\"",
            "data-testid=\"connection-mode\"",
            "data-testid=\"source-connection-input\"",
            "data-testid=\"target-connection-input\"",
            "data-testid=\"source-target-run\"",
            "data-testid=\"tab-compare-options\"",
            "data-testid=\"preview-repository-sync\"",
            "data-testid=\"write-repository-changes\"",
            "data-testid=\"tab-results\"",
            "data-testid=\"results-object-type-filter\"",
            "data-testid=\"results-status-filter\"",
            "data-testid=\"results-table\"",
            "data-testid=\"results-visible-row-count\"",
            "data-testid=\"results-included-count\"",
            "data-testid=\"results-status-legend\"",
            "data-testid=\"tab-object-diff\"",
            "data-testid=\"object-diff-left\"",
            "data-testid=\"object-diff-right\"",
            "data-testid=\"object-diff-repository\"",
            "data-testid=\"object-diff-database\"",
            "data-testid=\"selected-json-item\"",
            "data-testid=\"tab-warnings\"",
            "data-testid=\"warnings-panel\"",
            "data-testid=\"warnings-list\"",
            "data-testid=\"tab-release-plan\"",
            "data-testid=\"release-plan-dry-run\"",
            "data-testid=\"release-context\"",
            "data-testid=\"risk-summary\"",
            "data-testid=\"object-summary\"",
            "data-testid=\"release-candidates\"",
            "data-testid=\"generated-artifacts\"",
            "data-testid=\"generate-release-artifact\"",
            "data-testid=\"tab-reports-raw-json\"",
            "data-testid=\"reports-panel\"",
            "data-testid=\"raw-json-panel\"",
            "data-testid=\"raw-json-copy\"",
        ];

        for selector in expected_html_selectors {
            assert!(
                html.contains(selector),
                "missing stable demo selector {selector}"
            );
        }

        for expected in [
            "tr.setAttribute(\"data-testid\", \"results-row-actor\")",
            "include.setAttribute(\"data-testid\", \"results-include-checkbox\")",
            "td.setAttribute(\"data-testid\", \"results-row-first-selectable\")",
            "function setObjectDiffDdlTestIds(direction)",
            "sourceDetail.setAttribute(\"data-testid\", direction.sourceDdlSide === \"database\" ? \"object-diff-database\" : \"object-diff-repository\")",
            "targetDetail.setAttribute(\"data-testid\", direction.targetDdlSide === \"repository\" ? \"object-diff-repository\" : \"object-diff-database\")",
        ] {
            assert!(js.contains(expected), "missing JS selector path {expected}");
        }

        for not_added in [
            "data-testid=\"selected-json-copy\"",
            "data-testid=\"warnings-empty-state\"",
            "data-testid=\"raw-json-download\"",
        ] {
            assert!(
                !html.contains(not_added),
                "selector should not exist without a corresponding UI element: {not_added}"
            );
        }
    }

    #[test]
    fn slice22_object_diff_markers_are_visual_only_and_beta_colored() {
        let css = ui_css();
        let js = ui_js();

        assert!(css.contains(".diff-marker"));
        assert!(css.contains("user-select: none"));
        assert!(css.contains(".diff-line-text"));
        assert!(css.contains(".diff-line-same {\n  color: #ffffff;"));
        assert!(css.contains(".diff-line-source-only {\n  color: #7ee2a8;"));
        assert!(css.contains(".diff-line-target-only {\n  color: #ff8f85;"));
        assert!(!css.contains("text-decoration: line-through"));

        assert!(js.contains("appendVisualDiffLine(sourceLine"));
        assert!(js.contains("appendVisualDiffLine(targetLine"));
        assert!(js.contains("markerElement.className = \"diff-marker\""));
        assert!(js.contains("textElement.className = \"diff-line-text\""));
        assert!(js.contains("markerElement.setAttribute(\"aria-hidden\", \"true\")"));
        assert!(!js.contains("\"+ \" + row.source"));
        assert!(!js.contains("\"- \" + row.target"));
        assert!(!js.contains("line-through"));
    }

    #[test]
    fn slice22_current_private_beta_text_avoids_older_slice_labels() {
        let combined = format!("{}\n{}\n{}", ui_html(), ui_css(), ui_js());

        for forbidden in [
            "Slice 16A",
            "Slice 16.A",
            "Slice 17",
            "automatic ALTER is not generated in Slice",
            "Full dependency ordering is not implemented in Slice",
        ] {
            assert!(
                !combined.contains(forbidden),
                "current UI assets expose old implementation label {forbidden}"
            );
        }
    }

    #[test]
    fn slice13a_ui_html_contains_schema_compare_workflow_structure() {
        let html = ui_html();

        for expected in [
            "Workspace",
            "Source &amp; Target",
            "Compare Options",
            "Results",
            "Object Diff",
            "Warnings",
            "Release Plan",
            "Reports / Raw JSON",
            "About / Safety",
            "results-grid",
            "Object type",
            "Planned operation",
            "Source",
            "Target",
            "Source type",
            "Target type",
            "Diff detail not available yet",
            "reviewable files",
        ] {
            assert!(
                html.contains(expected),
                "missing UI workflow text {expected}"
            );
        }

        for forbidden in [
            "React",
            "Vue",
            "Svelte",
            "Angular",
            "Vite",
            "node_modules",
            "unpkg",
            "jsdelivr",
            "Deploy to database",
            "Execute SQL",
            "Sync to Database",
        ] {
            assert!(
                !html.contains(forbidden),
                "UI HTML contains forbidden pattern {forbidden}"
            );
        }
    }

    #[test]
    fn slice13b_results_grid_usability_contract_is_present() {
        let html = ui_html();
        let css = ui_css();
        let js = ui_js();

        assert!(html.contains("<label for=\"object-type-filter\">Object type"));
        assert!(html.contains("<option value=\"all\">All</option>"));
        assert!(html.contains("<option value=\"schema\">Schema</option>"));
        assert!(html.contains("<option value=\"table\">Table</option>"));
        assert!(!html.contains("<option value=\"column\">Column</option>"));
        assert!(!html.contains("<option value=\"referenceData\">Reference data</option>"));
        assert!(html.contains("<select id=\"compare-schema\">"));
        assert!(html.contains("<select id=\"compare-table\">"));
        assert!(html.contains("<select id=\"data-table\" disabled>"));
        assert!(html.contains("Run Inspect first to populate schema and table lists."));
        assert!(html.contains("reference-data compare are out-of-scope"));
        assert!(!html.contains("placeholder=\"dbstate_slice2\""));
        assert!(!html.contains("placeholder=\"schema.table\""));
        assert!(!html.contains("table:dbstate_slice2.sample_accounts"));
        assert!(!html.contains("schema:public"));
        assert!(html.contains("status-legend"));
        assert!(html.contains("inSync"));
        assert!(html.contains("repoDifferent"));
        assert!(html.contains("databaseOnly"));
        assert!(html.contains("inspected"));
        assert!(!html.contains("<th>Source</th>"));
        assert!(!html.contains("<th>Target</th>"));

        for class_name in [
            ".status-repodifferent",
            ".status-insync",
            ".status-repoonly",
            ".status-databaseonly",
            ".status-skipped",
            ".status-error",
            ".status-inspected",
        ] {
            assert!(
                css.contains(class_name),
                "missing status class {class_name}"
            );
        }

        for expected in [
            "identityFromPath",
            "database/objects/schemas/",
            "database/objects/tables/",
            "database/objects/extensions/",
            "database/objects/enums/",
            "database/objects/sequences/",
            "database/objects/indexes/",
            "database/objects/views/",
            "objectType: \"schema\"",
            "objectType: \"table\"",
            "objectType: \"extension\"",
            "objectType: \"enum\"",
            "objectType: \"sequence\"",
            "objectType: \"index\"",
            "objectType: \"view\"",
            "referenceDataRow",
            "rowMatchesFilter",
            "rowMatchesStatusFilter",
            "compareResultRows",
            "status-filter",
            "object-type-filter",
            "data.schemas",
            "data.tables",
            "data.columns",
            "data.extensions",
            "data.enums",
            "data.sequences",
            "data.indexes",
            "data.views",
            "updateCompareOptionLists",
            "updateTableOptions",
            "updateReferenceDataOptions",
            "updateObjectTypeFilterOptions",
            "columnsForSelectedTable",
            "projectStructureGuidance",
            "This workspace is a Git repository but not yet an initialized DbState project",
            "return { scope: \"table\", table: table };",
            "return { scope: \"schema\", schema: schema };",
            "return { scope: \"all\" };",
            "statusBadge(row.status)",
        ] {
            assert!(js.contains(expected), "missing JS mapping text {expected}");
        }

        assert!(!js.contains("objectRef: \"column:"));
        assert!(js.contains("label === \"Inspect\""));
        assert!(js.contains("label === \"Reference-data compare\""));
        assert!(js.contains("appendOption(select, \"referenceData\", \"Reference data\")"));
        assert!(!js.contains("service:response"));
        assert!(!js.contains("objectType: \"service\""));
    }

    #[test]
    fn slice15_service_routes_and_write_confirmation_are_present() {
        let routes = service_route_definitions();
        assert!(routes.contains(&("POST", "/api/v1/postgres/object-ddl")));
        assert!(routes.contains(&("POST", "/api/v1/postgres/repository-sync/preview")));
        assert!(routes.contains(&("POST", "/api/v1/postgres/repository-sync/write")));
        assert!(routes.contains(&("POST", "/api/v1/postgres/release/preview")));
        assert!(routes.contains(&("POST", "/api/v1/postgres/release/write")));
        assert!(routes.contains(&("GET", "/api/v1/workspace/roots")));
        assert!(routes.contains(&("POST", "/api/v1/workspace/list-directories")));
        assert!(routes.contains(&("POST", "/api/v1/workspace/validate")));

        let dir = create_temp_dir("slice15-confirmation");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let missing_confirmation = service_response(
            "POST",
            "/api/v1/postgres/repository-sync/write",
            r#"{ "scope": "all" }"#,
            &dir,
        );

        assert_eq!(missing_confirmation.status_code, 400);
        assert_common_json_contract(&missing_confirmation.body);
        assert!(missing_confirmation.body.contains("WRITE REPOSITORY FILES"));
        assert!(!missing_confirmation.body.contains("postgres://"));
    }

    #[test]
    fn slice15_object_ddl_reads_only_database_objects_under_workspace() {
        let dir = create_temp_dir("slice15-object-ddl");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        let schema_path = dir
            .join("database")
            .join("objects")
            .join("schemas")
            .join("core.sql");
        fs::write(&schema_path, "CREATE SCHEMA \"core\";\n").expect("write schema ddl");
        commit_all(&dir, "complete structure");

        let body = r#"{ "scope": "all", "objectType": "schema", "schema": "core", "objectName": "core", "relativePath": "database/objects/schemas/core.sql" }"#;
        let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

        assert_eq!(response.status_code, 200, "{}", response.body);
        assert_common_json_contract(&response.body);
        assert!(response.body.contains("\"repositoryDdl\":\"CREATE SCHEMA"));
        assert!(response.body.contains("Database DDL is unavailable"));
        assert!(!response.body.contains("postgres://"));

        for unsafe_path in [
            "database/releases/0001.sql",
            "database/objects/../secrets.sql",
            "/database/objects/schemas/core.sql",
            "database/objects/schemas/core.txt",
        ] {
            let body = format!(
                r#"{{ "objectType": "schema", "schema": "core", "objectName": "core", "relativePath": "{}" }}"#,
                unsafe_path
            );
            let rejected = service_response("POST", "/api/v1/postgres/object-ddl", &body, &dir);
            assert_eq!(rejected.status_code, 400, "{}", rejected.body);
        }
    }

    #[test]
    fn slice16a_object_ddl_returns_object_only_full_context_and_related_objects() {
        let dir = create_temp_dir("slice16a-object-ddl-context");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        let table_path = dir
            .join("database")
            .join("objects")
            .join("tables")
            .join("core.accounts.sql");
        fs::write(
            &table_path,
            "CREATE TABLE \"core\".\"accounts\" (\n    \"account_id\" integer NOT NULL\n);\n",
        )
        .expect("write table ddl");
        let index_path = dir
            .join("database")
            .join("objects")
            .join("indexes")
            .join("core.accounts.accounts_code_idx.sql");
        fs::write(
            &index_path,
            "CREATE INDEX \"accounts_code_idx\" ON \"core\".\"accounts\" (\"account_code\");\n",
        )
        .expect("write index ddl");
        commit_all(&dir, "complete structure with table and index");

        let body = r#"{ "scope": "all", "objectType": "table", "schema": "core", "objectName": "accounts", "relativePath": "database/objects/tables/core.accounts.sql" }"#;
        let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

        assert_eq!(response.status_code, 200, "{}", response.body);
        assert_common_json_contract(&response.body);
        assert!(response.body.contains("\"objectOnly\""));
        assert!(response.body.contains("\"fullContext\""));
        assert!(response.body.contains("\"relatedObjects\""));
        assert!(response.body.contains("CREATE TABLE"));
        assert!(response.body.contains("accounts_code_idx"));
        assert!(response
            .body
            .contains("database/objects/indexes/core.accounts.accounts_code_idx.sql"));
        assert!(response.body.contains("\"group\":\"Indexes\""));
        assert!(response.body.contains("\"group\":\"Constraints\""));
        assert!(response.body.contains("\"group\":\"Comments\""));
        assert!(response
            .body
            .contains("Object Only DDL is the normalized durable object"));
        assert!(!response.body.contains("postgres://"));
    }

    #[test]
    fn slice15_repository_sync_preview_uses_dry_run_and_does_not_write_without_connection() {
        let dir = create_temp_dir("slice15-preview");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        let objects = dir.join("database").join("objects");
        let before = fs::read_dir(&objects).expect("read objects").count();

        let response = service_response(
            "POST",
            "/api/v1/postgres/repository-sync/preview",
            r#"{ "scope": "all" }"#,
            &dir,
        );

        assert_eq!(response.status_code, 400);
        assert_common_json_contract(&response.body);
        assert!(response.body.contains("Missing PostgreSQL connection URL"));
        assert_eq!(
            fs::read_dir(&objects).expect("read objects").count(),
            before
        );
    }

    #[test]
    fn slice15_repository_sync_write_rejects_dirty_tree_before_files_are_written() {
        let dir = create_temp_dir("slice15-dirty-write");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");
        fs::write(dir.join("dirty.txt"), "dirty").expect("dirty tree");

        let report =
            sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

        assert!(!report.success);
        assert!(report.created_files.is_empty());
        assert!(report.updated_files.is_empty());
        assert!(report
            .errors
            .iter()
            .any(|error| error.contains("working tree has changes")));
    }

    #[test]
    fn slice15_workspace_directory_picker_endpoints_are_safe_and_directory_only() {
        let dir = create_temp_dir("slice15-directory-picker");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        fs::create_dir_all(dir.join("child-a")).expect("create child-a");
        fs::create_dir_all(dir.join("child-b")).expect("create child-b");
        fs::write(dir.join("not-listed.txt"), "not a directory").expect("write file");

        let roots = service_response("GET", "/api/v1/workspace/roots", "", &dir);
        assert_eq!(roots.status_code, 200, "{}", roots.body);
        assert_common_json_contract(&roots.body);
        assert!(roots.body.contains("workspace roots"));
        assert!(roots.body.contains("Service working directory"));

        let list_body = format!(r#"{{ "path": "{}" }}"#, escape_json(&display_path(&dir)));
        let list = service_response(
            "POST",
            "/api/v1/workspace/list-directories",
            &list_body,
            &dir,
        );
        assert_eq!(list.status_code, 200, "{}", list.body);
        assert_common_json_contract(&list.body);
        assert!(list.body.contains("child-a"));
        assert!(list.body.contains("child-b"));
        assert!(!list.body.contains("not-listed.txt"));

        for body in [
            r#"{ "path": "https://example.com/repo.git" }"#.to_string(),
            r#"{ "path": "bad\u0000path" }"#.to_string(),
            format!(
                r#"{{ "path": "{}" }}"#,
                escape_json(&display_path(&dir.join("missing")))
            ),
        ] {
            let rejected =
                service_response("POST", "/api/v1/workspace/list-directories", &body, &dir);
            assert_eq!(rejected.status_code, 400, "{}", rejected.body);
            assert_common_json_contract(&rejected.body);
        }

        let validate_body = format!(
            r#"{{ "repositoryPath": "{}" }}"#,
            escape_json(&display_path(&dir))
        );
        let validate = service_response("POST", "/api/v1/workspace/validate", &validate_body, &dir);
        assert_eq!(validate.status_code, 200, "{}", validate.body);
        assert_common_json_contract(&validate.body);
        assert!(validate.body.contains("repositoryPath"));
        assert!(validate.body.contains("dbstateProjectStatus"));
    }

    #[test]
    fn slice15_ui_database_to_repository_workflow_contract_is_present() {
        let html = ui_html();
        let js = ui_js();
        let css = ui_css();

        for expected in [
            "Database to Repository",
            "Preview Repository Sync",
            "WRITE REPOSITORY FILES",
            "WRITE REPOSITORY FILES",
            "Copy JSON",
            "Source type",
            "Target type",
            "Source DDL",
            "Target DDL",
            "Full Context DDL",
            "Object Only DDL",
            "Related Objects",
            "Raw Details",
            "data-diff-mode=\"fullContext\"",
            "source-related-objects",
            "target-related-objects",
            "DDL unavailable",
            "repository-sync-controls",
            "source-content",
            "target-content",
            "repository-context",
            "postgres-connection-context",
            "catalog-context",
            "repository-branch",
            "repository-tree",
            "repository-project",
            "repository-dirty",
            "workspace-browse",
            "directory-picker",
            "Select Workspace Folder",
            "Select this folder",
        ] {
            assert!(html.contains(expected), "missing UI text {expected}");
        }
        assert!(html.find("Workflow Mode").unwrap() < html.find("id=\"source-panel\"").unwrap());
        assert!(html.find("Workflow Mode").unwrap() < html.find("id=\"target-panel\"").unwrap());
        assert!(!html.contains("Repository Side"));
        assert!(!html.contains("Database Side"));

        for expected in [
            "/api/v1/postgres/object-ddl",
            "/api/v1/postgres/repository-sync/preview",
            "/api/v1/postgres/repository-sync/write",
            "/api/v1/workspace/roots",
            "/api/v1/workspace/list-directories",
            "/api/v1/workspace/validate",
            "repositorySyncBody",
            "confirmRepositoryWrite",
            "confirmationText",
            "data-standard-operation-action",
            "standardOperationAllowed",
            "Compare is not available in Database to Repository mode. Use Preview Repository Sync.",
            "copyRedactedJson",
            "jsonViewer.textContent",
            "Database to Repository Preview",
            "Database to Repository Write",
            "directionForResultRow",
            "directionForWorkflowMode",
            "rowMatchesWorkflowMode",
            "rowMatchesCurrentWorkflow",
            "isDatabaseToRepositoryRow",
            "workflowLayout",
            "placeSourceTargetContext",
            "sourceContext: \"connection\"",
            "targetContext: \"repository\"",
            "sourceContext: \"repository\"",
            "targetContext: \"connection\"",
            "targetContext: \"catalog\"",
            "Repository configured reference data",
            "Read-only catalog view",
            "DbState captures supported PostgreSQL database state into the selected repository after preview and explicit confirmation.",
            "DbState compares repository desired state to PostgreSQL through read-only service operations.",
            "DbState reads PostgreSQL catalog state through read-only inspection.",
            "DbState compares configured repository reference data to PostgreSQL through read-only service operations.",
            "normalizeDdlForComparison",
            "updateDdlComparisonStatus",
            "ddl-similar",
            "ddl-different",
            "ddl-unavailable",
            "Similar",
            "Different",
            "DDL not available yet for this object.",
            "objectDdlRequest",
            "loadSelectedObjectDdl",
            "objectDdlSection",
            "renderLoadedObjectDiff",
            "relatedObjectsForSide",
            "renderRelatedObjectList",
            "state.objectDiffMode = \"fullContext\"",
            "const label = mode === \"objectOnly\" ? \"Object Only DDL\" : \"Full Context DDL\";",
            "openDirectoryPicker",
            "loadDirectoryRoots",
            "listDirectories",
            "selectDirectoryAsWorkspace",
            "approvedEndpoints.workspaceValidate",
            "friendlyPath",
            "row.producingWorkflowMode = workflowModeForOperation(label);",
            "row.workflowMode = row.producingWorkflowMode;",
            "const direction = directionForResultRow(row);",
            "objectDiffDisplayPayload(row, direction)",
            "clearOperationResults",
            "Workflow mode changed. Run the selected operation again.",
            "operation: row.resultOperation || row.operation || \"review\"",
            "objectDiffDirectionRegressionFixture",
            "staleCompareResultInDatabaseToRepositoryFixture",
            "window.dbstateUiTestHooks",
            "sourceType: \"Repository\"",
            "targetType: \"Database\"",
            "sourceType: displayPayload.sourceType",
            "targetType: displayPayload.targetType",
            "renderedText",
            "\"sourceType \" + displayPayload.sourceType",
            "\"targetType \" + displayPayload.targetType",
            "\"Source type: \" + direction.sourceType",
            "\"Target type: \" + direction.targetType",
            "\"Source DDL \" + sourceDdl",
            "\"Target DDL \" + targetDdl",
        ] {
            assert!(js.contains(expected), "missing JS text {expected}");
        }
        assert!(js.contains(
            "const visibleRows = state.rows.filter(function (row) {\n      return rowMatchesFilter(row) && rowMatchesCurrentWorkflow(row);"
        ));
        assert!(js.contains(
            "Selected result belongs to a different workflow. Run the current workflow again."
        ));
        let direction_index = js.rfind("if (isDatabaseToRepositoryMode(mode))").unwrap();
        let direction_block = &js[direction_index..direction_index + 360];
        assert!(direction_block.contains("sourceType: \"Database\""));
        assert!(direction_block.contains("targetType: \"Repository\""));
        assert!(direction_block.contains("sourceDdlSide: \"database\""));
        assert!(direction_block.contains("targetDdlSide: \"repository\""));
        assert!(js.contains(
            "const ddlBySide = { repository: repositoryDdl, database: databaseDdl, \"\": \"\" };"
        ));
        assert!(js.contains("const sourceDdl = ddlBySide[direction.sourceDdlSide] || \"\";"));
        assert!(js.contains("const targetDdl = ddlBySide[direction.targetDdlSide] || \"\";"));
        assert!(js.contains("byId(\"selected-json\").textContent = redactedJson(objectDiffDisplayPayload(row, direction));"));
        assert!(!js.contains("row.sourceType"));
        assert!(!js.contains("row.targetType"));
        let fixture_index = js
            .find("function objectDiffDirectionRegressionFixture()")
            .unwrap();
        let fixture_end = js[fixture_index..]
            .find("if (typeof window !== \"undefined\")")
            .map(|offset| fixture_index + offset)
            .unwrap();
        let fixture_block = &js[fixture_index..fixture_end];
        assert!(fixture_block.contains("producingWorkflowMode: \"databaseToRepository\""));
        assert!(fixture_block.contains("resultOperation: \"Database to Repository Preview\""));
        assert!(fixture_block.contains("sourceType: \"Repository\""));
        assert!(fixture_block.contains("targetType: \"Database\""));
        assert!(fixture_block.contains("sourceType: displayPayload.sourceType"));
        assert!(fixture_block.contains("targetType: displayPayload.targetType"));
        assert!(fixture_block.contains("const sourceDdl = ddlBySide[direction.sourceDdlSide]"));
        assert!(fixture_block.contains("const targetDdl = ddlBySide[direction.targetDdlSide]"));
        assert!(fixture_block.contains("sourceDdl: sourceDdl"));
        assert!(fixture_block.contains("targetDdl: targetDdl"));
        assert!(fixture_block.contains("\"sourceType \" + displayPayload.sourceType"));
        assert!(fixture_block.contains("\"targetType \" + displayPayload.targetType"));
        assert!(fixture_block.contains("\"Source type: \" + direction.sourceType"));
        assert!(fixture_block.contains("\"Target type: \" + direction.targetType"));
        assert!(fixture_block.contains("\"Source DDL \" + sourceDdl"));
        assert!(fixture_block.contains("\"Target DDL \" + targetDdl"));
        assert!(!fixture_block.contains("\"sourceType \" + staleRow.sourceType"));
        assert!(!fixture_block.contains("\"targetType \" + staleRow.targetType"));
        let stale_fixture_index = js
            .find("function staleCompareResultInDatabaseToRepositoryFixture()")
            .unwrap();
        let stale_fixture_end = js[stale_fixture_index..]
            .find("if (typeof window !== \"undefined\")")
            .map(|offset| stale_fixture_index + offset)
            .unwrap();
        let stale_fixture_block = &js[stale_fixture_index..stale_fixture_end];
        assert!(stale_fixture_block.contains("producingWorkflowMode: \"compare\""));
        assert!(stale_fixture_block.contains("operation: \"Compare\""));
        assert!(stale_fixture_block.contains("sourceType: \"Repository\""));
        assert!(stale_fixture_block.contains("targetType: \"Database\""));
        assert!(stale_fixture_block.contains("const selectedMode = \"databaseToRepository\""));
        assert!(stale_fixture_block
            .contains("const canRender = rowMatchesWorkflowMode(staleCompareRow, selectedMode);"));
        assert!(stale_fixture_block.contains("canRender: canRender"));
        assert!(stale_fixture_block.contains("visibleRows: canRender ? 1 : 0"));
        assert!(stale_fixture_block.contains(
            "Selected result belongs to a different workflow. Run the current workflow again."
        ));
        for expected in [
            "addedFiles",
            "changedFiles",
            "unchangedFiles",
            "plannedCreates",
            "plannedUpdates",
            "createdFiles",
            "updatedFiles",
        ] {
            assert!(js.contains(expected), "missing sync mapping {expected}");
        }
        assert!(css.contains(".status-plannedcreate"));
        assert!(css.contains(".status-updated"));

        for forbidden in [
            "Deploy to database",
            "Execute SQL",
            "Execute generated SQL",
            "Sync to Database",
            "Apply to Target",
            "Push to Target Database",
            "localStorage",
            "sessionStorage",
            "showDirectoryPicker",
        ] {
            assert!(
                !html.contains(forbidden) && !js.contains(forbidden),
                "UI contains forbidden pattern {forbidden}"
            );
        }
    }

    #[test]
    fn slice15_workspace_path_display_normalizes_windows_verbatim_prefixes() {
        let verbatim = Path::new(r"\\?\D:\DbState\ExitPassDb");
        assert_eq!(display_path(verbatim), "D:/DbState/ExitPassDb");
        assert_eq!(
            normalize_local_path_input("///?/D:/DbState/ExitPassDb"),
            "D:/DbState/ExitPassDb"
        );

        let js = ui_js();
        assert!(js.contains("text.replace(/^\\/{2,3}\\?\\//, \"\")"));
        assert!(js.contains("byId(\"workspace-path\").value = gitRoot"));
        assert!(!js.contains("localStorage"));
        assert!(!js.contains("sessionStorage"));
        assert!(!js.contains("showDirectoryPicker"));
    }

    #[test]
    fn slice14_missing_profile_file_returns_empty_list_and_saves_outside_repo() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let repo = create_temp_dir("slice14-repo");
        let config = create_temp_dir("slice14-config");
        env::set_var("DBSTATE_CONFIG_DIR", &config);

        let store = load_connection_profiles().expect("load profiles");
        assert!(store.profiles.is_empty());
        assert!(!config.join(PROFILE_FILE_NAME).exists());

        let store = ConnectionProfileStore {
            profiles: vec![valid_connection_profile("exitpass-local")],
        };
        save_connection_profiles(&store).expect("save profiles");

        let profile_file = config.join(PROFILE_FILE_NAME);
        assert!(profile_file.exists());
        assert!(!profile_file.starts_with(&repo));
        let content = fs::read_to_string(profile_file).expect("read profile file");
        assert!(!content.contains("password"));
        assert!(!content.contains("token"));
        assert!(!content.contains("postgres://"));
        env::remove_var("DBSTATE_CONFIG_DIR");
    }

    #[test]
    fn slice14_profile_validation_rejects_duplicate_invalid_and_secret_values() {
        let duplicate = ConnectionProfileStore {
            profiles: vec![
                valid_connection_profile("local"),
                valid_connection_profile("LOCAL"),
            ],
        };
        assert!(validate_unique_profile_names(&duplicate.profiles).is_err());

        let mut invalid_port = valid_connection_profile("bad-port");
        invalid_port.port = 0;
        assert!(validate_connection_profile(&invalid_port).is_err());

        let mut url_host = valid_connection_profile("url-host");
        url_host.host = "postgres://localhost".to_string();
        assert!(validate_connection_profile(&url_host).is_err());

        let secret_json = r#"{ "name": "bad", "host": "localhost", "port": 5432, "database": "db", "username": "postgres", "sslMode": "disable", "password": "secret" }"#;
        let value: Value = serde_yaml::from_str(secret_json).expect("parse json");
        let error = parse_connection_profile(&value).expect_err("secret field rejected");
        assert!(!error.contains("password"));
        assert!(!error.contains("secret\""));
        assert!(error.contains("<redacted-field>"));
    }

    #[test]
    fn slice14_invalid_profile_json_returns_clear_error() {
        let path = temp_path("slice14-invalid-json").with_extension("json");
        fs::write(&path, "{ invalid json").expect("write invalid json");

        let error = load_connection_profiles_from_path(&path).expect_err("invalid json");

        assert!(error.contains("invalid JSON"));
    }

    #[test]
    fn slice14_connection_resolution_precedence_and_profile_session_password() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let config = create_temp_dir("slice14-resolution-config");
        env::set_var("DBSTATE_CONFIG_DIR", &config);
        env::set_var(
            "DBSTATE_POSTGRES_URL",
            "postgres://env_user:env_secret@example.invalid/env_db",
        );
        save_connection_profiles(&ConnectionProfileStore {
            profiles: vec![valid_connection_profile("local")],
        })
        .expect("save profiles");

        let request_url: Value = serde_yaml::from_str(
            r#"{ "postgresUrl": "postgres://request_user:request_secret@example.invalid/request_db", "connection": { "profileName": "local", "password": "profile_secret" } }"#,
        )
        .expect("parse request");
        let resolved = resolve_service_postgres_connection(&request_url)
            .expect("resolve")
            .expect("connection");
        assert_eq!(resolved.source, "requestUrl");
        assert!(resolved.url.contains("request_user"));

        let profile_request: Value = serde_yaml::from_str(
            r#"{ "connection": { "profileName": "local", "password": "profile secret" } }"#,
        )
        .expect("parse request");
        let resolved = resolve_service_postgres_connection(&profile_request)
            .expect("resolve")
            .expect("connection");
        assert_eq!(resolved.source, "profile");
        assert!(resolved.url.starts_with("postgres://postgres:"));
        assert!(resolved.url.contains("profile%20secret"));
        assert!(!resolved.url.contains("profile secret"));

        let env_request: Value = serde_yaml::from_str("{}").expect("parse request");
        let resolved = resolve_service_postgres_connection(&env_request)
            .expect("resolve")
            .expect("connection");
        assert_eq!(resolved.source, "environment");
        assert!(resolved.url.contains("env_user"));

        env::remove_var("DBSTATE_CONFIG_DIR");
        env::remove_var("DBSTATE_POSTGRES_URL");
    }

    #[test]
    fn slice14_service_profile_endpoints_return_json_and_never_store_secrets() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let config = create_temp_dir("slice14-service-config");
        let cwd = create_temp_dir("slice14-service-cwd");
        env::set_var("DBSTATE_CONFIG_DIR", &config);

        let empty = service_response("GET", "/api/v1/connections/profiles", "", &cwd);
        assert_eq!(empty.status_code, 200);
        assert!(empty.body.contains("\"profiles\":[]"));

        let create = service_response(
            "POST",
            "/api/v1/connections/profiles",
            r#"{ "name": "exitpass-local", "host": "localhost", "port": 5433, "database": "exitpass_v12_dev", "username": "postgres", "sslMode": "disable", "description": "local dev" }"#,
            &cwd,
        );
        assert_eq!(create.status_code, 200, "{}", create.body);
        assert!(create.body.contains("exitpass-local"));
        assert!(!create.body.contains("password"));
        assert!(!create.body.contains("postgres://"));

        let update = service_response(
            "PUT",
            "/api/v1/connections/profiles/exitpass-local",
            r#"{ "name": "exitpass-local", "host": "localhost", "port": 5434, "database": "exitpass_v12_dev", "username": "postgres", "sslMode": "prefer" }"#,
            &cwd,
        );
        assert_eq!(update.status_code, 200, "{}", update.body);
        assert!(update.body.contains("\"port\":5434"));

        let stored = fs::read_to_string(config.join(PROFILE_FILE_NAME)).expect("read profiles");
        assert!(!stored.contains("password"));
        assert!(!stored.contains("postgres://"));

        let delete = service_response(
            "DELETE",
            "/api/v1/connections/profiles/exitpass-local",
            "",
            &cwd,
        );
        assert_eq!(delete.status_code, 200, "{}", delete.body);
        assert!(delete.body.contains("\"profiles\":[]"));

        env::remove_var("DBSTATE_CONFIG_DIR");
    }

    #[test]
    fn slice14_connection_test_redacts_missing_and_bad_connection_values() {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let cwd = create_temp_dir("slice14-test-connection");
        env::remove_var("DBSTATE_POSTGRES_URL");

        let missing = service_response("POST", "/api/v1/connections/test", "{}", &cwd);
        assert_eq!(missing.status_code, 400);
        assert!(missing.body.contains("Missing PostgreSQL connection"));

        let bad = service_response(
            "POST",
            "/api/v1/connections/test",
            r#"{ "postgresUrl": "not-a-postgres-url-with-secret" }"#,
            &cwd,
        );
        assert_eq!(bad.status_code, 503);
        assert!(bad.body.contains("Invalid PostgreSQL connection URL"));
        assert!(!bad.body.contains("not-a-postgres-url-with-secret"));
        assert!(!bad.body.contains("password"));
    }

    #[test]
    fn slice14_ui_connection_profile_contract_is_present() {
        let html = ui_html();
        let js = ui_js();

        for expected in [
            "connection-mode",
            "Use session URL",
            "Use saved profile",
            "Use service environment variable",
            "profile-select",
            "profile-password",
            "profile-host",
            "profile-database",
            "profile-username",
            "profile-sslmode",
            "Save Profile",
            "Delete Profile",
            "Test Connection",
            "Passwords, tokens, and full URLs are never saved.",
        ] {
            assert!(
                html.contains(expected),
                "missing UI profile text {expected}"
            );
        }
        for forbidden in ["Save Password", "Remember Password", "save password"] {
            assert!(!html.contains(forbidden));
        }
        for expected in [
            "/api/v1/connections/profiles",
            "/api/v1/connections/test",
            "attachConnection",
            "profileName",
            "connection.password",
            "selectedConnectionMode",
            "connectionString",
            "connectionUrl",
            "postgresUrl",
        ] {
            assert!(js.contains(expected), "missing UI JS text {expected}");
        }
        assert!(!js.contains("localStorage"));
        assert!(!js.contains("sessionStorage"));
        assert!(!js.contains("deploy to database"));
        assert!(!js.contains("execute SQL"));
        assert!(!js.contains("sync to database"));
    }

    fn valid_connection_profile(name: &str) -> ConnectionProfile {
        ConnectionProfile {
            name: name.to_string(),
            host: "localhost".to_string(),
            port: 5432,
            database: "exitpass_v12_dev".to_string(),
            username: "postgres".to_string(),
            ssl_mode: "disable".to_string(),
            description: Some("Local development database".to_string()),
            default_schema: None,
        }
    }

    #[test]
    fn slice13_valid_repository_path_is_accepted_by_service() {
        let fallback = create_temp_dir("slice13-fallback");
        let selected = create_temp_dir("slice13-selected");
        init_git_repo(&selected);
        create_complete_structure(&selected);
        commit_all(&selected, "complete structure");
        let body = format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&selected));

        let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);

        assert_eq!(response.status_code, 200);
        assert_common_json_contract(&response.body);
        assert_repository_json_contract(&response.body);
        assert!(response.body.contains(&escape_json(&display_path(
            &selected.canonicalize().unwrap()
        ))));
        assert!(response
            .body
            .contains("\"dbstateProjectStatus\":\"completeDbStateStructure\""));
    }

    #[test]
    fn slice13_missing_or_empty_repository_path_falls_back_to_service_cwd() {
        let fallback = create_temp_dir("slice13-service-cwd");
        init_git_repo(&fallback);
        create_complete_structure(&fallback);
        commit_all(&fallback, "complete structure");

        let omitted = service_response("POST", "/api/v1/repo/status", "{}", &fallback);
        let empty = service_response(
            "POST",
            "/api/v1/repo/status",
            r#"{ "repositoryPath": "   " }"#,
            &fallback,
        );

        assert_eq!(omitted.status_code, 200);
        assert_eq!(empty.status_code, 200);
        assert!(omitted
            .body
            .contains(&escape_json(&display_path(&fallback))));
        assert!(empty.body.contains(&escape_json(&display_path(&fallback))));
    }

    #[test]
    fn slice13_invalid_repository_paths_are_rejected() {
        let fallback = create_temp_dir("slice13-invalid-paths");
        init_git_repo(&fallback);
        let file_path = fallback.join("not-a-directory.txt");
        fs::write(&file_path, "not a directory").expect("write test file");
        let nonexistent = fallback.join("missing");

        for (body, expected) in [
            (
                format!(
                    r#"{{ "repositoryPath": "{}" }}"#,
                    display_path(&nonexistent)
                ),
                "does not exist",
            ),
            (
                format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&file_path)),
                "must point to a directory",
            ),
            (
                r#"{ "repositoryPath": "https://example.com/repo.git" }"#.to_string(),
                "local filesystem path",
            ),
            (
                r#"{ "repositoryPath": "git@example.com:repo.git" }"#.to_string(),
                "local filesystem path",
            ),
        ] {
            let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);
            assert_eq!(response.status_code, 400);
            assert!(
                response.body.contains(expected),
                "response was {}",
                response.body
            );
        }
    }

    #[test]
    fn slice13_non_git_directory_is_rejected_as_workspace() {
        let fallback = create_temp_dir("slice13-fallback-git");
        init_git_repo(&fallback);
        let non_git = create_temp_dir("slice13-non-git");
        let body = format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&non_git));

        let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);

        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("Git working tree"));
    }

    #[test]
    fn slice13_git_without_dbstate_structure_returns_useful_status() {
        let fallback = create_temp_dir("slice13-fallback-status");
        init_git_repo(&fallback);
        let selected = create_temp_dir("slice13-git-no-structure");
        init_git_repo(&selected);
        let body = format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&selected));

        let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);

        assert_eq!(response.status_code, 200);
        assert!(response
            .body
            .contains("\"dbstateProjectStatus\":\"gitRepositoryWithoutDbStateStructure\""));
        assert!(response.body.contains("\"missingPaths\""));
    }

    #[test]
    fn slice13_project_operations_use_selected_repository_path() {
        let fallback = create_temp_dir("slice13-fallback-operation");
        init_git_repo(&fallback);
        create_complete_structure(&fallback);
        commit_all(&fallback, "fallback structure");

        let selected = create_temp_dir("slice13-selected-operation");
        init_git_repo(&selected);
        create_complete_structure(&selected);
        commit_all(&selected, "selected structure");
        let body = format!(
            r#"{{ "repositoryPath": "{}", "dryRun": true }}"#,
            display_path(&selected)
        );

        let response = service_response("POST", "/api/v1/init/plan", &body, &fallback);

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains(&escape_json(&display_path(
            &selected.canonicalize().unwrap()
        ))));
        assert!(!response
            .body
            .contains(&escape_json(&display_path(&fallback))));
    }

    #[test]
    fn init_write_service_endpoint_requires_confirmation_and_git_repository() {
        let dir = create_temp_dir("init-write-confirmation");
        init_git_repo(&dir);

        let missing = service_response("POST", "/api/v1/init/write", "{}", &dir);
        assert_eq!(missing.status_code, 400);
        assert!(missing.body.contains("confirmationText"));
        assert!(!dir.join("database").exists());

        let wrong = service_response(
            "POST",
            "/api/v1/init/write",
            r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE" }"#,
            &dir,
        );
        assert_eq!(wrong.status_code, 400);
        assert!(!dir.join("database").exists());

        let non_git = create_temp_dir("init-write-non-git");
        let body = r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;
        let rejected = service_response("POST", "/api/v1/init/write", body, &non_git);
        assert_eq!(rejected.status_code, 400);
        assert!(rejected.body.contains("\"success\":false"));
        assert!(!non_git.join("database").exists());
    }

    #[test]
    fn init_write_service_endpoint_creates_only_project_structure_without_staging() {
        let dir = create_temp_dir("init-write-success");
        init_git_repo(&dir);
        let body = r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;

        let response = service_response("POST", "/api/v1/init/write", body, &dir);

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"success\":true"));
        assert!(response
            .body
            .contains("\"dbstateProjectStatus\":\"completeDbStateStructure\""));
        assert!(response.body.contains("\"plannedCreates\""));
        assert!(response.body.contains("\"createdPaths\""));
        for expected in EXPECTED_PATHS {
            assert!(
                dir.join(expected.relative).exists(),
                "missing {}",
                expected.relative
            );
        }
        assert!(!dir.join("database/releases/0001_unexpected.sql").exists());
        let staged = Command::new("git")
            .arg("diff")
            .arg("--cached")
            .arg("--name-only")
            .current_dir(&dir)
            .output()
            .expect("run git diff cached");
        assert!(staged.status.success(), "git diff cached failed");
        assert!(
            String::from_utf8_lossy(&staged.stdout).trim().is_empty(),
            "init write staged files"
        );
    }

    #[test]
    fn init_write_service_endpoint_does_not_overwrite_existing_registry() {
        let dir = create_temp_dir("init-write-no-overwrite");
        init_git_repo(&dir);
        let registry = dir.join("database/reference-data/dbstate.reference-data.yml");
        fs::create_dir_all(registry.parent().expect("registry parent"))
            .expect("create registry parent");
        fs::write(&registry, "version: 1\ntables:\n  - keep_me\n").expect("write custom registry");
        commit_all(&dir, "existing registry");
        let body = r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;

        let response = service_response("POST", "/api/v1/init/write", body, &dir);

        assert_eq!(response.status_code, 200);
        assert_eq!(
            fs::read_to_string(&registry).expect("read registry"),
            "version: 1\ntables:\n  - keep_me\n"
        );
    }

    #[test]
    fn slice13_ui_javascript_includes_repository_path_without_persistence() {
        let js = ui_js();

        assert!(js.contains("repositoryPath"));
        assert!(js.contains("workspace-path"));
        assert!(js.contains("attachWorkspacePath"));
        assert!(!js.contains("localStorage"));
        assert!(!js.contains("sessionStorage"));
        assert!(!js.contains("clone"));
        assert!(!js.contains("git fetch"));
        assert!(!js.contains("git pull"));
        assert!(!js.contains("git push"));
        assert!(!js.contains("git add"));
        assert!(!js.contains("git commit"));
    }

    #[test]
    fn slice12_service_api_routes_remain_json() {
        let dir = create_temp_dir("slice12-api-json");
        let health = service_response("GET", "/api/v1/health", "", &dir);
        assert_eq!(health.status_code, 200);
        assert!(health.content_type.contains("application/json"));
        assert_common_json_contract(&health.body);

        let missing = service_response("GET", "/api/v1/missing", "", &dir);
        assert_eq!(missing.status_code, 404);
        assert!(missing.content_type.contains("application/json"));
        assert_common_json_contract(&missing.body);
    }

    #[test]
    fn slice11_service_health_and_repo_status_return_json_contract() {
        let dir = create_temp_dir("slice11-service-status");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let health = service_response("GET", "/health", "", &dir);
        assert_eq!(health.status_code, 200);
        assert_common_json_contract(&health.body);
        assert!(health.body.contains("\"service\":\"dbstate\""));

        let status = service_response("POST", "/api/v1/repo/status", "{}", &dir);
        assert_eq!(status.status_code, 200);
        assert_common_json_contract(&status.body);
        assert_repository_json_contract(&status.body);
        assert!(status.body.contains("\"command\":\"repo status\""));
    }

    #[test]
    fn slice11_service_init_plan_is_dry_run_only() {
        let dir = create_temp_dir("slice11-init-plan");
        init_git_repo(&dir);

        let plan = service_response("POST", "/api/v1/init/plan", r#"{ "dryRun": true }"#, &dir);
        assert_eq!(plan.status_code, 200);
        assert!(plan.body.contains("\"command\":\"init\""));
        assert!(plan.body.contains("\"plannedCreates\""));
        assert!(!dir.join("database").exists());

        let write = service_response("POST", "/api/v1/init/plan", r#"{ "dryRun": false }"#, &dir);
        assert_eq!(write.status_code, 400);
        assert!(write.body.contains("init planning only"));
    }

    #[test]
    fn slice11_service_rejects_invalid_scope_and_write_requests() {
        let dir = create_temp_dir("slice11-invalid-requests");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let invalid_scope = service_response(
            "POST",
            "/api/v1/postgres/compare",
            r#"{ "scope": "everything" }"#,
            &dir,
        );
        assert_eq!(invalid_scope.status_code, 400);
        assert!(invalid_scope.body.contains("Invalid scope"));

        let write_request = service_response(
            "POST",
            "/api/v1/postgres/plan",
            r#"{ "scope": "all", "apply": true }"#,
            &dir,
        );
        assert_eq!(write_request.status_code, 400);
        assert!(write_request.body.contains("read-only or plan-only"));

        let remote_workspace = service_response(
            "POST",
            "/api/v1/repo/status",
            r#"{ "repositoryPath": "https://example.com/repo.git" }"#,
            &dir,
        );
        assert_eq!(remote_workspace.status_code, 400);
        assert!(remote_workspace.body.contains("local filesystem path"));
    }

    #[test]
    fn slice11_service_redacts_postgres_url_and_credentials() {
        let dir = create_temp_dir("slice11-redaction");
        let raw_url = placeholder_url("service-user", "service-secret-marker");
        let body = format!(r#"{{ "postgresUrl": "{raw_url}", "scope": "all" }}"#);

        let response = service_response("POST", "/api/v1/postgres/inspect", &body, &dir);

        assert_eq!(response.status_code, 503);
        assert_common_json_contract(&response.body);
        assert!(!response.body.contains(&raw_url));
        assert!(!response.body.contains("service-secret-marker"));
        assert!(!response.body.contains("postgres://"));
    }

    #[test]
    fn slice11_service_rejects_unknown_route_and_invalid_json() {
        let dir = create_temp_dir("slice11-routing");

        let unknown = service_response("POST", "/api/v1/postgres/apply", "{}", &dir);
        assert_eq!(unknown.status_code, 404);
        assert!(!unknown.body.contains("direct apply"));

        let invalid_json = service_response("POST", "/api/v1/repo/status", "{ invalid json", &dir);
        assert_eq!(invalid_json.status_code, 400);
        assert!(invalid_json.body.contains("Invalid JSON request body"));
    }

    #[test]
    fn slice9_malformed_postgres_urls_are_rejected_without_leaking_values() {
        let invalid_url = "not-a-postgres-url-with-sensitive-marker";
        let export_args = ParsedArgs::parse(&[
            "export".to_string(),
            "postgres".to_string(),
            "--all".to_string(),
            "--url".to_string(),
            invalid_url.to_string(),
        ])
        .expect("parse export");
        let export_report = export_postgres_command(Path::new("."), export_args);
        assert!(!export_report.success);
        assert!(export_report
            .errors
            .iter()
            .any(|error| error.contains("Invalid PostgreSQL connection URL")));
        assert!(!export_report.to_json().contains(invalid_url));

        let inspect_report = inspect_postgres_command(Some(invalid_url.to_string()), None);
        assert!(!inspect_report.success);
        assert!(inspect_report
            .errors
            .iter()
            .any(|error| error.contains("Invalid PostgreSQL connection URL")));
        assert!(!inspect_report.to_json().contains(invalid_url));
    }

    #[test]
    fn slice9_exit_code_contract_for_key_paths_is_stable() {
        let dir = create_temp_dir("slice9-exit-codes");
        init_git_repo(&dir);
        create_complete_structure(&dir);
        commit_all(&dir, "complete structure");

        let status_args = vec![
            "repo".to_string(),
            "status".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let status = run_cli(&status_args, Ok(dir.as_path())).expect("status cli");
        assert_eq!(status.exit_code, 0);

        let invalid_args = vec!["inspect".to_string(), "mysql".to_string()];
        assert!(ParsedArgs::parse(&invalid_args).is_err());

        let compare_report =
            compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);
        assert!(compare_report.success);
        assert!(!compare_report.database_only.is_empty());

        let (config, state) = reference_config_and_state();
        let database_rows = vec![reference_row(&[
            ("code", Some("CASH")),
            ("name", Some("Cash Live Difference")),
        ])];
        let data_result = compare_reference_data_table(
            &config,
            &state,
            &database_rows,
            &reference_database_columns(),
        );
        assert_eq!(data_result.row_counts.repo_different, 1);

        let blocked_plan = plan_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
            &PlanSelection::include_all(),
        );
        assert!(blocked_plan.success);
        assert!(!blocked_plan.blocked_items.is_empty());

        let blocked_release = release_postgres_with_inventory(
            &dir,
            &sample_inventory(),
            &ExportSelection::Table {
                schema: "dbstate_slice2".to_string(),
                table: "sample_accounts".to_string(),
            },
            &PlanSelection::include_all(),
            "slice9_blocked",
            true,
        );
        assert!(!blocked_release.success);
        assert!(!blocked_release.blocked_items.is_empty());
    }

    #[test]
    fn cli_rejects_apply_and_database_mutation_commands() {
        let invalid_commands = [
            vec!["apply".to_string()],
            vec!["execute".to_string()],
            vec!["compare".to_string()],
            vec!["inspect".to_string(), "mysql".to_string()],
        ];

        for args in invalid_commands {
            assert!(ParsedArgs::parse(&args).is_err());
        }

        assert!(ParsedArgs::parse(&["inspect".to_string(), "postgres".to_string()]).is_ok());
        assert!(ParsedArgs::parse(&[
            "export".to_string(),
            "postgres".to_string(),
            "--all".to_string()
        ])
        .is_ok());
        assert!(ParsedArgs::parse(&[
            "sync".to_string(),
            "postgres".to_string(),
            "--all".to_string()
        ])
        .is_ok());
        assert!(ParsedArgs::parse(&[
            "compare".to_string(),
            "postgres".to_string(),
            "--all".to_string()
        ])
        .is_ok());
        assert!(ParsedArgs::parse(&[
            "plan".to_string(),
            "postgres".to_string(),
            "--all".to_string(),
            "--include".to_string(),
            "table:dbstate_slice2.sample_accounts".to_string()
        ])
        .is_ok());
        assert!(ParsedArgs::parse(&[
            "release".to_string(),
            "postgres".to_string(),
            "--all".to_string(),
            "--name".to_string(),
            "slice7".to_string(),
            "--dry-run".to_string()
        ])
        .is_ok());
        assert!(ParsedArgs::parse(&[
            "data-compare".to_string(),
            "postgres".to_string(),
            "--all".to_string()
        ])
        .is_ok());
        assert!(ParsedArgs::parse(&[
            "data-compare".to_string(),
            "postgres".to_string(),
            "--table".to_string(),
            "dbstate_ref.payment_methods".to_string()
        ])
        .is_ok());
        assert!(ParsedArgs::parse(&[
            "compare".to_string(),
            "postgres".to_string(),
            "--all".to_string(),
            "--include".to_string(),
            "schema:dbstate_slice2".to_string()
        ])
        .is_err());
    }
}
