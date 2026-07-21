use crate::postgres::{
    invalid_postgres_url_message, is_postgres_connection_url, resolve_postgres_url,
};
use crate::project::{project_structure_allows_release_subfolder_backfill, GitHandoffWorkflow};
use crate::repository::discovery::{
    is_supported_table_desired_state_type, render_database_objects_for_selection,
};
use crate::repository::objects::*;
use crate::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

pub(crate) fn export_postgres_command(cwd: &Path, parsed: ParsedArgs) -> ExportReport {
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
    pub(crate) fn from_options(
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

    pub(crate) fn scope_name(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::Schema(schema) => format!("schema:{schema}"),
            Self::Table { schema, table } => format!("table:{schema}.{table}"),
        }
    }

    pub(crate) fn selected_schemas(&self) -> Vec<String> {
        match self {
            Self::Schema(schema) => vec![schema.clone()],
            _ => Vec::new(),
        }
    }

    pub(crate) fn selected_tables(&self) -> Vec<String> {
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
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure
        && !project_structure_allows_release_subfolder_backfill(&project)
    {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return report;
    }
    if project_structure_allows_release_subfolder_backfill(&project) {
        report.warnings.push(
            "Project is missing release artifact subfolders. Run dbstate init to create database/releases/objects and database/releases/reference-data."
                .to_string(),
        );
    }
    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    let plan = match plan_export(&root, inventory, selection) {
        Ok(plan) => plan,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    report.warnings.extend(plan.warnings);
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

    let guard = scoped_write_guard(
        &root,
        report.branch.as_deref(),
        project.default_branch.as_deref(),
        &report.planned_files,
        GitHandoffWorkflow::SchemaExport,
        &report.export_scope,
    );
    report.warnings.extend(guard.warnings);
    if !guard.allowed {
        report.errors.extend(guard.errors);
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
            if !is_supported_table_desired_state_type(&selected_table.table_type) {
                return Err(format!(
                    "Selected table '{schema}.{table}' is not a supported base or partitioned table."
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

pub(crate) fn sync_postgres_command(cwd: &Path, parsed: ParsedArgs) -> SyncReport {
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
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure
        && !project_structure_allows_release_subfolder_backfill(&project)
    {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return report;
    }
    if project_structure_allows_release_subfolder_backfill(&project) {
        report.warnings.push(
            "Project is missing release artifact subfolders. Run dbstate init to create database/releases/objects and database/releases/reference-data."
                .to_string(),
        );
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
    report.warnings.extend(plan.warnings.clone());

    if dry_run {
        report.success = report.errors.is_empty();
        return report;
    }

    let intended_write_paths: Vec<String> = plan
        .writes
        .iter()
        .map(|write| write.relative_path.clone())
        .collect();
    let guard = scoped_write_guard(
        &root,
        report.branch.as_deref(),
        project.default_branch.as_deref(),
        &intended_write_paths,
        GitHandoffWorkflow::SchemaExport,
        &report.sync_scope,
    );
    report.warnings.extend(guard.warnings);
    if !guard.allowed {
        report.errors.extend(guard.errors);
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
                if !is_supported_table_desired_state_type(&table.table_type) {
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
            if !is_supported_table_desired_state_type(&selected_table.table_type) {
                return Err(format!(
                    "Selected table '{schema}.{table}' is not a supported base or partitioned table."
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

pub(crate) fn empty_export_report(dry_run: bool) -> ExportReport {
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

pub(crate) fn empty_sync_report(dry_run: bool) -> SyncReport {
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
