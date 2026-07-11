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

pub(crate) fn parse_reference_data_table_state(
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
