use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use postgres::{Client, NoTls};

const DEFAULT_REGISTRY: &str = "version: 1\ntables: []\n";
const DEFERRED_OBJECT_TYPES: &[&str] = &[
    "extensions",
    "enums",
    "sequences",
    "primaryKeys",
    "foreignKeys",
    "uniqueConstraints",
    "checkConstraints",
    "indexes",
    "views",
    "materializedViews",
    "functions",
    "triggers",
    "grants",
    "rlsPolicies",
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
}

impl CommandKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::RepoStatus => "repo status",
            Self::Init => "init",
            Self::InspectPostgres => "inspect postgres",
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandOutput {
    Project(ProjectReport),
    Inspection(InspectionReport),
}

impl CommandOutput {
    pub fn to_text(&self) -> String {
        match self {
            Self::Project(report) => report.to_text(),
            Self::Inspection(report) => report.to_text(),
        }
    }

    pub fn to_json(&self) -> String {
        match self {
            Self::Project(report) => report.to_json(),
            Self::Inspection(report) => report.to_json(),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionCounts {
    pub schemas: usize,
    pub tables: usize,
    pub columns: usize,
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
    pub counts: InspectionCounts,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
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
            let report =
                inspect_postgres_command(parsed.url, env::var("DBSTATE_POSTGRES_URL").ok());
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format: parsed.format,
                output: CommandOutput::Inspection(report),
                exit_code,
            })
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedArgs {
    command: CommandKind,
    format: OutputFormat,
    dry_run: bool,
    url: Option<String>,
}

impl ParsedArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        if args.is_empty() {
            return Err(usage());
        }

        let mut format = OutputFormat::Text;
        let mut dry_run = false;
        let mut url = None;
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
            _ => return Err(usage()),
        };

        if dry_run && command != CommandKind::Init {
            return Err("--dry-run is only supported for dbstate init".to_string());
        }

        if url.is_some() && command != CommandKind::InspectPostgres {
            return Err("--url is only supported for dbstate inspect postgres".to_string());
        }

        Ok(Self {
            command,
            format,
            dry_run,
            url,
        })
    }
}

fn usage() -> String {
    "Usage:\n  dbstate repo status [--format json]\n  dbstate init [--dry-run] [--format json]\n  dbstate inspect postgres [--url <postgres-url>] [--format json]".to_string()
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
    let mut report = empty_inspection_report(CommandKind::InspectPostgres);

    let Some(connection_url) = resolve_postgres_url(cli_url, env_url) else {
        report.errors.push(
            "Missing PostgreSQL connection URL. Provide --url or DBSTATE_POSTGRES_URL.".to_string(),
        );
        return report;
    };

    match inspect_postgres(&connection_url) {
        Ok(inventory) => {
            report.schemas = inventory.schemas;
            report.tables = inventory.tables;
            report.columns = inventory.columns;
            report.counts = InspectionCounts {
                schemas: report.schemas.len(),
                tables: report.tables.len(),
                columns: report.columns.len(),
            };
            report.success = true;
        }
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
        }
    }

    report
}

fn resolve_postgres_url(cli_url: Option<String>, env_url: Option<String>) -> Option<String> {
    cli_url
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env_url.filter(|value| !value.trim().is_empty()))
}

#[derive(Debug, Clone)]
pub struct PostgresInventory {
    pub schemas: Vec<SchemaInfo>,
    pub tables: Vec<TableInfo>,
    pub columns: Vec<ColumnInfo>,
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
                    pg_catalog.pg_get_expr(ad.adbin, ad.adrelid) IS NOT NULL
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
        })
        .collect();

    Ok(PostgresInventory {
        schemas,
        tables,
        columns,
    })
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
        ],
        schemas: Vec::new(),
        tables: Vec::new(),
        columns: Vec::new(),
        counts: InspectionCounts {
            schemas: 0,
            tables: 0,
            columns: 0,
        },
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

pub fn redact_message(message: &str, secret: &str) -> String {
    let redacted = message.replace(secret, "<redacted>");
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
    path.to_string_lossy().replace('\\', "/")
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

fn write_counts_field(json: &mut String, name: &str, counts: &InspectionCounts) {
    json.push(',');
    write!(
        json,
        "\"{}\":{{\"schemas\":{},\"tables\":{},\"columns\":{}}}",
        escape_json(name),
        counts.schemas,
        counts.tables,
        counts.columns
    )
    .ok();
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let resolved = resolve_postgres_url(
            Some("postgres://cli-user:cli-password@example.invalid/db".to_string()),
            Some("postgres://env-user:env-password@example.invalid/db".to_string()),
        )
        .expect("resolved url");

        assert_eq!(
            resolved,
            "postgres://cli-user:cli-password@example.invalid/db"
        );
    }

    #[test]
    fn redaction_removes_raw_url_and_password() {
        let raw = "postgres://user:secret-password@example.invalid:5432/db";
        let redacted = redact_message(&format!("could not connect to {raw}"), raw);

        assert!(!redacted.contains(raw));
        assert!(!redacted.contains("secret-password"));
        assert!(redacted.contains("<redacted>"));
    }

    #[test]
    fn inspection_json_output_includes_expected_fields_and_no_secrets() {
        let mut report = empty_inspection_report(CommandKind::InspectPostgres);
        report.errors.push(redact_postgres_url(
            "postgres://user:secret-token@example.invalid:5432/db failed",
        ));
        let json = report.to_json();

        for field in [
            "\"command\"",
            "\"success\"",
            "\"databaseType\"",
            "\"inspectionScope\"",
            "\"schemas\"",
            "\"tables\"",
            "\"columns\"",
            "\"counts\"",
            "\"warnings\"",
            "\"errors\"",
            "\"deferredObjectTypes\"",
        ] {
            assert!(json.contains(field), "missing JSON field {field}");
        }

        assert!(!json.contains("secret-token"));
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
            }],
        };

        assert_eq!(inventory.schemas[0].name, "app");
        assert_eq!(inventory.tables[0].table_name, "orders");
        assert_eq!(inventory.columns[0].column_name, "id");
    }

    #[test]
    fn deferred_object_types_are_explicit() {
        let report = empty_inspection_report(CommandKind::InspectPostgres);

        assert!(report
            .deferred_object_types
            .contains(&"extensions".to_string()));
        assert!(report
            .deferred_object_types
            .contains(&"indexes".to_string()));
        assert!(report
            .deferred_object_types
            .contains(&"functions".to_string()));
        assert!(report.deferred_object_types.contains(&"grants".to_string()));
    }

    #[test]
    fn cli_rejects_apply_and_database_mutation_commands() {
        let invalid_commands = [
            vec!["apply".to_string()],
            vec!["execute".to_string()],
            vec!["data-compare".to_string()],
            vec!["compare".to_string()],
            vec!["inspect".to_string(), "mysql".to_string()],
        ];

        for args in invalid_commands {
            assert!(ParsedArgs::parse(&args).is_err());
        }

        assert!(ParsedArgs::parse(&["inspect".to_string(), "postgres".to_string()]).is_ok());
    }
}
