use crate::postgres::{
    invalid_postgres_url_message, is_postgres_connection_url, quote_postgres_identifier,
    resolve_postgres_url,
};
use crate::project::{project_structure_allows_release_subfolder_backfill, GitHandoffWorkflow};
use crate::repository::safe_file_component;
use crate::*;
use ::postgres::{Client, NoTls};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
    pub foreign_key_references: Vec<ReferenceDataForeignKeyReference>,
    pub row_counts: ReferenceDataCompareCounts,
    pub row_results: Vec<ReferenceRowCompareResult>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceDataForeignKeyReference {
    pub constraint_name: String,
    pub referencing_schema: String,
    pub referencing_table: String,
    pub referencing_columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceRowCompareResult {
    pub table_name: String,
    pub row_key: String,
    pub classification: String,
    pub key_values: BTreeMap<String, String>,
    pub repository_values: BTreeMap<String, String>,
    pub database_values: BTreeMap<String, String>,
    pub changed_columns: Vec<String>,
    pub ignored_columns: Vec<String>,
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
pub struct ReferenceDataReviewScriptReport {
    pub command: String,
    pub success: bool,
    pub dry_run: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub script_name: String,
    pub selected_tables: Vec<String>,
    pub affected_rows: usize,
    pub insert_count: usize,
    pub update_count: usize,
    pub manual_review_count: usize,
    pub database_only_count: usize,
    pub foreign_key_warning_count: usize,
    pub delete_generated_count: usize,
    pub planned_artifacts: Vec<String>,
    pub created_artifacts: Vec<String>,
    pub script_content: String,
    pub summary_content: String,
    pub risk_content: String,
    pub manifest_content: String,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub(crate) fn data_compare_postgres_command(
    cwd: &Path,
    parsed: ParsedArgs,
) -> ReferenceDataCompareReport {
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
    Tables(Vec<String>),
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
            Self::Tables(tables) => format!("tables:{}", tables.join(",")),
        }
    }

    fn selected_tables(&self) -> Vec<String> {
        match self {
            Self::All => Vec::new(),
            Self::Table(table) => vec![table.clone()],
            Self::Tables(tables) => tables.clone(),
        }
    }

    fn includes_table(&self, table_name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Table(selected) => selected == table_name,
            Self::Tables(selected) => selected.iter().any(|table| table == table_name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceDataStatusReport {
    pub command: String,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub repository_path_used: String,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub dbstate_project_status: DbStateProjectStatus,
    pub registry_path: String,
    pub registry_exists: bool,
    pub status: String,
    pub configured_tables: Vec<ReferenceDataConfiguredTableStatus>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataConfiguredTableStatus {
    pub schema: String,
    pub name: String,
    pub table_name: String,
    pub file: String,
    pub key_columns: Vec<String>,
    pub ignored_columns: Vec<String>,
    pub masked_columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataDatabaseTablesReport {
    pub command: String,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub tables: Vec<ReferenceDataCandidateTable>,
    pub configured_tables: Vec<ReferenceDataConfiguredTableStatus>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataCandidateTable {
    pub schema: String,
    pub name: String,
    pub table_name: String,
    pub columns: Vec<ReferenceDataCandidateColumn>,
    pub suggested_key_columns: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataCandidateColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub ordinal: i32,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataExportSelection {
    pub schema: String,
    pub name: String,
    pub key_columns: Vec<String>,
    pub versioned_columns: Vec<String>,
    pub masked_columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataExportReport {
    pub command: String,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub registry_path: String,
    pub registry_yaml: String,
    pub table_previews: Vec<ReferenceDataTableExportPreview>,
    pub files_created: Vec<String>,
    pub files_updated: Vec<String>,
    pub files_unchanged: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataTableExportPreview {
    pub schema: String,
    pub name: String,
    pub table_name: String,
    pub file: String,
    pub yaml: String,
    pub key_columns: Vec<String>,
    pub versioned_columns: Vec<String>,
    pub ignored_columns: Vec<String>,
    pub masked_columns: Vec<String>,
    pub row_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataRegistry {
    pub(crate) version: i64,
    pub(crate) tables: Vec<ReferenceDataTableConfig>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataTableConfig {
    pub(crate) name: String,
    pub(crate) file: String,
    pub(crate) key_columns: Vec<String>,
    pub(crate) ignore_columns: Vec<String>,
    pub(crate) masked_columns: Vec<String>,
    pub(crate) allow_deletes: bool,
}

pub fn reference_data_status(cwd: &Path) -> ReferenceDataStatusReport {
    let project = status_report(cwd, CommandKind::RepoStatus);
    let repository_path_used = project
        .git_root
        .clone()
        .unwrap_or_else(|| project.repository_path.clone());
    let mut report = ReferenceDataStatusReport {
        command: "reference-data status".to_string(),
        success: project.is_git_repository,
        repository_path: project.repository_path.clone(),
        git_root: project.git_root.clone(),
        repository_path_used,
        is_git_repository: project.is_git_repository,
        branch: project.branch.clone(),
        working_tree_status: project.working_tree_status,
        is_dirty: project.is_dirty,
        dbstate_project_status: project.dbstate_project_status,
        registry_path: "database/reference-data/dbstate.reference-data.yml".to_string(),
        registry_exists: false,
        status: if project.is_git_repository {
            "checking".to_string()
        } else {
            "notGitRepository".to_string()
        },
        configured_tables: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    if !project.is_git_repository {
        report
            .errors
            .push("Selected path is not inside a Git repository.".to_string());
        return report;
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    let registry_path = root.join("database/reference-data/dbstate.reference-data.yml");
    report.registry_exists = registry_path.is_file();
    if !report.registry_exists {
        report.success = true;
        report.status = "missingRegistry".to_string();
        report.warnings.push(
            "Reference-data registry was not found at database/reference-data/dbstate.reference-data.yml under the selected repository. DbState will not infer reference-data tables automatically."
                .to_string(),
        );
        return report;
    }

    match read_reference_data_registry(&root) {
        Ok(registry) => {
            report.configured_tables = registry
                .tables
                .iter()
                .map(reference_data_config_status)
                .collect();
            if report.configured_tables.is_empty() {
                report.status = "emptyRegistry".to_string();
                report.warnings.push(
                    "No configured reference-data tables. DbState will not infer reference-data tables automatically."
                        .to_string(),
                );
            } else {
                report.status = "ready".to_string();
            }
        }
        Err(error) => {
            report.success = false;
            report.status = "invalidRegistry".to_string();
            report.errors.push(error);
        }
    }

    report
}

fn reference_data_config_status(
    config: &ReferenceDataTableConfig,
) -> ReferenceDataConfiguredTableStatus {
    let (schema, name) = split_schema_qualified_name(&config.name)
        .unwrap_or_else(|_| ("".to_string(), config.name.clone()));
    ReferenceDataConfiguredTableStatus {
        schema,
        name,
        table_name: config.name.clone(),
        file: config.file.clone(),
        key_columns: config.key_columns.clone(),
        ignored_columns: config.ignore_columns.clone(),
        masked_columns: config.masked_columns.clone(),
    }
}

pub fn reference_data_database_tables_with_connection(
    cwd: &Path,
    connection_url: &str,
) -> Result<ReferenceDataDatabaseTablesReport, String> {
    let project = status_report(cwd, CommandKind::RepoStatus);
    let mut report = ReferenceDataDatabaseTablesReport {
        command: "reference-data database-tables".to_string(),
        success: false,
        repository_path: project.repository_path.clone(),
        git_root: project.git_root.clone(),
        is_git_repository: project.is_git_repository,
        branch: project.branch.clone(),
        working_tree_status: project.working_tree_status,
        is_dirty: project.is_dirty,
        database_type: "postgresql".to_string(),
        tables: Vec::new(),
        configured_tables: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };
    if !project.is_git_repository {
        report
            .errors
            .push("Selected path is not inside a Git repository.".to_string());
        return Ok(report);
    }
    if !is_postgres_connection_url(connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return Ok(report);
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    match read_reference_data_registry(&root) {
        Ok(registry) => {
            report.configured_tables = registry
                .tables
                .iter()
                .map(reference_data_config_status)
                .collect();
        }
        Err(error) => {
            report.warnings.push(format!(
                "Reference-data registry could not be loaded for table registry status: {error}"
            ));
        }
    }

    let mut client = Client::connect(connection_url, NoTls)
        .map_err(|error| redact_message(&error.to_string(), connection_url))?;
    report.tables = reference_data_candidate_tables_from_postgres(&mut client)?;
    report.success = true;
    Ok(report)
}

pub fn reference_data_export_preview_with_connection(
    cwd: &Path,
    connection_url: &str,
    selections: &[ReferenceDataExportSelection],
) -> Result<ReferenceDataExportReport, String> {
    reference_data_export_with_connection(cwd, connection_url, selections, false)
}

pub fn reference_data_export_write_with_connection(
    cwd: &Path,
    connection_url: &str,
    selections: &[ReferenceDataExportSelection],
) -> Result<ReferenceDataExportReport, String> {
    reference_data_export_with_connection(cwd, connection_url, selections, true)
}

fn reference_data_export_with_connection(
    cwd: &Path,
    connection_url: &str,
    selections: &[ReferenceDataExportSelection],
    write_files: bool,
) -> Result<ReferenceDataExportReport, String> {
    let project = status_report(cwd, CommandKind::RepoStatus);
    let mut report = ReferenceDataExportReport {
        command: if write_files {
            "reference-data export write".to_string()
        } else {
            "reference-data export preview".to_string()
        },
        success: false,
        repository_path: project.repository_path.clone(),
        git_root: project.git_root.clone(),
        is_git_repository: project.is_git_repository,
        branch: project.branch.clone(),
        working_tree_status: project.working_tree_status,
        is_dirty: project.is_dirty,
        database_type: "postgresql".to_string(),
        registry_path: "database/reference-data/dbstate.reference-data.yml".to_string(),
        registry_yaml: String::new(),
        table_previews: Vec::new(),
        files_created: Vec::new(),
        files_updated: Vec::new(),
        files_unchanged: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };
    if !project.is_git_repository {
        report
            .errors
            .push("Selected path is not inside a Git repository.".to_string());
        return Ok(report);
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure
        && !project_structure_allows_release_subfolder_backfill(&project)
    {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Initialize the repository before exporting reference data."
                .to_string(),
        );
        return Ok(report);
    }
    if project_structure_allows_release_subfolder_backfill(&project) {
        report.warnings.push(
            "Project is missing release artifact subfolders. Run dbstate init to create database/releases/objects and database/releases/reference-data."
                .to_string(),
        );
    }
    if selections.is_empty() {
        report
            .errors
            .push("Select at least one database table to export as reference data.".to_string());
        return Ok(report);
    }
    if !is_postgres_connection_url(connection_url) {
        report.errors.push(invalid_postgres_url_message());
        return Ok(report);
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    let mut client = Client::connect(connection_url, NoTls)
        .map_err(|error| redact_message(&error.to_string(), connection_url))?;
    let existing_registry = read_reference_data_registry(&root).ok();
    let mut combined_tables: BTreeMap<String, ReferenceDataTableConfig> = BTreeMap::new();
    if let Some(registry) = existing_registry {
        for table in registry.tables {
            combined_tables.insert(table.name.clone(), table);
        }
    }

    for selection in selections {
        let preview = reference_data_export_table_preview(&mut client, selection)?;
        let config = ReferenceDataTableConfig {
            name: preview.table_name.clone(),
            file: preview
                .file
                .strip_prefix("database/reference-data/")
                .unwrap_or(&preview.file)
                .to_string(),
            key_columns: preview.key_columns.clone(),
            ignore_columns: preview.ignored_columns.clone(),
            masked_columns: preview.masked_columns.clone(),
            allow_deletes: false,
        };
        if combined_tables
            .insert(config.name.clone(), config)
            .is_some()
        {
            report.warnings.push(format!(
                "Existing reference-data registry entry for '{}' will be updated.",
                preview.table_name
            ));
        }
        report.table_previews.push(preview);
    }

    report.registry_yaml = render_reference_data_registry_yaml(&combined_tables);
    if write_files {
        let mut intended_paths =
            vec!["database/reference-data/dbstate.reference-data.yml".to_string()];
        intended_paths.extend(
            report
                .table_previews
                .iter()
                .map(|preview| preview.file.clone()),
        );
        let summary = report
            .table_previews
            .first()
            .map(|preview| preview.table_name.as_str())
            .unwrap_or("reference data export");
        let guard = scoped_write_guard(
            &root,
            report.branch.as_deref(),
            project.default_branch.as_deref(),
            &intended_paths,
            GitHandoffWorkflow::ReferenceDataExport,
            summary,
        );
        report.warnings.extend(guard.warnings);
        if !guard.allowed {
            report.errors.extend(guard.errors);
            report.success = false;
            return Ok(report);
        }
        write_reference_data_export_files(&root, &mut report)?;
    }
    report.success = report.errors.is_empty();
    Ok(report)
}

fn reference_data_candidate_tables_from_postgres(
    client: &mut Client,
) -> Result<Vec<ReferenceDataCandidateTable>, String> {
    let rows = client
        .query(
            "SELECT n.nspname,
                    c.relname,
                    a.attname,
                    a.attnum::int,
                    format_type(a.atttypid, a.atttypmod),
                    NOT a.attnotnull,
                    EXISTS (
                        SELECT 1
                        FROM pg_constraint con
                        WHERE con.conrelid = c.oid
                          AND con.contype = 'p'
                          AND a.attnum = ANY(con.conkey)
                    ) AS is_primary_key,
                    EXISTS (
                        SELECT 1
                        FROM pg_index idx
                        WHERE idx.indrelid = c.oid
                          AND idx.indisunique
                          AND a.attnum = ANY(idx.indkey)
                    ) AS is_unique
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             JOIN pg_attribute a ON a.attrelid = c.oid
             WHERE c.relkind = 'r'
               AND a.attnum > 0
               AND NOT a.attisdropped
               AND n.nspname NOT LIKE 'pg_%'
               AND n.nspname <> 'information_schema'
             ORDER BY n.nspname, c.relname, a.attnum",
            &[],
        )
        .map_err(|_| "PostgreSQL reference-data table metadata query failed.".to_string())?;

    let mut tables: BTreeMap<String, ReferenceDataCandidateTable> = BTreeMap::new();
    for row in rows {
        let schema: String = row.get(0);
        let name: String = row.get(1);
        let table_name = format!("{schema}.{name}");
        let column = ReferenceDataCandidateColumn {
            name: row.get(2),
            ordinal: row.get(3),
            data_type: row.get(4),
            nullable: row.get(5),
            is_primary_key: row.get(6),
            is_unique: row.get(7),
        };
        let table =
            tables
                .entry(table_name.clone())
                .or_insert_with(|| ReferenceDataCandidateTable {
                    schema: schema.clone(),
                    name: name.clone(),
                    table_name: table_name.clone(),
                    columns: Vec::new(),
                    suggested_key_columns: Vec::new(),
                    warnings: Vec::new(),
                });
        if column.is_primary_key {
            table.suggested_key_columns.push(column.name.clone());
        }
        table.columns.push(column);
    }
    Ok(tables.into_values().collect())
}

fn reference_data_export_table_preview(
    client: &mut Client,
    selection: &ReferenceDataExportSelection,
) -> Result<ReferenceDataTableExportPreview, String> {
    validate_export_selection_identity(selection)?;
    let table_name = format!("{}.{}", selection.schema, selection.name);
    let columns =
        reference_data_table_columns_from_postgres(client, &selection.schema, &selection.name)?;
    if columns.is_empty() {
        return Err(format!(
            "Selected reference-data export table '{}' was not found in PostgreSQL.",
            table_name
        ));
    }
    let column_names: Vec<String> = columns.iter().map(|column| column.name.clone()).collect();
    validate_export_columns("keyColumns", &selection.key_columns, &column_names)?;
    validate_export_columns(
        "versionedColumns",
        &selection.versioned_columns,
        &column_names,
    )?;
    validate_export_columns("maskedColumns", &selection.masked_columns, &column_names)?;
    if selection.key_columns.is_empty() {
        return Err(format!(
            "Reference-data export table '{}' must have at least one key column.",
            table_name
        ));
    }
    for key in &selection.key_columns {
        if selection.masked_columns.contains(key) {
            return Err(format!(
                "Reference-data export table '{}' key column '{}' cannot be masked.",
                table_name, key
            ));
        }
    }
    let mut versioned_columns = selection.versioned_columns.clone();
    versioned_columns.sort();
    versioned_columns.dedup();
    let mut masked_columns = selection.masked_columns.clone();
    masked_columns.sort();
    masked_columns.dedup();

    let mut compared_columns: BTreeSet<String> = selection.key_columns.iter().cloned().collect();
    compared_columns.extend(versioned_columns.iter().cloned());
    compared_columns.extend(masked_columns.iter().cloned());
    let ignored_columns: Vec<String> = column_names
        .iter()
        .filter(|column| !compared_columns.contains(*column))
        .cloned()
        .collect();
    let mut warnings = Vec::new();
    if versioned_columns.is_empty() && masked_columns.is_empty() {
        warnings.push(format!(
            "Reference-data export table '{}' has no versioned columns; generated rows contain keys only.",
            table_name
        ));
    }
    if !masked_columns.is_empty() {
        warnings.push(format!(
            "Masked column values for '{}' are written as [masked] placeholders; raw database values are not exported.",
            table_name
        ));
    }
    let row_count = reference_data_table_row_count(client, &selection.schema, &selection.name)?;
    if row_count > 1000 {
        warnings.push(format!(
            "Reference-data export table '{}' has {row_count} rows. Review generated YAML before committing.",
            table_name
        ));
    }
    let rows = reference_data_export_rows_from_postgres(
        client,
        selection,
        &versioned_columns,
        &masked_columns,
    )?;
    let file = reference_data_table_output_file(&selection.schema, &selection.name)?;
    let yaml = render_reference_data_table_yaml(&table_name, &rows);
    Ok(ReferenceDataTableExportPreview {
        schema: selection.schema.clone(),
        name: selection.name.clone(),
        table_name,
        file,
        yaml,
        key_columns: selection.key_columns.clone(),
        versioned_columns,
        ignored_columns,
        masked_columns,
        row_count,
        warnings,
    })
}

fn reference_data_table_columns_from_postgres(
    client: &mut Client,
    schema: &str,
    table: &str,
) -> Result<Vec<ReferenceDataCandidateColumn>, String> {
    let rows = client
        .query(
            "SELECT a.attname,
                    a.attnum::int,
                    format_type(a.atttypid, a.atttypmod),
                    NOT a.attnotnull,
                    EXISTS (
                        SELECT 1
                        FROM pg_constraint con
                        WHERE con.conrelid = c.oid
                          AND con.contype = 'p'
                          AND a.attnum = ANY(con.conkey)
                    ) AS is_primary_key,
                    EXISTS (
                        SELECT 1
                        FROM pg_index idx
                        WHERE idx.indrelid = c.oid
                          AND idx.indisunique
                          AND a.attnum = ANY(idx.indkey)
                    ) AS is_unique
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             JOIN pg_attribute a ON a.attrelid = c.oid
             WHERE c.relkind = 'r'
               AND n.nspname = $1
               AND c.relname = $2
               AND a.attnum > 0
               AND NOT a.attisdropped
             ORDER BY a.attnum",
            &[&schema, &table],
        )
        .map_err(|_| {
            format!(
                "PostgreSQL reference-data column metadata query failed for '{schema}.{table}'."
            )
        })?;
    Ok(rows
        .into_iter()
        .map(|row| ReferenceDataCandidateColumn {
            name: row.get(0),
            ordinal: row.get(1),
            data_type: row.get(2),
            nullable: row.get(3),
            is_primary_key: row.get(4),
            is_unique: row.get(5),
        })
        .collect())
}

fn reference_data_table_row_count(
    client: &mut Client,
    schema: &str,
    table: &str,
) -> Result<usize, String> {
    let sql = format!(
        "SELECT count(*)::bigint FROM {}.{}",
        quote_postgres_identifier(schema),
        quote_postgres_identifier(table)
    );
    let row = client.query_one(&sql, &[]).map_err(|_| {
        format!("PostgreSQL reference-data row count query failed for '{schema}.{table}'.")
    })?;
    let count: i64 = row.get(0);
    Ok(count.max(0) as usize)
}

fn reference_data_export_rows_from_postgres(
    client: &mut Client,
    selection: &ReferenceDataExportSelection,
    versioned_columns: &[String],
    masked_columns: &[String],
) -> Result<Vec<BTreeMap<String, Option<String>>>, String> {
    let mut selected_columns: BTreeSet<String> = selection.key_columns.iter().cloned().collect();
    selected_columns.extend(versioned_columns.iter().cloned());
    let selected_columns: Vec<String> = selected_columns
        .into_iter()
        .filter(|column| !masked_columns.contains(column))
        .collect();
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
    let order_by = selection
        .key_columns
        .iter()
        .map(|column| quote_postgres_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT {select_list} FROM {}.{} ORDER BY {order_by}",
        quote_postgres_identifier(&selection.schema),
        quote_postgres_identifier(&selection.name)
    );
    let rows = client.query(&sql, &[]).map_err(|_| {
        format!(
            "PostgreSQL reference-data export SELECT failed for '{}.{}'.",
            selection.schema, selection.name
        )
    })?;
    let mut output = Vec::new();
    for row in rows {
        let mut values = BTreeMap::new();
        for key in &selection.key_columns {
            let value: Option<String> = row.try_get(key.as_str()).map_err(|_| {
                format!(
                    "Could not read key column '{}' from '{}.{}'.",
                    key, selection.schema, selection.name
                )
            })?;
            values.insert(key.clone(), value);
        }
        for column in versioned_columns {
            if masked_columns.contains(column) {
                continue;
            }
            let value: Option<String> = row.try_get(column.as_str()).map_err(|_| {
                format!(
                    "Could not read versioned column '{}' from '{}.{}'.",
                    column, selection.schema, selection.name
                )
            })?;
            values.insert(column.clone(), value);
        }
        for column in masked_columns {
            values.insert(column.clone(), Some("[masked]".to_string()));
        }
        output.push(values);
    }
    Ok(output)
}

fn validate_export_selection_identity(
    selection: &ReferenceDataExportSelection,
) -> Result<(), String> {
    safe_file_component(&selection.schema)?;
    safe_file_component(&selection.name)?;
    if selection.schema.trim().is_empty() || selection.name.trim().is_empty() {
        return Err("Reference-data export tables require non-empty schema and name.".to_string());
    }
    Ok(())
}

fn validate_export_columns(
    field: &str,
    values: &[String],
    available_columns: &[String],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(format!("{field} contains duplicate column '{value}'."));
        }
        if !available_columns.contains(value) {
            return Err(format!("{field} contains unknown column '{value}'."));
        }
    }
    Ok(())
}

fn reference_data_table_output_file(schema: &str, table: &str) -> Result<String, String> {
    Ok(format!(
        "database/reference-data/tables/{}.{}.yml",
        safe_file_component(schema)?,
        safe_file_component(table)?
    ))
}

fn render_reference_data_registry_yaml(
    tables: &BTreeMap<String, ReferenceDataTableConfig>,
) -> String {
    let mut yaml = String::from("version: 1\ntables:\n");
    if tables.is_empty() {
        return "version: 1\ntables: []\n".to_string();
    }
    for config in tables.values() {
        let (schema, name) = split_schema_qualified_name(&config.name)
            .unwrap_or_else(|_| ("".to_string(), config.name.clone()));
        yaml.push_str(&format!("  - schema: {}\n", yaml_plain_or_quoted(&schema)));
        yaml.push_str(&format!("    name: {}\n", yaml_plain_or_quoted(&name)));
        yaml.push_str("    keyColumns:\n");
        for column in &config.key_columns {
            yaml.push_str(&format!("      - {}\n", yaml_plain_or_quoted(column)));
        }
        yaml.push_str("    ignoredColumns:");
        if config.ignore_columns.is_empty() {
            yaml.push_str(" []\n");
        } else {
            yaml.push('\n');
            for column in &config.ignore_columns {
                yaml.push_str(&format!("      - {}\n", yaml_plain_or_quoted(column)));
            }
        }
        yaml.push_str("    maskedColumns:");
        if config.masked_columns.is_empty() {
            yaml.push_str(" []\n");
        } else {
            yaml.push('\n');
            for column in &config.masked_columns {
                yaml.push_str(&format!("      - {}\n", yaml_plain_or_quoted(column)));
            }
        }
    }
    yaml
}

fn render_reference_data_table_yaml(
    table_name: &str,
    rows: &[BTreeMap<String, Option<String>>],
) -> String {
    if rows.is_empty() {
        return format!(
            "version: 1\ntable: {}\nrows: []\n",
            yaml_plain_or_quoted(table_name)
        );
    }
    let mut yaml = format!(
        "version: 1\ntable: {}\nrows:\n",
        yaml_plain_or_quoted(table_name)
    );
    for row in rows {
        yaml.push_str("  -");
        let mut first = true;
        for (column, value) in row {
            if first {
                yaml.push_str(&format!(
                    " {}: {}\n",
                    yaml_plain_or_quoted(column),
                    yaml_scalar(value.as_deref())
                ));
                first = false;
            } else {
                yaml.push_str(&format!(
                    "    {}: {}\n",
                    yaml_plain_or_quoted(column),
                    yaml_scalar(value.as_deref())
                ));
            }
        }
    }
    yaml
}

fn write_reference_data_export_files(
    root: &Path,
    report: &mut ReferenceDataExportReport,
) -> Result<(), String> {
    let reference_root = root.join("database/reference-data");
    let tables_root = reference_root.join("tables");
    fs::create_dir_all(&tables_root)
        .map_err(|error| format!("Could not create database/reference-data/tables: {error}"))?;
    let registry_yaml = report.registry_yaml.clone();
    write_reference_data_file(
        root,
        "database/reference-data/dbstate.reference-data.yml",
        &registry_yaml,
        report,
    )?;
    for preview in report.table_previews.clone() {
        write_reference_data_file(root, &preview.file, &preview.yaml, report)?;
    }
    Ok(())
}

fn write_reference_data_file(
    root: &Path,
    relative_path: &str,
    content: &str,
    report: &mut ReferenceDataExportReport,
) -> Result<(), String> {
    if relative_path.contains("..")
        || relative_path.contains('\\')
        || relative_path.starts_with('/')
        || relative_path.contains(':')
        || !relative_path.starts_with("database/reference-data/")
    {
        return Err(format!(
            "Reference-data export path must stay under database/reference-data/: {relative_path}"
        ));
    }
    let target = root.join(relative_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create parent for {relative_path}: {error}"))?;
    }
    let existing = fs::read_to_string(&target).ok();
    match existing {
        Some(existing) if existing == content => {
            report.files_unchanged.push(relative_path.to_string());
        }
        Some(_) => {
            fs::write(&target, content)
                .map_err(|error| format!("Could not write {relative_path}: {error}"))?;
            report.files_updated.push(relative_path.to_string());
        }
        None => {
            fs::write(&target, content)
                .map_err(|error| format!("Could not write {relative_path}: {error}"))?;
            report.files_created.push(relative_path.to_string());
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ReferenceDataTableState {
    pub(crate) table_name: String,
    pub(crate) key_columns: Vec<String>,
    pub(crate) rows: Vec<ReferenceDataRow>,
}

#[derive(Debug, Clone)]
pub struct ReferenceDataRow {
    pub(crate) values: BTreeMap<String, Option<String>>,
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
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure
        && !project_structure_allows_release_subfolder_backfill(&project)
    {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return Ok(report);
    }
    if project_structure_allows_release_subfolder_backfill(&project) {
        report.warnings.push(
            "Project is missing release artifact subfolders. Run dbstate init to create database/releases/objects and database/releases/reference-data."
                .to_string(),
        );
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
    if let ReferenceDataSelection::Tables(selected) = selection {
        let configured: BTreeSet<String> = selected_configs
            .iter()
            .map(|config| config.name.clone())
            .collect();
        let missing: Vec<String> = selected
            .iter()
            .filter(|table| !configured.contains(*table))
            .cloned()
            .collect();
        if !missing.is_empty() {
            report.errors.push(format!(
                "Selected reference-data table is not configured in database/reference-data/dbstate.reference-data.yml: {}.",
                missing.join(", ")
            ));
            return Ok(report);
        }
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
        let mut table_result =
            compare_reference_data_table(&config, &state, &database_rows, &database_columns);
        match read_reference_data_foreign_key_references(&mut client, &config) {
            Ok(foreign_keys) => {
                if !foreign_keys.is_empty() {
                    let descriptions: Vec<String> = foreign_keys
                        .iter()
                        .map(reference_data_foreign_key_description)
                        .collect();
                    for row in &mut table_result.row_results {
                        if row.classification == "databaseOnly" {
                            row.warnings.push(format!(
                                "Potentially affected foreign keys: {}. DbState does not generate DELETE for database-only reference-data rows in Private Beta.",
                                descriptions.join("; ")
                            ));
                        }
                    }
                }
                table_result.foreign_key_references = foreign_keys;
            }
            Err(error) => table_result.warnings.push(error),
        }
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

pub(crate) fn parse_reference_data_registry(
    content: &str,
) -> Result<ReferenceDataRegistry, String> {
    let value: Value = serde_yaml::from_str(content)
        .map_err(|error| format!("Invalid reference-data registry YAML: {error}"))?;
    let mapping = expect_mapping(&value, "reference-data registry")?;
    let version = optional_i64(mapping, "version", "reference-data registry")?.unwrap_or(1);
    let tables_value = required_value(mapping, "tables", "reference-data registry")?;
    let table_values = match tables_value {
        Value::Sequence(values) => values,
        _ => return Err("reference-data registry tables must be a list.".to_string()),
    };

    let mut tables = Vec::new();
    for table_value in table_values {
        let table_mapping = expect_mapping(table_value, "reference-data registry table")?;
        let name = reference_data_table_name(table_mapping)?;
        validate_schema_qualified_name(&name)?;
        let file = optional_string(table_mapping, "file", "reference-data registry table")?
            .unwrap_or_else(|| format!("tables/{name}.yml"));
        ensure_reference_data_relative_path(&file)?;
        let key_columns = required_alias_string_list(
            table_mapping,
            "key",
            "keyColumns",
            "reference-data registry table",
        )?;
        if key_columns.is_empty() {
            return Err(format!(
                "Reference-data table '{name}' must define at least one key column."
            ));
        }
        let ignore_columns = optional_alias_string_list(
            table_mapping,
            "ignoreColumns",
            "ignoredColumns",
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

pub(crate) fn parse_reference_data_table_state(
    content: &str,
    config: &ReferenceDataTableConfig,
) -> Result<ReferenceDataTableState, String> {
    let value: Value = serde_yaml::from_str(content)
        .map_err(|error| format!("Invalid reference-data table YAML: {error}"))?;
    let file_label = reference_data_table_file_label(config);
    let mapping = expect_mapping(&value, &file_label)?;
    let table_name = required_string(mapping, "table", &file_label).map_err(|error| {
        format!(
            "{file_label}: {error}. Expected top-level field 'table: {}'.",
            config.name
        )
    })?;
    if table_name != config.name {
        return Err(format!(
            "{file_label} declares table '{table_name}' but registry expects '{}'.",
            config.name
        ));
    }
    let key_columns = optional_reference_table_key_columns(mapping, config)?;
    if key_columns != config.key_columns {
        return Err(format!(
            "{file_label} key for '{}' does not match registry key.",
            config.name
        ));
    }
    let rows_value = required_value(mapping, "rows", &file_label)?;
    let row_values = match rows_value {
        Value::Sequence(values) => values,
        _ => return Err(format!("{file_label}: rows must be a list.")),
    };

    let mut rows = Vec::new();
    let mut keys = BTreeSet::new();
    for (row_index, row_value) in row_values.iter().enumerate() {
        let row_number = row_index + 1;
        let values = reference_table_row_values(row_value, config, &file_label, row_number)?;
        let row = ReferenceDataRow { values };
        let row_key = reference_row_key(&row, &config.key_columns)
            .map_err(|error| format!("{file_label} row {row_number}: {error}"))?;
        if !keys.insert(row_key.clone()) {
            return Err(format!(
                "{file_label} row {row_number}: Reference-data table '{}' has duplicate row key '{}'.",
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

fn reference_data_table_file_label(config: &ReferenceDataTableConfig) -> String {
    format!("database/reference-data/{}", config.file)
}

fn optional_reference_table_key_columns(
    mapping: &Mapping,
    config: &ReferenceDataTableConfig,
) -> Result<Vec<String>, String> {
    if mapping.get(Value::String("key".to_string())).is_some() {
        return required_string_list(mapping, "key", &reference_data_table_file_label(config));
    }
    if mapping
        .get(Value::String("keyColumns".to_string()))
        .is_some()
    {
        return required_string_list(
            mapping,
            "keyColumns",
            &reference_data_table_file_label(config),
        );
    }
    Ok(config.key_columns.clone())
}

fn reference_table_row_values(
    row_value: &Value,
    config: &ReferenceDataTableConfig,
    file_label: &str,
    row_number: usize,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let row_mapping = expect_mapping(row_value, "reference-data row").map_err(|error| {
        format!("{file_label} row {row_number}: {error}. Expected row format: key + values.")
    })?;
    if row_mapping
        .get(Value::String("values".to_string()))
        .is_some()
    {
        return canonical_reference_row_values(row_mapping, config, file_label, row_number);
    }
    if row_mapping.get(Value::String("key".to_string())).is_some()
        && !config.key_columns.iter().any(|column| column == "key")
    {
        return Err(format!(
            "{file_label} row {row_number} is missing required field 'values'. Expected row format: key + values."
        ));
    }
    flat_reference_row_values(row_mapping, file_label, row_number)
}

fn canonical_reference_row_values(
    row_mapping: &Mapping,
    config: &ReferenceDataTableConfig,
    file_label: &str,
    row_number: usize,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let Some(key_value) = row_mapping.get(Value::String("key".to_string())) else {
        return Err(format!(
            "{file_label} row {row_number} is missing required field 'key'. Expected row format: key + values."
        ));
    };
    let key_values = reference_row_key_values(key_value, config, file_label, row_number)?;
    let values_value = row_mapping
        .get(Value::String("values".to_string()))
        .expect("values field exists");
    let values_mapping = expect_mapping(values_value, "reference-data row values").map_err(|error| {
        format!("{file_label} row {row_number}: {error}. Expected 'values' to be a mapping of column names to scalar values.")
    })?;
    let mut values = flat_reference_row_values(values_mapping, file_label, row_number)?;
    for (column, key_value) in key_values {
        match values.get(&column) {
            Some(existing) if existing != &key_value => {
                return Err(format!(
                    "{file_label} row {row_number}: key column '{column}' value does not match values.{column}."
                ))
            }
            Some(_) => {}
            None => {
                values.insert(column, key_value);
            }
        }
    }
    Ok(values)
}

fn reference_row_key_values(
    key_value: &Value,
    config: &ReferenceDataTableConfig,
    file_label: &str,
    row_number: usize,
) -> Result<BTreeMap<String, Option<String>>, String> {
    match key_value {
        Value::Mapping(mapping) => {
            let values = flat_reference_row_values(mapping, file_label, row_number)?;
            for key_column in &config.key_columns {
                if !values.contains_key(key_column) {
                    return Err(format!(
                        "{file_label} row {row_number}: key is missing configured key column '{key_column}'. Expected key mapping to include all registry keyColumns."
                    ));
                }
            }
            Ok(values)
        }
        _ if config.key_columns.len() == 1 => {
            let mut values = BTreeMap::new();
            values.insert(
                config.key_columns[0].clone(),
                yaml_value_to_reference_string(key_value).map_err(|error| {
                    format!(
                        "{file_label} row {row_number}: {error}. Expected scalar key value."
                    )
                })?,
            );
            Ok(values)
        }
        _ => Err(format!(
            "{file_label} row {row_number}: scalar key is supported only for a single key column. Expected key mapping with all registry keyColumns."
        )),
    }
}

fn flat_reference_row_values(
    row_mapping: &Mapping,
    file_label: &str,
    row_number: usize,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let mut values = BTreeMap::new();
    for (key, value) in row_mapping {
        let Some(column) = key.as_str() else {
            return Err(format!(
                "{file_label} row {row_number}: Reference-data row column names must be strings."
            ));
        };
        values.insert(
            column.to_string(),
            yaml_value_to_reference_string(value)
                .map_err(|error| format!("{file_label} row {row_number}: {error}"))?,
        );
    }
    Ok(values)
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

fn read_reference_data_foreign_key_references(
    client: &mut Client,
    config: &ReferenceDataTableConfig,
) -> Result<Vec<ReferenceDataForeignKeyReference>, String> {
    let (schema, table) = split_schema_qualified_name(&config.name)?;
    let rows = client
        .query(
            "SELECT con.conname,
                    src_ns.nspname AS referencing_schema,
                    src_rel.relname AS referencing_table,
                    array_agg(src_att.attname ORDER BY key_ord.ordinality) AS referencing_columns,
                    tgt_ns.nspname AS referenced_schema,
                    tgt_rel.relname AS referenced_table,
                    array_agg(tgt_att.attname ORDER BY key_ord.ordinality) AS referenced_columns
             FROM pg_constraint con
             JOIN pg_class src_rel ON src_rel.oid = con.conrelid
             JOIN pg_namespace src_ns ON src_ns.oid = src_rel.relnamespace
             JOIN pg_class tgt_rel ON tgt_rel.oid = con.confrelid
             JOIN pg_namespace tgt_ns ON tgt_ns.oid = tgt_rel.relnamespace
             JOIN unnest(con.conkey, con.confkey) WITH ORDINALITY AS key_ord(src_attnum, tgt_attnum, ordinality) ON true
             JOIN pg_attribute src_att ON src_att.attrelid = con.conrelid AND src_att.attnum = key_ord.src_attnum
             JOIN pg_attribute tgt_att ON tgt_att.attrelid = con.confrelid AND tgt_att.attnum = key_ord.tgt_attnum
             WHERE con.contype = 'f'
               AND tgt_ns.nspname = $1
               AND tgt_rel.relname = $2
             GROUP BY con.conname, src_ns.nspname, src_rel.relname, tgt_ns.nspname, tgt_rel.relname
             ORDER BY src_ns.nspname, src_rel.relname, con.conname",
            &[&schema, &table],
        )
        .map_err(|_| {
            format!(
                "PostgreSQL foreign-key metadata query failed for reference-data table '{}'.",
                config.name
            )
        })?;
    Ok(rows
        .into_iter()
        .map(|row| ReferenceDataForeignKeyReference {
            constraint_name: row.get::<_, String>(0),
            referencing_schema: row.get::<_, String>(1),
            referencing_table: row.get::<_, String>(2),
            referencing_columns: row.get::<_, Vec<String>>(3),
            referenced_schema: row.get::<_, String>(4),
            referenced_table: row.get::<_, String>(5),
            referenced_columns: row.get::<_, Vec<String>>(6),
        })
        .collect())
}

fn reference_data_foreign_key_description(value: &ReferenceDataForeignKeyReference) -> String {
    format!(
        "{}.{}.{} -> {}.{}.{}",
        value.referencing_schema,
        value.referencing_table,
        value.referencing_columns.join(","),
        value.referenced_schema,
        value.referenced_table,
        value.referenced_columns.join(",")
    )
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
        foreign_key_references: Vec::new(),
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
                    push_reference_row_result(
                        &mut result,
                        config,
                        &key,
                        "inSync",
                        Some(repo_row),
                        Some(database_row),
                        Vec::new(),
                    );
                } else {
                    push_reference_row_result(
                        &mut result,
                        config,
                        &key,
                        "repoDifferent",
                        Some(repo_row),
                        Some(database_row),
                        changed_columns,
                    );
                }
            }
            (Some(repo_row), None) => {
                push_reference_row_result(
                    &mut result,
                    config,
                    &key,
                    "repoOnly",
                    Some(repo_row),
                    None,
                    Vec::new(),
                );
            }
            (None, Some(database_row)) => {
                push_reference_row_result(
                    &mut result,
                    config,
                    &key,
                    "databaseOnly",
                    None,
                    Some(database_row),
                    Vec::new(),
                );
            }
            (None, None) => {}
        }
    }
    result
}

fn push_reference_row_result(
    table_result: &mut ReferenceTableCompareResult,
    config: &ReferenceDataTableConfig,
    row_key: &str,
    classification: &str,
    repository_row: Option<&ReferenceDataRow>,
    database_row: Option<&ReferenceDataRow>,
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
        key_values: safe_reference_key_values(repository_row.or(database_row), &config.key_columns),
        repository_values: safe_reference_row_values(repository_row, config),
        database_values: safe_reference_row_values(database_row, config),
        changed_columns,
        ignored_columns: table_result.ignored_columns.clone(),
        masked_columns: table_result.masked_columns.clone(),
        warnings: Vec::new(),
    });
}

fn safe_reference_key_values(
    row: Option<&ReferenceDataRow>,
    key_columns: &[String],
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let Some(row) = row else {
        return values;
    };
    for column in key_columns {
        let value = row
            .values
            .get(column)
            .and_then(|value| value.as_ref())
            .map_or_else(|| "null".to_string(), |value| value.to_string());
        values.insert(column.clone(), value);
    }
    values
}

fn safe_reference_row_values(
    row: Option<&ReferenceDataRow>,
    config: &ReferenceDataTableConfig,
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let columns: BTreeSet<String> = config
        .key_columns
        .iter()
        .chain(config.ignore_columns.iter())
        .chain(config.masked_columns.iter())
        .cloned()
        .chain(
            row.into_iter()
                .flat_map(|row| row.values.keys().cloned())
                .collect::<Vec<_>>(),
        )
        .collect();
    for column in columns {
        if config.masked_columns.contains(&column) {
            values.insert(column, "[masked]".to_string());
            continue;
        }
        if config.ignore_columns.contains(&column) {
            values.insert(column, "[ignored]".to_string());
            continue;
        }
        let value = row
            .and_then(|row| row.values.get(&column))
            .and_then(|value| value.as_ref())
            .map_or_else(|| "null".to_string(), |value| value.to_string());
        values.insert(column, value);
    }
    values
}

pub(crate) fn append_reference_table_result(
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

pub fn reference_data_review_script_preview_with_connection(
    cwd: &Path,
    connection_url: &str,
    selection: &ReferenceDataSelection,
    script_name: &str,
) -> Result<ReferenceDataReviewScriptReport, String> {
    reference_data_review_script_with_connection(cwd, connection_url, selection, script_name, true)
}

pub fn reference_data_review_script_write_with_connection(
    cwd: &Path,
    connection_url: &str,
    selection: &ReferenceDataSelection,
    script_name: &str,
) -> Result<ReferenceDataReviewScriptReport, String> {
    reference_data_review_script_with_connection(cwd, connection_url, selection, script_name, false)
}

#[cfg(test)]
pub(crate) fn reference_data_review_script_preview_from_compare(
    compare: &ReferenceDataCompareReport,
    script_name: &str,
) -> Result<ReferenceDataReviewScriptReport, String> {
    let slug = reference_data_review_script_slug(script_name)?;
    let artifacts = ReferenceDataReviewArtifactPaths {
        sql: format!("database/releases/reference-data/0001_{slug}.reference-data.sql"),
        summary: format!("database/releases/reference-data/0001_{slug}.reference-data.summary.md"),
        risk: format!("database/releases/reference-data/0001_{slug}.reference-data.risk.json"),
        manifest: format!(
            "database/releases/reference-data/0001_{slug}.reference-data.manifest.json"
        ),
    };
    let mut report = empty_reference_data_review_script_report(true);
    report.success = true;
    report.repository_path = compare.repository_path.clone();
    report.git_root = compare.git_root.clone();
    report.is_git_repository = compare.is_git_repository;
    report.branch = compare.branch.clone();
    report.working_tree_status = compare.working_tree_status;
    report.is_dirty = compare.is_dirty;
    report.script_name = script_name.to_string();
    report.selected_tables = compare.selected_tables.clone();
    report.planned_artifacts = artifacts.relative_paths();
    report.warnings.extend(compare.warnings.clone());
    report.errors.extend(compare.errors.clone());
    render_reference_data_review_artifacts(&mut report, compare, &artifacts);
    report.success = report.errors.is_empty();
    Ok(report)
}

#[cfg(test)]
pub(crate) fn reference_data_review_script_write_from_compare(
    root: &Path,
    compare: &ReferenceDataCompareReport,
    script_name: &str,
) -> Result<ReferenceDataReviewScriptReport, String> {
    let slug = reference_data_review_script_slug(script_name)?;
    fs::create_dir_all(root.join("database/releases/reference-data"))
        .map_err(|error| format!("Could not create reference-data artifact folder: {error}"))?;
    let artifacts = plan_reference_data_review_artifact_paths(root, &slug)?;
    let mut report = empty_reference_data_review_script_report(false);
    report.success = true;
    report.repository_path = compare.repository_path.clone();
    report.git_root = compare.git_root.clone();
    report.is_git_repository = compare.is_git_repository;
    report.branch = compare.branch.clone();
    report.working_tree_status = compare.working_tree_status;
    report.is_dirty = compare.is_dirty;
    report.script_name = script_name.to_string();
    report.selected_tables = compare.selected_tables.clone();
    report.planned_artifacts = artifacts.relative_paths();
    report.warnings.extend(compare.warnings.clone());
    report.errors.extend(compare.errors.clone());
    render_reference_data_review_artifacts(&mut report, compare, &artifacts);
    if !report.errors.is_empty() {
        report.success = false;
        return Ok(report);
    }
    let generated = [
        (&artifacts.sql, report.script_content.clone()),
        (&artifacts.summary, report.summary_content.clone()),
        (&artifacts.risk, report.risk_content.clone()),
        (&artifacts.manifest, report.manifest_content.clone()),
    ];
    for (relative_path, _) in &generated {
        ensure_reference_data_review_release_path(relative_path)?;
        if root.join(relative_path).exists() {
            report.errors.push(format!(
                "Refusing to overwrite existing reference-data review artifact {relative_path}."
            ));
        }
    }
    if report.errors.is_empty() {
        for (relative_path, content) in generated {
            fs::write(root.join(relative_path), content)
                .map_err(|error| format!("Could not write {relative_path}: {error}"))?;
            report.created_artifacts.push(relative_path.clone());
        }
    }
    report.success = report.errors.is_empty();
    Ok(report)
}

fn reference_data_review_script_with_connection(
    cwd: &Path,
    connection_url: &str,
    selection: &ReferenceDataSelection,
    script_name: &str,
    dry_run: bool,
) -> Result<ReferenceDataReviewScriptReport, String> {
    let mut report = empty_reference_data_review_script_report(dry_run);
    report.script_name = script_name.to_string();
    let slug = match reference_data_review_script_slug(script_name) {
        Ok(slug) => slug,
        Err(error) => {
            report.errors.push(error);
            return Ok(report);
        }
    };

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
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure
        && !project_structure_allows_release_subfolder_backfill(&project)
    {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return Ok(report);
    }
    if project_structure_allows_release_subfolder_backfill(&project) {
        report.warnings.push(
            "Project is missing release artifact subfolders. database/releases/reference-data will be created during review script generation."
                .to_string(),
        );
    }
    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    if !root.join("database/releases").is_dir() {
        report.errors.push(
            "database/releases is missing. Run dbstate init before generating review script artifacts."
                .to_string(),
        );
        return Ok(report);
    }
    let artifacts = match plan_reference_data_review_artifact_paths(&root, &slug) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            report.errors.push(error);
            return Ok(report);
        }
    };
    report.planned_artifacts = artifacts.relative_paths();

    if !dry_run {
        let guard = scoped_write_guard(
            &root,
            report.branch.as_deref(),
            project.default_branch.as_deref(),
            &report.planned_artifacts,
            GitHandoffWorkflow::ReferenceDataReview,
            script_name,
        );
        report.warnings.extend(guard.warnings);
        if !guard.allowed {
            report.errors.extend(guard.errors);
            report.success = false;
            return Ok(report);
        }
    }

    let compare = data_compare_postgres_with_connection(cwd, connection_url, selection)?;
    report.selected_tables = compare.selected_tables.clone();
    report.warnings.extend(compare.warnings.clone());
    report.errors.extend(compare.errors.clone());
    if !report.errors.is_empty() {
        report.success = false;
        return Ok(report);
    }

    render_reference_data_review_artifacts(&mut report, &compare, &artifacts);
    report.success = report.errors.is_empty();
    if dry_run || !report.success {
        return Ok(report);
    }

    if let Err(error) = fs::create_dir_all(root.join("database/releases/reference-data")) {
        report.errors.push(format!(
            "Could not create database/releases/reference-data for review script artifacts: {error}"
        ));
        report.success = false;
        return Ok(report);
    }

    let generated = [
        (&artifacts.sql, report.script_content.clone()),
        (&artifacts.summary, report.summary_content.clone()),
        (&artifacts.risk, report.risk_content.clone()),
        (&artifacts.manifest, report.manifest_content.clone()),
    ];
    for (relative_path, _) in &generated {
        if let Err(error) = ensure_reference_data_review_release_path(relative_path) {
            report.errors.push(error);
        }
        if root.join(relative_path).exists() {
            report.errors.push(format!(
                "Refusing to overwrite existing reference-data review artifact {relative_path}."
            ));
        }
    }
    if !report.errors.is_empty() {
        report.success = false;
        return Ok(report);
    }
    for (relative_path, content) in generated {
        if let Err(error) = fs::write(root.join(relative_path), content) {
            report
                .errors
                .push(format!("Could not write {relative_path}: {error}"));
            continue;
        }
        report.created_artifacts.push(relative_path.clone());
    }
    report.success = report.errors.is_empty();
    Ok(report)
}

#[derive(Debug, Clone)]
struct ReferenceDataReviewArtifactPaths {
    sql: String,
    summary: String,
    risk: String,
    manifest: String,
}

impl ReferenceDataReviewArtifactPaths {
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

fn reference_data_review_script_slug(script_name: &str) -> Result<String, String> {
    let trimmed = script_name.trim();
    if trimmed.is_empty() {
        return Err("Review script name cannot be empty.".to_string());
    }
    if trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
        || trimmed.contains(' ')
        || trimmed.contains('\t')
    {
        return Err(
            "Review script name must use only letters, numbers, hyphen, or underscore.".to_string(),
        );
    }
    let slug = trimmed.to_ascii_lowercase();
    if slug
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        Ok(slug)
    } else {
        Err("Review script name must use only letters, numbers, hyphen, or underscore.".to_string())
    }
}

fn plan_reference_data_review_artifact_paths(
    root: &Path,
    slug: &str,
) -> Result<ReferenceDataReviewArtifactPaths, String> {
    for sequence in 1..=9999 {
        let prefix = format!("{sequence:04}_{slug}.reference-data");
        let artifacts = ReferenceDataReviewArtifactPaths {
            sql: format!("database/releases/reference-data/{prefix}.sql"),
            summary: format!("database/releases/reference-data/{prefix}.summary.md"),
            risk: format!("database/releases/reference-data/{prefix}.risk.json"),
            manifest: format!("database/releases/reference-data/{prefix}.manifest.json"),
        };
        for relative_path in artifacts.relative_paths() {
            ensure_reference_data_review_release_path(&relative_path)?;
        }
        if !root.join(&artifacts.sql).exists()
            && !root.join(&artifacts.summary).exists()
            && !root.join(&artifacts.risk).exists()
            && !root.join(&artifacts.manifest).exists()
        {
            return Ok(artifacts);
        }
    }
    Err("Could not choose a reference-data review artifact sequence from 0001 to 9999.".to_string())
}

fn ensure_reference_data_review_release_path(relative_path: &str) -> Result<(), String> {
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

fn empty_reference_data_review_script_report(dry_run: bool) -> ReferenceDataReviewScriptReport {
    ReferenceDataReviewScriptReport {
        command: if dry_run {
            "reference-data review-script preview".to_string()
        } else {
            "reference-data review-script write".to_string()
        },
        success: false,
        dry_run,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        script_name: String::new(),
        selected_tables: Vec::new(),
        affected_rows: 0,
        insert_count: 0,
        update_count: 0,
        manual_review_count: 0,
        database_only_count: 0,
        foreign_key_warning_count: 0,
        delete_generated_count: 0,
        planned_artifacts: Vec::new(),
        created_artifacts: Vec::new(),
        script_content: String::new(),
        summary_content: String::new(),
        risk_content: String::new(),
        manifest_content: String::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    }
}

fn render_reference_data_review_artifacts(
    report: &mut ReferenceDataReviewScriptReport,
    compare: &ReferenceDataCompareReport,
    artifacts: &ReferenceDataReviewArtifactPaths,
) {
    let sql = render_reference_data_review_sql(report, compare);
    report.script_content = sql;
    report.summary_content = render_reference_data_review_summary(report, artifacts);
    report.risk_content = render_reference_data_review_risk_json(report, artifacts);
    report.manifest_content = render_reference_data_review_manifest_json(report, artifacts);
}

fn render_reference_data_review_summary(
    report: &ReferenceDataReviewScriptReport,
    artifacts: &ReferenceDataReviewArtifactPaths,
) -> String {
    let mut summary = String::new();
    writeln!(summary, "# Reference Data Review Script").ok();
    writeln!(summary).ok();
    writeln!(summary, "- Review only: true").ok();
    writeln!(summary, "- Executes SQL: false").ok();
    writeln!(summary, "- Mutates PostgreSQL: false").ok();
    writeln!(summary, "- Source: Repository reference-data").ok();
    writeln!(summary, "- Target: PostgreSQL database").ok();
    writeln!(
        summary,
        "- Selected tables: {}",
        if report.selected_tables.is_empty() {
            "none".to_string()
        } else {
            report.selected_tables.join(", ")
        }
    )
    .ok();
    writeln!(summary, "- INSERT candidates: {}", report.insert_count).ok();
    writeln!(summary, "- UPDATE candidates: {}", report.update_count).ok();
    writeln!(
        summary,
        "- Manual-review rows: {}",
        report.manual_review_count
    )
    .ok();
    writeln!(
        summary,
        "- Database-only rows: {}",
        report.database_only_count
    )
    .ok();
    writeln!(
        summary,
        "- DELETE statements generated: {}",
        report.delete_generated_count
    )
    .ok();
    writeln!(
        summary,
        "- Potential foreign-key warnings: {}",
        report.foreign_key_warning_count
    )
    .ok();
    writeln!(summary).ok();
    writeln!(
        summary,
        "DbState does not execute this SQL. Review manually before applying outside DbState."
    )
    .ok();
    writeln!(
        summary,
        "DbState does not generate DELETE for database-only reference-data rows in Private Beta."
    )
    .ok();
    writeln!(summary).ok();
    writeln!(summary, "## Artifacts").ok();
    for path in artifacts.relative_paths() {
        writeln!(summary, "- {path}").ok();
    }
    summary
}

fn render_reference_data_review_risk_json(
    report: &ReferenceDataReviewScriptReport,
    artifacts: &ReferenceDataReviewArtifactPaths,
) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_bool_field(&mut json, "reviewOnly", true);
    write_json_bool_field(&mut json, "executesSql", false);
    write_json_bool_field(&mut json, "mutatesPostgres", false);
    write_json_bool_field(&mut json, "deleteGenerated", false);
    write_json_usize_field(&mut json, "insertCandidates", report.insert_count);
    write_json_usize_field(&mut json, "updateCandidates", report.update_count);
    write_json_usize_field(&mut json, "manualReviewRows", report.manual_review_count);
    write_json_usize_field(&mut json, "databaseOnlyRows", report.database_only_count);
    write_json_usize_field(
        &mut json,
        "foreignKeyWarnings",
        report.foreign_key_warning_count,
    );
    write_json_usize_field(
        &mut json,
        "deleteGeneratedCount",
        report.delete_generated_count,
    );
    write_json_array_field(&mut json, "selectedTables", &report.selected_tables);
    write_json_array_field(&mut json, "artifactPaths", &artifacts.relative_paths());
    write_json_array_field(&mut json, "warnings", &report.warnings);
    json.push('}');
    json
}

fn render_reference_data_review_manifest_json(
    report: &ReferenceDataReviewScriptReport,
    artifacts: &ReferenceDataReviewArtifactPaths,
) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "artifactKind", "referenceDataReviewScript", true);
    write_json_string_field(&mut json, "source", "repositoryReferenceData", false);
    write_json_string_field(&mut json, "target", "postgresDatabase", false);
    write_json_string_field(&mut json, "sequence", &artifacts.sequence(), false);
    write_json_string_field(&mut json, "scriptName", &report.script_name, false);
    write_json_bool_field(&mut json, "reviewOnly", true);
    write_json_bool_field(&mut json, "executesSql", false);
    write_json_bool_field(&mut json, "mutatesPostgres", false);
    write_json_array_field(&mut json, "selectedTables", &report.selected_tables);
    write_json_array_field(&mut json, "paths", &artifacts.relative_paths());
    json.push('}');
    json
}

fn render_reference_data_review_sql(
    report: &mut ReferenceDataReviewScriptReport,
    compare: &ReferenceDataCompareReport,
) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- REVIEW ONLY.").ok();
    writeln!(sql, "-- DbState does not execute this SQL.").ok();
    writeln!(sql, "-- Review manually before applying outside DbState.").ok();
    writeln!(sql, "-- Source: Repository reference-data").ok();
    writeln!(sql, "-- Target: PostgreSQL database").ok();
    writeln!(
        sql,
        "-- DbState does not generate DELETE for database-only reference-data rows in Private Beta."
    )
    .ok();
    writeln!(sql).ok();

    for table in &compare.table_results {
        writeln!(sql, "-- Reference table: {}", table.table_name).ok();
        writeln!(sql, "-- Key columns: {}", table.key_columns.join(", ")).ok();
        writeln!(sql).ok();
        for row in &table.row_results {
            match row.classification.as_str() {
                "repoOnly" => render_reference_data_insert_candidate(&mut sql, report, table, row),
                "repoDifferent" => {
                    render_reference_data_update_candidate(&mut sql, report, table, row)
                }
                "databaseOnly" => {
                    render_reference_data_database_only_comment(&mut sql, report, table, row)
                }
                "skipped" | "error" => {
                    report.manual_review_count += 1;
                    report.affected_rows += 1;
                    writeln!(sql, "-- REVIEW REQUIRED: skipped/error reference-data row.").ok();
                    writeln!(sql, "-- Row: {}", row.row_key).ok();
                    writeln!(sql, "-- No INSERT, UPDATE, or DELETE generated.").ok();
                    write_reference_row_warnings(&mut sql, row);
                    writeln!(sql).ok();
                }
                _ => {}
            }
        }
        writeln!(sql).ok();
    }
    if report.insert_count == 0 && report.update_count == 0 && report.manual_review_count == 0 {
        writeln!(
            sql,
            "-- No non-in-sync reference-data rows selected for review script generation."
        )
        .ok();
    }
    sql
}

fn render_reference_data_insert_candidate(
    sql: &mut String,
    report: &mut ReferenceDataReviewScriptReport,
    table: &ReferenceTableCompareResult,
    row: &ReferenceRowCompareResult,
) {
    let columns = reference_data_insert_columns(table, row);
    if columns.is_empty() {
        report.manual_review_count += 1;
        report.affected_rows += 1;
        writeln!(sql, "-- REVIEW REQUIRED: repository-only reference-data row has no safely renderable columns.").ok();
        writeln!(sql, "-- Row: {}", row.row_key).ok();
        writeln!(sql, "-- No INSERT generated.").ok();
        writeln!(sql).ok();
        return;
    }
    let Some((schema, table_name)) = table.table_name.split_once('.') else {
        report.errors.push(format!(
            "Reference-data table '{}' is not schema-qualified.",
            table.table_name
        ));
        return;
    };
    let values: Vec<String> = columns
        .iter()
        .map(|column| sql_literal(row.repository_values.get(column)))
        .collect();
    writeln!(sql, "-- Row: {}", reference_row_comment_key(row)).ok();
    writeln!(sql, "-- Classification: Repository only").ok();
    writeln!(
        sql,
        "INSERT INTO {}.{} ({})",
        quote_postgres_identifier(schema),
        quote_postgres_identifier(table_name),
        columns
            .iter()
            .map(|column| quote_postgres_identifier(column))
            .collect::<Vec<_>>()
            .join(", ")
    )
    .ok();
    writeln!(sql, "VALUES ({});", values.join(", ")).ok();
    writeln!(sql).ok();
    report.insert_count += 1;
    report.affected_rows += 1;
}

fn render_reference_data_update_candidate(
    sql: &mut String,
    report: &mut ReferenceDataReviewScriptReport,
    table: &ReferenceTableCompareResult,
    row: &ReferenceRowCompareResult,
) {
    let columns = reference_data_update_columns(table, row);
    if columns.is_empty() {
        report.manual_review_count += 1;
        report.affected_rows += 1;
        writeln!(sql, "-- REVIEW REQUIRED: different reference-data row has no safely renderable changed versioned columns.").ok();
        writeln!(sql, "-- Row: {}", row.row_key).ok();
        writeln!(sql, "-- No UPDATE generated.").ok();
        writeln!(sql).ok();
        return;
    }
    let Some((schema, table_name)) = table.table_name.split_once('.') else {
        report.errors.push(format!(
            "Reference-data table '{}' is not schema-qualified.",
            table.table_name
        ));
        return;
    };
    writeln!(sql, "-- Row: {}", reference_row_comment_key(row)).ok();
    writeln!(sql, "-- Classification: Different").ok();
    writeln!(
        sql,
        "UPDATE {}.{}",
        quote_postgres_identifier(schema),
        quote_postgres_identifier(table_name)
    )
    .ok();
    writeln!(
        sql,
        "SET {}",
        columns
            .iter()
            .map(|column| format!(
                "{} = {}",
                quote_postgres_identifier(column),
                sql_literal(row.repository_values.get(column))
            ))
            .collect::<Vec<_>>()
            .join(", ")
    )
    .ok();
    writeln!(sql, "WHERE {};", reference_data_where_clause(table, row)).ok();
    writeln!(sql).ok();
    report.update_count += 1;
    report.affected_rows += 1;
}

fn render_reference_data_database_only_comment(
    sql: &mut String,
    report: &mut ReferenceDataReviewScriptReport,
    table: &ReferenceTableCompareResult,
    row: &ReferenceRowCompareResult,
) {
    report.manual_review_count += 1;
    report.database_only_count += 1;
    report.affected_rows += 1;
    writeln!(sql, "-- REVIEW REQUIRED: database-only reference-data row.").ok();
    writeln!(
        sql,
        "-- DbState does not generate DELETE for reference-data rows in Private Beta."
    )
    .ok();
    writeln!(
        sql,
        "-- Reason: deleting reference data can break foreign keys or historical meaning."
    )
    .ok();
    writeln!(sql, "-- Key: {}", reference_row_comment_key(row)).ok();
    writeln!(sql, "-- Potentially affected foreign keys:").ok();
    if table.foreign_key_references.is_empty() {
        writeln!(sql, "--   none detected by read-only metadata query").ok();
    } else {
        report.foreign_key_warning_count += table.foreign_key_references.len();
        for foreign_key in &table.foreign_key_references {
            writeln!(
                sql,
                "--   {}",
                reference_data_foreign_key_description(foreign_key)
            )
            .ok();
        }
    }
    writeln!(sql, "-- No DELETE generated.").ok();
    write_reference_row_warnings(sql, row);
    writeln!(sql).ok();
}

fn write_reference_row_warnings(sql: &mut String, row: &ReferenceRowCompareResult) {
    for warning in &row.warnings {
        writeln!(sql, "-- Warning: {warning}").ok();
    }
}

fn reference_data_insert_columns(
    table: &ReferenceTableCompareResult,
    row: &ReferenceRowCompareResult,
) -> Vec<String> {
    let mut columns = Vec::new();
    for column in &table.key_columns {
        if row.repository_values.contains_key(column) && !columns.contains(column) {
            columns.push(column.clone());
        }
    }
    for column in row.repository_values.keys() {
        if table.ignored_columns.contains(column)
            || table.masked_columns.contains(column)
            || columns.contains(column)
        {
            continue;
        }
        columns.push(column.clone());
    }
    columns
}

fn reference_data_update_columns(
    table: &ReferenceTableCompareResult,
    row: &ReferenceRowCompareResult,
) -> Vec<String> {
    row.changed_columns
        .iter()
        .filter(|column| {
            !table.key_columns.contains(*column)
                && !table.ignored_columns.contains(*column)
                && !table.masked_columns.contains(*column)
                && row.repository_values.contains_key(*column)
        })
        .cloned()
        .collect()
}

fn reference_data_where_clause(
    table: &ReferenceTableCompareResult,
    row: &ReferenceRowCompareResult,
) -> String {
    table
        .key_columns
        .iter()
        .map(|column| {
            format!(
                "{} = {}",
                quote_postgres_identifier(column),
                sql_literal(row.key_values.get(column))
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn reference_row_comment_key(row: &ReferenceRowCompareResult) -> String {
    if row.key_values.is_empty() {
        return row.row_key.clone();
    }
    row.key_values
        .iter()
        .map(|(key, value)| format!("{key} = {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn sql_literal(value: Option<&String>) -> String {
    let Some(value) = value else {
        return "NULL".to_string();
    };
    if value == "[masked]" || value == "[ignored]" {
        return "NULL".to_string();
    }
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("null") {
        return "NULL".to_string();
    }
    if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
        return trimmed.to_ascii_lowercase();
    }
    if is_safe_sql_number(trimmed) {
        return trimmed.to_string();
    }
    format!("'{}'", trimmed.replace('\'', "''"))
}

fn is_safe_sql_number(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let mut chars = value.chars().peekable();
    if matches!(chars.peek(), Some('-')) {
        chars.next();
    }
    let mut saw_digit = false;
    let mut saw_dot = false;
    for character in chars {
        if character.is_ascii_digit() {
            saw_digit = true;
        } else if character == '.' && !saw_dot {
            saw_dot = true;
        } else {
            return false;
        }
    }
    saw_digit
}

fn write_json_usize_field(json: &mut String, name: &str, value: usize) {
    json.push(',');
    write!(json, "\"{}\":{}", escape_json(name), value).ok();
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

pub(crate) fn reference_row_key(
    row: &ReferenceDataRow,
    key_columns: &[String],
) -> Result<String, String> {
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

fn yaml_scalar(value: Option<&str>) -> String {
    match value {
        None => "null".to_string(),
        Some(value)
            if value.parse::<i64>().is_ok()
                || value.parse::<f64>().is_ok()
                || value == "true"
                || value == "false" =>
        {
            value.to_string()
        }
        Some(value) => yaml_plain_or_quoted(value),
    }
}

fn yaml_plain_or_quoted(value: &str) -> String {
    if value.is_empty()
        || value.starts_with('[')
        || value.contains(':')
        || value.contains('#')
        || value.contains('"')
        || value.contains('\'')
        || value.chars().any(char::is_whitespace)
        || matches!(value, "null" | "true" | "false" | "~")
    {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
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

fn optional_string(mapping: &Mapping, key: &str, context: &str) -> Result<Option<String>, String> {
    match mapping.get(Value::String(key.to_string())) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        Some(_) => Err(format!(
            "{context} field '{key}' must be a non-empty string."
        )),
        None => Ok(None),
    }
}

fn optional_i64(mapping: &Mapping, key: &str, context: &str) -> Result<Option<i64>, String> {
    match mapping.get(Value::String(key.to_string())) {
        Some(Value::Number(value)) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("{context} field '{key}' must be an integer.")),
        Some(_) => Err(format!("{context} field '{key}' must be an integer.")),
        None => Ok(None),
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

fn required_alias_string_list(
    mapping: &Mapping,
    primary_key: &str,
    alias_key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    if mapping
        .get(Value::String(primary_key.to_string()))
        .is_some()
    {
        return required_string_list(mapping, primary_key, context);
    }
    match mapping.get(Value::String(alias_key.to_string())) {
        Some(Value::Sequence(values)) => yaml_sequence_to_strings(values, alias_key, context),
        Some(_) => Err(format!(
            "{context} field '{alias_key}' must be a list of strings."
        )),
        None => Err(format!(
            "{context} is missing required field '{primary_key}'."
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

fn optional_alias_string_list(
    mapping: &Mapping,
    primary_key: &str,
    alias_key: &str,
    context: &str,
) -> Result<Vec<String>, String> {
    if mapping
        .get(Value::String(primary_key.to_string()))
        .is_some()
    {
        return optional_string_list(mapping, primary_key, context);
    }
    match mapping.get(Value::String(alias_key.to_string())) {
        Some(Value::Sequence(values)) => yaml_sequence_to_strings(values, alias_key, context),
        Some(_) => Err(format!(
            "{context} field '{alias_key}' must be a list of strings."
        )),
        None => Ok(Vec::new()),
    }
}

fn reference_data_table_name(mapping: &Mapping) -> Result<String, String> {
    let name = required_string(mapping, "name", "reference-data registry table")?;
    if name.contains('.') {
        return Ok(name);
    }
    let Some(schema) = optional_string(mapping, "schema", "reference-data registry table")? else {
        return Ok(name);
    };
    Ok(format!("{schema}.{name}"))
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

pub(crate) fn empty_reference_data_compare_report() -> ReferenceDataCompareReport {
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

impl ReferenceDataStatusReport {
    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "command", &self.command, true);
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
        write_json_string_field(
            &mut json,
            "repositoryPathUsed",
            &self.repository_path_used,
            false,
        );
        write_json_string_field(
            &mut json,
            "dbstateProjectStatus",
            self.dbstate_project_status.as_str(),
            false,
        );
        write_json_string_field(&mut json, "registryPath", &self.registry_path, false);
        write_json_bool_field(&mut json, "registryExists", self.registry_exists);
        write_json_string_field(&mut json, "status", &self.status, false);
        write_reference_data_configured_table_array_field(
            &mut json,
            "configuredTables",
            &self.configured_tables,
        );
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        json.push('}');
        json
    }
}

fn write_reference_data_configured_table_array_field(
    json: &mut String,
    name: &str,
    values: &[ReferenceDataConfiguredTableStatus],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schema", &value.schema, true);
        write_json_string_field(json, "name", &value.name, false);
        write_json_string_field(json, "tableName", &value.table_name, false);
        write_json_string_field(json, "file", &value.file, false);
        write_json_array_field(json, "keyColumns", &value.key_columns);
        write_json_array_field(json, "ignoredColumns", &value.ignored_columns);
        write_json_array_field(json, "maskedColumns", &value.masked_columns);
        json.push('}');
    }
    json.push(']');
}

impl ReferenceDataDatabaseTablesReport {
    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "command", &self.command, true);
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
        write_reference_data_candidate_table_array_field(&mut json, "tables", &self.tables);
        write_reference_data_configured_table_array_field(
            &mut json,
            "configuredTables",
            &self.configured_tables,
        );
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        json.push('}');
        json
    }
}

fn write_reference_data_candidate_table_array_field(
    json: &mut String,
    name: &str,
    values: &[ReferenceDataCandidateTable],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schema", &value.schema, true);
        write_json_string_field(json, "name", &value.name, false);
        write_json_string_field(json, "tableName", &value.table_name, false);
        write_reference_data_candidate_column_array_field(json, "columns", &value.columns);
        write_json_array_field(json, "suggestedKeyColumns", &value.suggested_key_columns);
        write_json_array_field(json, "warnings", &value.warnings);
        json.push('}');
    }
    json.push(']');
}

fn write_reference_data_candidate_column_array_field(
    json: &mut String,
    name: &str,
    values: &[ReferenceDataCandidateColumn],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "name", &value.name, true);
        write_json_string_field(json, "dataType", &value.data_type, false);
        write_json_bool_field(json, "nullable", value.nullable);
        write_json_bool_field(json, "isPrimaryKey", value.is_primary_key);
        write_json_bool_field(json, "isUnique", value.is_unique);
        write_json_i32_field(json, "ordinal", value.ordinal, false);
        json.push('}');
    }
    json.push(']');
}

impl ReferenceDataExportReport {
    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "command", &self.command, true);
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
        write_json_string_field(&mut json, "registryPath", &self.registry_path, false);
        write_json_string_field(&mut json, "registryYaml", &self.registry_yaml, false);
        write_reference_data_export_table_array_field(
            &mut json,
            "tablePreviews",
            &self.table_previews,
        );
        write_json_array_field(&mut json, "filesCreated", &self.files_created);
        write_json_array_field(&mut json, "filesUpdated", &self.files_updated);
        write_json_array_field(&mut json, "filesUnchanged", &self.files_unchanged);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        json.push('}');
        json
    }
}

fn write_reference_data_export_table_array_field(
    json: &mut String,
    name: &str,
    values: &[ReferenceDataTableExportPreview],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schema", &value.schema, true);
        write_json_string_field(json, "name", &value.name, false);
        write_json_string_field(json, "tableName", &value.table_name, false);
        write_json_string_field(json, "file", &value.file, false);
        write_json_string_field(json, "yaml", &value.yaml, false);
        write_json_array_field(json, "keyColumns", &value.key_columns);
        write_json_array_field(json, "versionedColumns", &value.versioned_columns);
        write_json_array_field(json, "ignoredColumns", &value.ignored_columns);
        write_json_array_field(json, "maskedColumns", &value.masked_columns);
        write!(json, ",\"rowCount\":{}", value.row_count).ok();
        write_json_array_field(json, "warnings", &value.warnings);
        json.push('}');
    }
    json.push(']');
}

impl ReferenceDataReviewScriptReport {
    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "command", self.command.as_str(), true);
        write_json_bool_field(&mut json, "success", self.success);
        write_json_bool_field(&mut json, "dryRun", self.dry_run);
        write_repository_context_fields(
            &mut json,
            &self.repository_path,
            self.git_root.as_deref(),
            self.is_git_repository,
            self.branch.as_deref(),
            self.working_tree_status,
            self.is_dirty,
        );
        write_json_string_field(&mut json, "scriptName", &self.script_name, false);
        write_json_array_field(&mut json, "selectedTables", &self.selected_tables);
        write_json_usize_field(&mut json, "affectedRows", self.affected_rows);
        write_json_usize_field(&mut json, "insertCount", self.insert_count);
        write_json_usize_field(&mut json, "updateCount", self.update_count);
        write_json_usize_field(&mut json, "manualReviewCount", self.manual_review_count);
        write_json_usize_field(&mut json, "databaseOnlyCount", self.database_only_count);
        write_json_usize_field(
            &mut json,
            "foreignKeyWarningCount",
            self.foreign_key_warning_count,
        );
        write_json_usize_field(
            &mut json,
            "deleteGeneratedCount",
            self.delete_generated_count,
        );
        write_json_array_field(&mut json, "plannedArtifacts", &self.planned_artifacts);
        write_json_array_field(&mut json, "createdArtifacts", &self.created_artifacts);
        write_json_array_field(&mut json, "filesCreated", &self.created_artifacts);
        write_json_string_field(&mut json, "scriptContent", &self.script_content, false);
        write_json_string_field(&mut json, "summaryContent", &self.summary_content, false);
        write_json_string_field(&mut json, "riskContent", &self.risk_content, false);
        write_json_string_field(&mut json, "manifestContent", &self.manifest_content, false);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
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
        write_reference_foreign_key_array_field(
            json,
            "foreignKeyReferences",
            &value.foreign_key_references,
        );
        write_reference_counts_field(json, "rowCounts", &value.row_counts);
        write_reference_row_result_array_field(json, "rowResults", &value.row_results);
        write_json_array_field(json, "warnings", &value.warnings);
        write_json_array_field(json, "errors", &value.errors);
        json.push('}');
    }
    json.push(']');
}

fn write_reference_foreign_key_array_field(
    json: &mut String,
    name: &str,
    values: &[ReferenceDataForeignKeyReference],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "constraintName", &value.constraint_name, true);
        write_json_string_field(json, "referencingSchema", &value.referencing_schema, false);
        write_json_string_field(json, "referencingTable", &value.referencing_table, false);
        write_json_array_field(json, "referencingColumns", &value.referencing_columns);
        write_json_string_field(json, "referencedSchema", &value.referenced_schema, false);
        write_json_string_field(json, "referencedTable", &value.referenced_table, false);
        write_json_array_field(json, "referencedColumns", &value.referenced_columns);
        write_json_string_field(
            json,
            "description",
            &reference_data_foreign_key_description(value),
            false,
        );
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
        write_json_object_string_field(json, "keyValues", &value.key_values);
        write_json_object_string_field(json, "repositoryValues", &value.repository_values);
        write_json_object_string_field(json, "databaseValues", &value.database_values);
        write_json_array_field(json, "changedColumns", &value.changed_columns);
        write_json_array_field(json, "ignoredColumns", &value.ignored_columns);
        write_json_array_field(json, "maskedColumns", &value.masked_columns);
        write_json_array_field(json, "warnings", &value.warnings);
        json.push('}');
    }
    json.push(']');
}

fn write_json_object_string_field(
    json: &mut String,
    name: &str,
    values: &BTreeMap<String, String>,
) {
    json.push(',');
    write!(json, "\"{}\":{{", escape_json(name)).ok();
    for (index, (key, value)) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(json, "\"{}\":\"{}\"", escape_json(key), escape_json(value)).ok();
    }
    json.push('}');
}
