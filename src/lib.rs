use std::fmt::Write as _;
use std::path::Path;

const DEFERRED_OBJECT_TYPES: &[&str] = &["rlsPolicies"];

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

mod cli;
mod git;
mod json;
mod object_ddl;
mod postgres;
mod project;
mod redaction;
mod reference_data;
mod release;
mod repository;
mod service;
mod ui;
mod workspace;

pub(crate) use crate::cli::ParsedArgs;
pub use crate::cli::{run_cli, usage, CliResult, CommandKind, CommandOutput, OutputFormat};
pub(crate) use crate::json::*;
pub use crate::redaction::{redact_message, redact_postgres_url};

pub use crate::postgres::{
    inspect_postgres, inspect_postgres_command, is_user_schema, normalize_desired_state_text,
    quote_postgres_identifier, render_constraint_sql, render_enum_sql, render_extension_sql,
    render_function_sql, render_grant_sql, render_index_sql, render_materialized_view_sql,
    render_schema_sql, render_sequence_sql, render_table_sql, render_trigger_sql, render_view_sql,
    ColumnInfo, ConstraintInfo, EnumInfo, ExtensionInfo, FunctionInfo, GrantInfo, IndexInfo,
    InspectionCounts, InspectionReport, MaterializedViewInfo, PostgresInventory, SchemaInfo,
    SequenceInfo, TableInfo, TriggerInfo, ViewInfo,
};
pub use crate::reference_data::{
    compare_reference_data_table, data_compare_postgres_with_connection,
    ReferenceDataCompareCounts, ReferenceDataCompareReport, ReferenceDataRegistry,
    ReferenceDataRow, ReferenceDataSelection, ReferenceDataTableConfig, ReferenceDataTableState,
    ReferenceRowCompareResult, ReferenceTableCompareResult,
};
pub use crate::release::{release_postgres_with_inventory, ReleaseReport};
pub use crate::repository::{
    compare_postgres_with_inventory, export_postgres_with_inventory, plan_postgres_with_inventory,
    sync_postgres_with_inventory, CompareReport, CompareSummary, DependencyWarning, ExportReport,
    ExportSelection, PlanItem, PlanReport, PlanSelection, SyncReport,
};
pub use project::{
    init_project, status_report, DbStateProjectStatus, ProjectReport, WorkingTreeStatus,
};
pub use service::{
    load_connection_profiles, parse_service_args, run_service, service_response,
    service_route_definitions, service_usage, ServiceConfig, ServiceHttpResponse,
};
pub use ui::{app_css, app_js, index_html, ui_css, ui_html, ui_js};

fn display_path(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized
        .strip_prefix("///?/")
        .or_else(|| normalized.strip_prefix("//?/"))
        .unwrap_or(&normalized)
        .to_string()
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

fn write_materialized_view_array_field(
    json: &mut String,
    name: &str,
    values: &[MaterializedViewInfo],
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(
            json,
            "materializedViewName",
            &value.materialized_view_name,
            false,
        );
        if let Some(is_populated) = value.is_populated {
            write_json_bool_field(json, "isPopulated", is_populated);
        }
        write_json_optional_string_field(json, "tablespace", value.tablespace.as_deref());
        json.push('}');
    }
    json.push(']');
}

fn write_constraint_array_field(json: &mut String, name: &str, values: &[ConstraintInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "tableName", &value.table_name, false);
        write_json_string_field(json, "constraintName", &value.constraint_name, false);
        write_json_string_field(json, "constraintType", &value.constraint_type, false);
        write_json_array_field(json, "columns", &value.columns);
        write_json_optional_string_field(
            json,
            "referencedSchema",
            value.referenced_schema.as_deref(),
        );
        write_json_optional_string_field(
            json,
            "referencedTable",
            value.referenced_table.as_deref(),
        );
        write_json_array_field(json, "referencedColumns", &value.referenced_columns);
        json.push('}');
    }
    json.push(']');
}

fn write_function_array_field(json: &mut String, name: &str, values: &[FunctionInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "functionName", &value.function_name, false);
        write_json_string_field(json, "identityArguments", &value.identity_arguments, false);
        write_json_optional_string_field(json, "resultType", value.result_type.as_deref());
        write_json_optional_string_field(json, "language", value.language.as_deref());
        write_json_optional_string_field(json, "volatility", value.volatility.as_deref());
        write_json_bool_field(json, "securityDefiner", value.security_definer);
        write_json_bool_field(json, "isStrict", value.is_strict);
        json.push('}');
    }
    json.push(']');
}

fn write_trigger_array_field(json: &mut String, name: &str, values: &[TriggerInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "schemaName", &value.schema_name, true);
        write_json_string_field(json, "relationName", &value.relation_name, false);
        write_json_string_field(json, "triggerName", &value.trigger_name, false);
        write_json_optional_string_field(
            json,
            "triggerFunctionSchema",
            value.trigger_function_schema.as_deref(),
        );
        write_json_optional_string_field(
            json,
            "triggerFunctionName",
            value.trigger_function_name.as_deref(),
        );
        write_json_optional_string_field(json, "timing", value.timing.as_deref());
        write_json_array_field(json, "events", &value.events);
        write_json_optional_string_field(json, "orientation", value.orientation.as_deref());
        json.push('}');
    }
    json.push(']');
}

fn write_grant_array_field(json: &mut String, name: &str, values: &[GrantInfo]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "targetKind", &value.target_kind, true);
        write_json_string_field(json, "schemaName", &value.schema_name, false);
        write_json_optional_string_field(json, "objectName", value.object_name.as_deref());
        write_json_optional_string_field(
            json,
            "identityArguments",
            value.identity_arguments.as_deref(),
        );
        write_json_string_field(json, "grantee", &value.grantee, false);
        write_json_optional_string_field(json, "grantor", value.grantor.as_deref());
        write_json_array_field(json, "privileges", &value.privileges);
        write_json_array_field(json, "grantablePrivileges", &value.grantable_privileges);
        write_json_bool_field(json, "withGrantOption", value.with_grant_option);
        json.push('}');
    }
    json.push(']');
}

fn write_counts_field(json: &mut String, name: &str, counts: &InspectionCounts) {
    json.push(',');
    write!(
        json,
        "\"{}\":{{\"schemas\":{},\"tables\":{},\"columns\":{},\"extensions\":{},\"enums\":{},\"sequences\":{},\"indexes\":{},\"views\":{},\"materializedViews\":{},\"constraints\":{},\"functions\":{},\"triggers\":{},\"grants\":{}}}",
        escape_json(name),
        counts.schemas,
        counts.tables,
        counts.columns,
        counts.extensions,
        counts.enums,
        counts.sequences,
        counts.indexes,
        counts.views,
        counts.materialized_views,
        counts.constraints,
        counts.functions,
        counts.triggers,
        counts.grants
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
        write_json_string_field(json, "operationKind", &value.operation_kind, false);
        write_json_string_field(json, "operationLabel", &value.operation_label, false);
        write_json_string_field(json, "safetyBadge", &value.safety_badge, false);
        write_json_string_field(json, "safetyLevel", &value.safety_level, false);
        write_json_string_field(
            json,
            "operationExplanation",
            &value.operation_explanation,
            false,
        );
        write_json_array_field(json, "operationReasons", &value.operation_reasons);
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

#[cfg(test)]
mod tests;
