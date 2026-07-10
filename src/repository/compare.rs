use crate::repository::discovery::{
    discover_repository_objects, render_database_objects_for_selection, select_repository_objects,
};
use crate::repository::objects::*;
use crate::repository::sync::ExportSelection;
use crate::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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

pub(crate) fn compare_postgres_command(cwd: &Path, parsed: ParsedArgs) -> CompareReport {
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

pub(crate) fn empty_compare_report() -> CompareReport {
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
