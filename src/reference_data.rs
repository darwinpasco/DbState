use crate::postgres::{
    invalid_postgres_url_message, is_postgres_connection_url, quote_postgres_identifier,
    resolve_postgres_url,
};
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
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Initialize the repository before exporting reference data."
                .to_string(),
        );
        return Ok(report);
    }
    if write_files && project.is_dirty {
        report.errors.push(
            "Working tree has uncommitted changes. Reference-data file write requires a clean working tree."
                .to_string(),
        );
        return Ok(report);
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
