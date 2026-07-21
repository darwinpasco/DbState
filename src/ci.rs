use crate::postgres::{invalid_postgres_url_message, is_postgres_connection_url};
use crate::project::status_report;
use crate::reference_data::{parse_reference_data_registry, parse_reference_data_table_state};
use crate::repository::discovery::discover_repository_objects;
use crate::repository::objects::{
    object_ref_from_relative_path, DesiredStateObject, ObjectRef, RepositoryObjectType,
};
use crate::repository::sync::ExportSelection;
use crate::*;
use ::postgres::{Client, NoTls};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

const DISPOSABLE_REQUIRED: &str = "Database State CI can execute repository-defined SQL only against an explicitly supplied disposable validation database. Re-run with --disposable after confirming the target database is disposable.";
const DIRTY_DBSTATE_MANAGED: &str = "Database State CI validates committed branch state. Commit, stash, or clean DbState-managed changes before running CI validation.";

#[derive(Debug, Clone)]
pub struct CiValidateReport {
    pub command: String,
    pub success: bool,
    pub repository_path: String,
    pub validation_target: String,
    pub scope: String,
    pub disposable: bool,
    pub project_structure: CiProjectStructure,
    pub object_files: CiObjectFiles,
    pub reference_data: CiReferenceData,
    pub release_artifacts: CiReleaseArtifacts,
    pub comparison: CiComparison,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CiProjectStructure {
    pub success: bool,
    pub missing_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CiObjectFiles {
    pub total: usize,
    pub applied: usize,
    pub skipped: usize,
    pub failed: usize,
    pub by_type: BTreeMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct CiReferenceData {
    pub registry_valid: bool,
    pub table_files: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CiReleaseArtifacts {
    pub objects: usize,
    pub reference_data: usize,
    pub executed: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CiComparison {
    pub similar: usize,
    pub repository_only: usize,
    pub database_only: usize,
    pub different: usize,
    pub unexpected_drift: bool,
    pub details: CiComparisonDetails,
}

#[derive(Debug, Clone)]
pub struct CiComparisonDetails {
    pub repository_only: Vec<CiDriftDetail>,
    pub database_only: Vec<CiDriftDetail>,
    pub different: Vec<CiDriftDetail>,
}

#[derive(Debug, Clone)]
pub struct CiDriftDetail {
    pub status: String,
    pub object_type: String,
    pub identifier: String,
    pub repository_path: Option<String>,
    pub database_identifier: Option<String>,
}

pub(crate) fn ci_validate_command(cwd: &Path, parsed: ParsedArgs) -> CiValidateReport {
    let repository_path = parsed
        .repository
        .as_deref()
        .map(|path| resolve_cli_path(cwd, path))
        .unwrap_or_else(|| cwd.to_path_buf());
    let mut report = empty_ci_report(&repository_path, parsed.disposable);

    if !parsed.disposable {
        report.errors.push(DISPOSABLE_REQUIRED.to_string());
        return report;
    }

    let Some(repository_arg) = parsed.repository.as_deref() else {
        report
            .errors
            .push("Missing --repository <path>.".to_string());
        return report;
    };
    let repository_path = resolve_cli_path(cwd, repository_arg);
    report.repository_path = display_path(&repository_path);

    let Some(postgres_url) = parsed.postgres_url.as_deref() else {
        report
            .errors
            .push("Missing --postgres-url <postgres-url>.".to_string());
        return report;
    };
    if !is_postgres_connection_url(postgres_url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    let report_path = match parsed.report_path.as_deref() {
        Some(value) => match prepare_report_path(&repository_path, value) {
            Ok(path) => Some(path),
            Err(error) => {
                report.errors.push(error);
                return report;
            }
        },
        None => None,
    };
    report.report_path = report_path.as_ref().map(|path| display_path(path));

    let project = status_report(&repository_path, CommandKind::CiValidate);
    report.project_structure.missing_paths = project.missing_paths.clone();
    if !project.is_git_repository {
        report
            .errors
            .push("Repository path is not inside a Git repository.".to_string());
        return finish_ci_report(report, report_path.as_ref());
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        report.errors.push(
            "DbState PostgreSQL project structure is incomplete. Run dbstate init first."
                .to_string(),
        );
        return finish_ci_report(report, report_path.as_ref());
    }
    report.project_structure.success = true;

    let Some(root) = project.git_root.as_deref().map(PathBuf::from) else {
        report
            .errors
            .push("Repository path is not inside a Git repository.".to_string());
        return finish_ci_report(report, report_path.as_ref());
    };

    let dirty_managed_paths = project
        .dirty_paths
        .iter()
        .filter(|path| is_dbstate_managed_path(path))
        .cloned()
        .collect::<Vec<_>>();
    if !dirty_managed_paths.is_empty() {
        report.errors.push(DIRTY_DBSTATE_MANAGED.to_string());
        report.errors.extend(dirty_managed_paths);
        return finish_ci_report(report, report_path.as_ref());
    }

    validate_reference_data(&root, &mut report);
    summarize_release_artifacts(&root, &mut report);
    if !report.errors.is_empty() {
        return finish_ci_report(report, report_path.as_ref());
    }

    let before_inventory = match inspect_postgres(postgres_url) {
        Ok(inventory) => inventory,
        Err(error) => {
            report.errors.push(redact_message(&error, postgres_url));
            return finish_ci_report(report, report_path.as_ref());
        }
    };
    if let Err(errors) = validate_disposable_database(&before_inventory) {
        report.errors.extend(errors);
        return finish_ci_report(report, report_path.as_ref());
    }
    let bootstrap_public_schema_exists = before_inventory
        .schemas
        .iter()
        .any(|schema| schema.name == "public");

    let repo_import = match discover_repository_objects(&root) {
        Ok(import) => import,
        Err(error) => {
            report.errors.push(error);
            return finish_ci_report(report, report_path.as_ref());
        }
    };
    report.object_files.skipped = repo_import.skipped.len();
    report.warnings.extend(repo_import.warnings);
    report.errors.extend(repo_import.errors);
    if !report.errors.is_empty() {
        return finish_ci_report(report, report_path.as_ref());
    }

    let objects = ci_ordered_objects(repo_import.objects.into_values().collect());
    report.object_files.total = objects.len();
    let mut pending_functions = Vec::new();
    for object in objects {
        *report
            .object_files
            .by_type
            .entry(ci_object_type_label(&object).to_string())
            .or_insert(0) += 1;
        if ci_should_skip_bootstrap_public_schema(&object, bootstrap_public_schema_exists) {
            report.object_files.skipped += 1;
            report.warnings.push(format!(
                "Skipped {} because the disposable database already has the bootstrap public schema.",
                object.relative_path
            ));
            continue;
        }
        if object.object_type == RepositoryObjectType::Function {
            pending_functions.push(object);
            continue;
        }
        if !pending_functions.is_empty()
            && !apply_ci_function_objects(postgres_url, &pending_functions, &mut report)
        {
            return finish_ci_report(report, report_path.as_ref());
        }
        pending_functions.clear();
        match apply_ci_object(postgres_url, &object) {
            Ok(()) => report.object_files.applied += 1,
            Err(error) => {
                report.object_files.failed += 1;
                report.errors.push(error.message);
                return finish_ci_report(report, report_path.as_ref());
            }
        }
    }
    if !pending_functions.is_empty()
        && !apply_ci_function_objects(postgres_url, &pending_functions, &mut report)
    {
        return finish_ci_report(report, report_path.as_ref());
    }

    let after_inventory = match inspect_postgres(postgres_url) {
        Ok(inventory) => inventory,
        Err(error) => {
            report.errors.push(redact_message(&error, postgres_url));
            return finish_ci_report(report, report_path.as_ref());
        }
    };

    let compare = compare_postgres_with_inventory(&root, &after_inventory, &ExportSelection::All);
    apply_compare_back_result(&mut report, &compare);
    report.warnings.extend(compare.warnings);
    if !compare.errors.is_empty() {
        report.errors.extend(compare.errors);
        return finish_ci_report(report, report_path.as_ref());
    }
    if report.comparison.unexpected_drift {
        report.errors.push(
            "Compare-back found unexpected drift after building disposable database.".to_string(),
        );
        return finish_ci_report(report, report_path.as_ref());
    }

    report.success = true;
    finish_ci_report(report, report_path.as_ref())
}

fn empty_ci_report(repository_path: &Path, disposable: bool) -> CiValidateReport {
    CiValidateReport {
        command: "ci validate".to_string(),
        success: false,
        repository_path: display_path(repository_path),
        validation_target: "disposable-postgres".to_string(),
        scope: "database-state".to_string(),
        disposable,
        project_structure: CiProjectStructure {
            success: false,
            missing_paths: Vec::new(),
        },
        object_files: CiObjectFiles {
            total: 0,
            applied: 0,
            skipped: 0,
            failed: 0,
            by_type: BTreeMap::new(),
        },
        reference_data: CiReferenceData {
            registry_valid: false,
            table_files: 0,
            warnings: Vec::new(),
        },
        release_artifacts: CiReleaseArtifacts {
            objects: 0,
            reference_data: 0,
            executed: false,
            warnings: vec![
                "Release artifacts are validated for safe paths only and are not executed by Database State CI.".to_string(),
            ],
        },
        comparison: CiComparison {
            similar: 0,
            repository_only: 0,
            database_only: 0,
            different: 0,
            unexpected_drift: false,
            details: CiComparisonDetails {
                repository_only: Vec::new(),
                database_only: Vec::new(),
                different: Vec::new(),
            },
        },
        warnings: Vec::new(),
        errors: Vec::new(),
        report_path: None,
    }
}

fn apply_compare_back_result(report: &mut CiValidateReport, compare: &CompareReport) {
    let normalized = normalize_ci_compare_back(compare);
    if normalized.ignored_bootstrap_public_schema_grants > 0 {
        report.warnings.push(
            "Ignored PostgreSQL bootstrap public schema grant noise during CI compare-back."
                .to_string(),
        );
    }
    let compare = &normalized.compare;
    report.comparison.similar = compare.in_sync.len();
    report.comparison.repository_only = compare.repo_only.len();
    report.comparison.database_only = compare.database_only.len();
    report.comparison.different = compare.repo_different.len();
    report.comparison.unexpected_drift = report.comparison.repository_only > 0
        || report.comparison.database_only > 0
        || report.comparison.different > 0;
    report.comparison.details = compare_back_details(compare);
}

#[derive(Debug, Clone)]
struct NormalizedCiCompareBack {
    compare: CompareReport,
    ignored_bootstrap_public_schema_grants: usize,
}

fn normalize_ci_compare_back(compare: &CompareReport) -> NormalizedCiCompareBack {
    let mut normalized = compare.clone();
    let before_database_only = normalized.database_only.len();
    normalized
        .database_only
        .retain(|path| !is_ignored_ci_bootstrap_public_schema_database_only_grant(path));
    let before_different = normalized.repo_different.len();
    normalized
        .repo_different
        .retain(|path| !is_ignored_ci_bootstrap_public_schema_different_grant(path));

    NormalizedCiCompareBack {
        ignored_bootstrap_public_schema_grants: before_database_only
            - normalized.database_only.len()
            + before_different
            - normalized.repo_different.len(),
        compare: normalized,
    }
}

fn is_ignored_ci_bootstrap_public_schema_database_only_grant(relative_path: &str) -> bool {
    is_ci_bootstrap_public_schema_grant_for(relative_path, &["pg_database_owner"])
}

fn is_ignored_ci_bootstrap_public_schema_different_grant(relative_path: &str) -> bool {
    is_ci_bootstrap_public_schema_grant_for(relative_path, &["postgres", "public"])
}

fn is_ci_bootstrap_public_schema_grant_for(relative_path: &str, grantees: &[&str]) -> bool {
    match object_ref_from_relative_path(relative_path) {
        Ok(ObjectRef::Grant {
            target_kind,
            schema,
            object,
            signature,
            grantee,
        }) => {
            target_kind == "schema"
                && schema == "public"
                && object.is_none()
                && signature.is_none()
                && grantees.iter().any(|allowed| grantee == *allowed)
        }
        _ => false,
    }
}

fn compare_back_details(compare: &CompareReport) -> CiComparisonDetails {
    CiComparisonDetails {
        repository_only: compare
            .repo_only
            .iter()
            .map(|path| drift_detail_from_path("repositoryOnly", path, true, false))
            .collect(),
        database_only: compare
            .database_only
            .iter()
            .map(|path| drift_detail_from_path("databaseOnly", path, false, true))
            .collect(),
        different: compare
            .repo_different
            .iter()
            .map(|path| drift_detail_from_path("different", path, true, true))
            .collect(),
    }
}

fn drift_detail_from_path(
    status: &str,
    relative_path: &str,
    include_repository_path: bool,
    include_database_identifier: bool,
) -> CiDriftDetail {
    let (object_type, identifier) = match object_ref_from_relative_path(relative_path) {
        Ok(object_ref) => (
            object_ref.object_type().to_string(),
            object_ref_identifier(&object_ref),
        ),
        Err(_) => (
            object_type_from_relative_path(relative_path).to_string(),
            relative_path.to_string(),
        ),
    };

    CiDriftDetail {
        status: status.to_string(),
        object_type,
        identifier: identifier.clone(),
        repository_path: include_repository_path.then(|| relative_path.to_string()),
        database_identifier: include_database_identifier.then_some(identifier),
    }
}

fn object_ref_identifier(object_ref: &ObjectRef) -> String {
    match object_ref {
        ObjectRef::Schema(schema) => schema.clone(),
        ObjectRef::Table { schema, table } => format!("{schema}.{table}"),
        ObjectRef::Extension(extension) => extension.clone(),
        ObjectRef::Enum { schema, enum_name } => format!("{schema}.{enum_name}"),
        ObjectRef::Domain { schema, domain } => format!("{schema}.{domain}"),
        ObjectRef::Aggregate {
            schema,
            aggregate,
            signature,
        } => format!("{schema}.{aggregate}({})", signature.replace('_', ", ")),
        ObjectRef::Sequence { schema, sequence } => format!("{schema}.{sequence}"),
        ObjectRef::Index {
            schema,
            table,
            index,
        } => format!("{schema}.{table}.{index}"),
        ObjectRef::View { schema, view } => format!("{schema}.{view}"),
        ObjectRef::MaterializedView {
            schema,
            materialized_view,
        } => format!("{schema}.{materialized_view}"),
        ObjectRef::Constraint {
            schema,
            table,
            constraint,
        } => format!("{schema}.{table}.{constraint}"),
        ObjectRef::Function {
            schema,
            function,
            signature,
        } => format!("{schema}.{function}({})", signature.replace('_', ", ")),
        ObjectRef::Trigger {
            schema,
            relation,
            trigger,
        } => format!("{schema}.{relation}.{trigger}"),
        ObjectRef::Grant {
            target_kind,
            schema,
            object,
            signature,
            grantee,
        } => match (object, signature) {
            (Some(object), Some(signature)) => {
                format!("{target_kind}.{schema}.{object}.{signature}.{grantee}")
            }
            (Some(object), None) => format!("{target_kind}.{schema}.{object}.{grantee}"),
            (None, _) => format!("{target_kind}.{schema}.{grantee}"),
        },
        ObjectRef::RlsPolicy {
            schema,
            table,
            policy,
        } => format!("{schema}.{table}.{policy}"),
    }
}

fn object_type_from_relative_path(relative_path: &str) -> &'static str {
    let path = relative_path.replace('\\', "/");
    if path.starts_with("database/objects/schemas/") {
        "schema"
    } else if path.starts_with("database/objects/tables/") {
        "table"
    } else if path.starts_with("database/objects/extensions/") {
        "extension"
    } else if path.starts_with("database/objects/enums/") {
        "enum"
    } else if path.starts_with("database/objects/domains/") {
        "domain"
    } else if path.starts_with("database/objects/aggregates/") {
        "aggregate"
    } else if path.starts_with("database/objects/sequences/") {
        "sequence"
    } else if path.starts_with("database/objects/indexes/") {
        "index"
    } else if path.starts_with("database/objects/views/") {
        "view"
    } else if path.starts_with("database/objects/materialized-views/") {
        "materializedView"
    } else if path.starts_with("database/objects/functions/") {
        "function"
    } else if path.starts_with("database/objects/triggers/") {
        "trigger"
    } else if path.starts_with("database/objects/rls-policies/") {
        "rlsPolicy"
    } else if path.starts_with("database/objects/constraints/") {
        "constraint"
    } else if path.starts_with("database/objects/grants/") {
        "grant"
    } else {
        "object"
    }
}

fn resolve_cli_path(cwd: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn is_dbstate_managed_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized == "database"
        || normalized.starts_with("database/objects/")
        || normalized.starts_with("database/reference-data/")
        || normalized.starts_with("database/releases/")
}

fn validate_reference_data(root: &Path, report: &mut CiValidateReport) {
    let registry_path = root.join("database/reference-data/dbstate.reference-data.yml");
    let content = match fs::read_to_string(&registry_path) {
        Ok(content) => content,
        Err(error) => {
            report.errors.push(format!(
                "Could not read database/reference-data/dbstate.reference-data.yml: {error}"
            ));
            return;
        }
    };
    let registry = match parse_reference_data_registry(&content) {
        Ok(registry) => registry,
        Err(error) => {
            report.errors.push(error);
            return;
        }
    };
    report.reference_data.registry_valid = true;

    for table in &registry.tables {
        let table_path = root.join("database/reference-data").join(&table.file);
        let table_content = match fs::read_to_string(&table_path) {
            Ok(content) => content,
            Err(error) => {
                report.errors.push(format!(
                    "Could not read reference-data table file {}: {error}",
                    table.file
                ));
                continue;
            }
        };
        match parse_reference_data_table_state(&table_content, table) {
            Ok(_) => report.reference_data.table_files += 1,
            Err(error) => report.errors.push(error),
        }
    }
}

fn summarize_release_artifacts(root: &Path, report: &mut CiValidateReport) {
    report.release_artifacts.objects = count_files(&root.join("database/releases/objects"));
    report.release_artifacts.reference_data =
        count_files(&root.join("database/releases/reference-data"));
}

fn count_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .count()
}

pub(crate) fn validate_disposable_database(
    inventory: &PostgresInventory,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let user_schema_count = inventory
        .schemas
        .iter()
        .filter(|schema| schema.name != "public")
        .count();
    let object_count = user_schema_count
        + inventory.tables.len()
        + inventory.views.len()
        + inventory.materialized_views.len()
        + inventory.functions.len()
        + inventory.aggregates.len()
        + inventory.sequences.len()
        + inventory.enums.len()
        + inventory.domains.len()
        + inventory.triggers.len()
        + inventory.rls_policies.len()
        + inventory.extensions.len()
        + inventory.indexes.len()
        + inventory.constraints.len();

    if object_count > 0 {
        errors.push(
            "Disposable validation database is not empty/user-clean. DbState will not drop, truncate, clean, or reset it automatically."
                .to_string(),
        );
        errors.push(format!(
            "Detected user object count before CI validation: {object_count}."
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn ci_ordered_objects(mut objects: Vec<DesiredStateObject>) -> Vec<DesiredStateObject> {
    objects.sort_by(|left, right| {
        ci_object_apply_key(left)
            .cmp(&ci_object_apply_key(right))
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    objects
}

fn ci_object_apply_key(object: &DesiredStateObject) -> u8 {
    match object.object_type {
        RepositoryObjectType::Schema => 1,
        RepositoryObjectType::Extension => 2,
        RepositoryObjectType::Enum => 3,
        RepositoryObjectType::Domain => 4,
        RepositoryObjectType::Sequence => 5,
        RepositoryObjectType::Table => 6,
        RepositoryObjectType::Function => 7,
        RepositoryObjectType::Aggregate => 8,
        RepositoryObjectType::Constraint => {
            let path = object.relative_path.replace('\\', "/");
            if path.contains("/constraints/primary-keys/") {
                9
            } else if path.contains("/constraints/unique-constraints/") {
                10
            } else if path.contains("/constraints/check-constraints/") {
                11
            } else {
                12
            }
        }
        RepositoryObjectType::View => 13,
        RepositoryObjectType::MaterializedView => 14,
        RepositoryObjectType::Index => 15,
        RepositoryObjectType::Trigger => 16,
        RepositoryObjectType::RlsPolicy => 17,
        RepositoryObjectType::Grant => 18,
    }
}

fn ci_should_skip_bootstrap_public_schema(
    object: &DesiredStateObject,
    bootstrap_public_schema_exists: bool,
) -> bool {
    bootstrap_public_schema_exists
        && matches!(&object.object_type, RepositoryObjectType::Schema)
        && object.schema_name == "public"
}

fn ci_object_type_label(object: &DesiredStateObject) -> &'static str {
    match object.object_type {
        RepositoryObjectType::Schema => "schema",
        RepositoryObjectType::Extension => "extension",
        RepositoryObjectType::Enum => "enum",
        RepositoryObjectType::Domain => "domain",
        RepositoryObjectType::Sequence => "sequence",
        RepositoryObjectType::Table => "table",
        RepositoryObjectType::Function => "function",
        RepositoryObjectType::Aggregate => "aggregate",
        RepositoryObjectType::Constraint => "constraint",
        RepositoryObjectType::Index => "index",
        RepositoryObjectType::View => "view",
        RepositoryObjectType::MaterializedView => "materializedView",
        RepositoryObjectType::Trigger => "trigger",
        RepositoryObjectType::RlsPolicy => "rlsPolicy",
        RepositoryObjectType::Grant => "grant",
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CiSqlExecutionError {
    pub message: String,
    pub dependency_style: bool,
}

fn apply_ci_function_objects(
    postgres_url: &str,
    functions: &[DesiredStateObject],
    report: &mut CiValidateReport,
) -> bool {
    apply_ci_function_objects_with(functions, report, |object| {
        apply_ci_object(postgres_url, object)
    })
}

fn apply_ci_function_objects_with<F>(
    functions: &[DesiredStateObject],
    report: &mut CiValidateReport,
    mut apply: F,
) -> bool
where
    F: FnMut(&DesiredStateObject) -> Result<(), CiSqlExecutionError>,
{
    let mut remaining = functions.to_vec();
    let mut deferred_seen = BTreeSet::new();
    let mut last_errors = BTreeMap::new();

    while !remaining.is_empty() {
        let mut progress = false;
        let mut next = Vec::new();

        for object in remaining {
            match apply(&object) {
                Ok(()) => {
                    report.object_files.applied += 1;
                    progress = true;
                    last_errors.remove(&object.relative_path);
                }
                Err(error) if error.dependency_style => {
                    if deferred_seen.insert(object.relative_path.clone()) {
                        report.warnings.push(format!(
                            "Deferred {} due to a missing dependency and will retry after other functions are applied.",
                            object.relative_path
                        ));
                    }
                    last_errors.insert(object.relative_path.clone(), error.message);
                    next.push(object);
                }
                Err(error) => {
                    report.object_files.failed += 1;
                    report.errors.push(error.message);
                    return false;
                }
            }
        }

        if next.is_empty() {
            return true;
        }
        if !progress {
            report.object_files.failed += next.len();
            report.errors.push(
                "Function dependency retry made no progress. Remaining function files could not be applied."
                    .to_string(),
            );
            for object in next {
                if let Some(error) = last_errors.remove(&object.relative_path) {
                    report.errors.push(error);
                }
            }
            return false;
        }

        report.warnings.push(format!(
            "Retrying {} deferred PostgreSQL function file(s) after applying available dependencies.",
            next.len()
        ));
        remaining = next;
    }

    true
}

fn apply_ci_object(
    postgres_url: &str,
    object: &DesiredStateObject,
) -> Result<(), CiSqlExecutionError> {
    let mut client = Client::connect(postgres_url, NoTls).map_err(|error| CiSqlExecutionError {
        message: redact_message(&error.to_string(), postgres_url),
        dependency_style: false,
    })?;
    client
        .batch_execute(&object.content)
        .map_err(|error| ci_sql_execution_error(postgres_url, object, &error))
}

fn ci_sql_execution_error(
    postgres_url: &str,
    object: &DesiredStateObject,
    error: &::postgres::Error,
) -> CiSqlExecutionError {
    let (message, dependency_style) = if let Some(db_error) = error.as_db_error() {
        let position = db_error.position().map(|position| format!("{position:?}"));
        let message = format_ci_sql_execution_message(
            object,
            db_error.message(),
            db_error.detail(),
            db_error.hint(),
            position.as_deref(),
            postgres_url,
        );
        (message.clone(), is_dependency_style_sql_error(&message))
    } else {
        let message = format_ci_sql_execution_message(
            object,
            &error.to_string(),
            None,
            None,
            None,
            postgres_url,
        );
        (message.clone(), is_dependency_style_sql_error(&message))
    };
    CiSqlExecutionError {
        message,
        dependency_style,
    }
}

pub(crate) fn format_ci_sql_execution_message(
    object: &DesiredStateObject,
    primary: &str,
    detail: Option<&str>,
    hint: Option<&str>,
    position: Option<&str>,
    postgres_url: &str,
) -> String {
    let mut message = format!(
        "SQL execution failed for {} ({}): {}",
        object.relative_path,
        ci_object_type_label(object),
        primary
    );
    if let Some(detail) = detail.filter(|value| !value.trim().is_empty()) {
        message.push_str("\nDetail: ");
        message.push_str(detail.trim());
    }
    if let Some(hint) = hint.filter(|value| !value.trim().is_empty()) {
        message.push_str("\nHint: ");
        message.push_str(hint.trim());
    }
    if let Some(position) = position.filter(|value| !value.trim().is_empty()) {
        message.push_str("\nPosition: ");
        message.push_str(position.trim());
    }
    redact_message(&message, postgres_url)
}

fn is_dependency_style_sql_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "function ",
        "type ",
        "relation ",
        "operator ",
        "could not find function",
        "does not exist",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
        && [
            "function",
            "type",
            "relation",
            "operator",
            "could not find function",
        ]
        .iter()
        .any(|pattern| lower.contains(pattern))
}

fn prepare_report_path(repository_root: &Path, report_path: &str) -> Result<PathBuf, String> {
    let raw = PathBuf::from(report_path);
    if raw.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) && !raw.is_absolute()
    {
        return Err("CI report path must not contain traversal components.".to_string());
    }
    if raw
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err("CI report path must not contain traversal components.".to_string());
    }
    let path = if raw.is_absolute() {
        raw
    } else {
        repository_root.join(raw)
    };
    if path.starts_with(repository_root.join("database")) {
        return Err("CI report path must not be under database/.".to_string());
    }
    Ok(path)
}

fn write_markdown_report(report: &CiValidateReport, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create CI report directory: {error}"))?;
    }
    fs::write(path, report.to_markdown())
        .map_err(|error| format!("Could not write CI report {}: {error}", display_path(path)))
}

fn finish_ci_report(
    mut report: CiValidateReport,
    report_path: Option<&PathBuf>,
) -> CiValidateReport {
    if let Some(path) = report_path {
        if let Err(error) = write_markdown_report(&report, path) {
            report.success = false;
            report.errors.push(error);
        }
    }
    report
}

impl CiValidateReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Repository path: {}", self.repository_path).ok();
        writeln!(text, "Validation target: {}", self.validation_target).ok();
        writeln!(text, "Scope: {}", self.scope).ok();
        writeln!(text, "Disposable: {}", self.disposable).ok();
        writeln!(
            text,
            "Project structure: {}",
            if self.project_structure.success {
                "passed"
            } else {
                "failed"
            }
        )
        .ok();
        write_path_list(
            &mut text,
            "Missing project paths",
            &self.project_structure.missing_paths,
        );
        writeln!(
            text,
            "Object files: total={}, applied={}, skipped={}, failed={}",
            self.object_files.total,
            self.object_files.applied,
            self.object_files.skipped,
            self.object_files.failed
        )
        .ok();
        writeln!(
            text,
            "Reference data: registryValid={}, tableFiles={}",
            self.reference_data.registry_valid, self.reference_data.table_files
        )
        .ok();
        writeln!(
            text,
            "Release artifacts: objects={}, referenceData={}, executed={}",
            self.release_artifacts.objects,
            self.release_artifacts.reference_data,
            self.release_artifacts.executed
        )
        .ok();
        writeln!(
            text,
            "Compare-back: similar={}, repositoryOnly={}, databaseOnly={}, different={}, unexpectedDrift={}",
            self.comparison.similar,
            self.comparison.repository_only,
            self.comparison.database_only,
            self.comparison.different,
            self.comparison.unexpected_drift
        )
        .ok();
        if self.comparison.unexpected_drift {
            write_compare_back_details_text(&mut text, &self.comparison.details);
        }
        writeln!(
            text,
            "Report path: {}",
            self.report_path.as_deref().unwrap_or("<none>")
        )
        .ok();
        for warning in &self.release_artifacts.warnings {
            writeln!(text, "Warning: {warning}").ok();
        }
        for warning in &self.reference_data.warnings {
            writeln!(text, "Warning: {warning}").ok();
        }
        for warning in &self.warnings {
            writeln!(text, "Warning: {warning}").ok();
        }
        for error in &self.errors {
            writeln!(text, "Error: {error}").ok();
        }
        if self.success {
            writeln!(text, "Result: PASS").ok();
        } else {
            writeln!(text, "Result: FAIL").ok();
        }
        text
    }

    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "command", &self.command, true);
        write_json_bool_field(&mut json, "success", self.success);
        write_json_string_field(&mut json, "repositoryPath", &self.repository_path, false);
        write_json_string_field(
            &mut json,
            "validationTarget",
            &self.validation_target,
            false,
        );
        write_json_string_field(&mut json, "scope", &self.scope, false);
        write_json_bool_field(&mut json, "disposable", self.disposable);
        write_project_structure_json(&mut json, &self.project_structure);
        write_object_files_json(&mut json, &self.object_files);
        write_reference_data_json(&mut json, &self.reference_data);
        write_release_artifacts_json(&mut json, &self.release_artifacts);
        write_comparison_json(&mut json, &self.comparison);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        write_json_optional_string_field(&mut json, "reportPath", self.report_path.as_deref());
        json.push('}');
        json
    }

    pub fn to_markdown(&self) -> String {
        let mut markdown = String::new();
        writeln!(markdown, "# Database State CI Report\n").ok();
        writeln!(markdown, "## Summary\n").ok();
        writeln!(
            markdown,
            "- Result: {}",
            if self.success { "PASS" } else { "FAIL" }
        )
        .ok();
        writeln!(markdown, "- Command: `{}`", self.command).ok();
        writeln!(markdown, "- Scope: `{}`", self.scope).ok();
        writeln!(markdown, "\n## Repository\n").ok();
        writeln!(markdown, "- Path: `{}`", self.repository_path).ok();
        writeln!(markdown, "\n## Disposable validation database\n").ok();
        writeln!(markdown, "- Target: `{}`", self.validation_target).ok();
        writeln!(markdown, "- Disposable confirmed: `{}`", self.disposable).ok();
        writeln!(markdown, "\n## Project structure validation\n").ok();
        writeln!(markdown, "- Success: `{}`", self.project_structure.success).ok();
        for path in &self.project_structure.missing_paths {
            writeln!(markdown, "- Missing: `{path}`").ok();
        }
        writeln!(markdown, "\n## Object build summary\n").ok();
        writeln!(markdown, "- Total: `{}`", self.object_files.total).ok();
        writeln!(markdown, "- Applied: `{}`", self.object_files.applied).ok();
        writeln!(markdown, "- Failed: `{}`", self.object_files.failed).ok();
        writeln!(markdown, "\n## Reference-data validation\n").ok();
        writeln!(
            markdown,
            "- Registry valid: `{}`",
            self.reference_data.registry_valid
        )
        .ok();
        writeln!(
            markdown,
            "- Table files: `{}`",
            self.reference_data.table_files
        )
        .ok();
        writeln!(markdown, "\n## Release artifact validation\n").ok();
        writeln!(
            markdown,
            "- Object artifacts: `{}`",
            self.release_artifacts.objects
        )
        .ok();
        writeln!(
            markdown,
            "- Reference-data artifacts: `{}`",
            self.release_artifacts.reference_data
        )
        .ok();
        writeln!(
            markdown,
            "- Executed: `{}`",
            self.release_artifacts.executed
        )
        .ok();
        writeln!(markdown, "\n## Compare-back result\n").ok();
        writeln!(markdown, "- Similar: `{}`", self.comparison.similar).ok();
        writeln!(
            markdown,
            "- Repository only: `{}`",
            self.comparison.repository_only
        )
        .ok();
        writeln!(
            markdown,
            "- Database only: `{}`",
            self.comparison.database_only
        )
        .ok();
        writeln!(markdown, "- Different: `{}`", self.comparison.different).ok();
        writeln!(
            markdown,
            "- Unexpected drift: `{}`",
            self.comparison.unexpected_drift
        )
        .ok();
        write_compare_back_details_markdown(&mut markdown, &self.comparison.details);
        writeln!(markdown, "\n## Warnings\n").ok();
        for warning in self
            .release_artifacts
            .warnings
            .iter()
            .chain(self.reference_data.warnings.iter())
            .chain(self.warnings.iter())
        {
            writeln!(markdown, "- {warning}").ok();
        }
        writeln!(markdown, "\n## Errors\n").ok();
        for error in &self.errors {
            writeln!(markdown, "- {error}").ok();
        }
        writeln!(markdown, "\n## Safety statement\n").ok();
        writeln!(markdown, "Database State CI executed repository desired-state object SQL only against the supplied disposable validation database. It did not execute release artifacts, did not execute reference-data review scripts, did not apply changes to a real environment, and did not mutate Git.").ok();
        markdown
    }
}

fn write_project_structure_json(json: &mut String, value: &CiProjectStructure) {
    json.push_str(",\"projectStructure\":{");
    write!(json, "\"success\":{}", value.success).ok();
    write_json_array_field(json, "missingPaths", &value.missing_paths);
    json.push('}');
}

fn write_object_files_json(json: &mut String, value: &CiObjectFiles) {
    json.push_str(",\"objectFiles\":{");
    write!(json, "\"total\":{}", value.total).ok();
    write!(json, ",\"applied\":{}", value.applied).ok();
    write!(json, ",\"skipped\":{}", value.skipped).ok();
    write!(json, ",\"failed\":{}", value.failed).ok();
    json.push_str(",\"byType\":{");
    for (index, (key, count)) in value.by_type.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(json, "\"{}\":{}", escape_json(key), count).ok();
    }
    json.push_str("}}");
}

fn write_reference_data_json(json: &mut String, value: &CiReferenceData) {
    json.push_str(",\"referenceData\":{");
    write!(json, "\"registryValid\":{}", value.registry_valid).ok();
    write!(json, ",\"tableFiles\":{}", value.table_files).ok();
    write_json_array_field(json, "warnings", &value.warnings);
    json.push('}');
}

fn write_release_artifacts_json(json: &mut String, value: &CiReleaseArtifacts) {
    json.push_str(",\"releaseArtifacts\":{");
    write!(json, "\"objects\":{}", value.objects).ok();
    write!(json, ",\"referenceData\":{}", value.reference_data).ok();
    write!(json, ",\"executed\":{}", value.executed).ok();
    write_json_array_field(json, "warnings", &value.warnings);
    json.push('}');
}

fn write_comparison_json(json: &mut String, value: &CiComparison) {
    json.push_str(",\"comparison\":{");
    write!(json, "\"similar\":{}", value.similar).ok();
    write!(json, ",\"repositoryOnly\":{}", value.repository_only).ok();
    write!(json, ",\"databaseOnly\":{}", value.database_only).ok();
    write!(json, ",\"different\":{}", value.different).ok();
    write!(json, ",\"unexpectedDrift\":{}", value.unexpected_drift).ok();
    write_compare_back_details_json(json, &value.details);
    json.push('}');
}

fn write_compare_back_details_text(text: &mut String, details: &CiComparisonDetails) {
    writeln!(text, "Compare-back drift details:").ok();
    write_drift_detail_text(text, "repositoryOnly", &details.repository_only);
    write_drift_detail_text(text, "databaseOnly", &details.database_only);
    write_drift_detail_text(text, "different", &details.different);
}

fn write_drift_detail_text(text: &mut String, title: &str, items: &[CiDriftDetail]) {
    writeln!(text, "  {title}:").ok();
    if items.is_empty() {
        writeln!(text, "    None").ok();
        return;
    }
    for item in items {
        writeln!(text, "    - {} {}", item.object_type, item.identifier).ok();
        if let Some(path) = &item.repository_path {
            writeln!(text, "      repository: {path}").ok();
        }
        if let Some(identifier) = &item.database_identifier {
            writeln!(text, "      database: {identifier}").ok();
        }
    }
}

fn write_compare_back_details_markdown(markdown: &mut String, details: &CiComparisonDetails) {
    writeln!(markdown, "\n## Compare-back drift details\n").ok();
    write_drift_detail_markdown(markdown, "Repository only", &details.repository_only);
    write_drift_detail_markdown(markdown, "Database only", &details.database_only);
    write_drift_detail_markdown(markdown, "Different", &details.different);
}

fn write_drift_detail_markdown(markdown: &mut String, title: &str, items: &[CiDriftDetail]) {
    writeln!(markdown, "### {title}\n").ok();
    if items.is_empty() {
        writeln!(markdown, "None.\n").ok();
        return;
    }
    for item in items {
        writeln!(markdown, "- `{}` `{}`", item.object_type, item.identifier).ok();
        if let Some(path) = &item.repository_path {
            writeln!(markdown, "  - Repository: `{path}`").ok();
        }
        if let Some(identifier) = &item.database_identifier {
            writeln!(markdown, "  - Database: `{identifier}`").ok();
        }
    }
    writeln!(markdown).ok();
}

fn write_compare_back_details_json(json: &mut String, details: &CiComparisonDetails) {
    json.push_str(",\"details\":{");
    write_drift_detail_json_array(json, "repositoryOnly", &details.repository_only, true);
    write_drift_detail_json_array(json, "databaseOnly", &details.database_only, false);
    write_drift_detail_json_array(json, "different", &details.different, false);
    json.push('}');
}

fn write_drift_detail_json_array(
    json: &mut String,
    name: &str,
    items: &[CiDriftDetail],
    first: bool,
) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "status", &item.status, true);
        write_json_string_field(json, "objectType", &item.object_type, false);
        write_json_string_field(json, "identifier", &item.identifier, false);
        write_json_optional_string_field(json, "repositoryPath", item.repository_path.as_deref());
        write_json_optional_string_field(
            json,
            "databaseIdentifier",
            item.database_identifier.as_deref(),
        );
        json.push('}');
    }
    json.push(']');
}

#[cfg(test)]
pub(crate) fn ci_apply_order_for_test(objects: Vec<DesiredStateObject>) -> Vec<String> {
    ci_ordered_objects(objects)
        .into_iter()
        .map(|object| object.relative_path)
        .collect()
}

#[cfg(test)]
pub(crate) fn ci_apply_function_retry_for_test(
    functions: Vec<DesiredStateObject>,
    mut outcomes: BTreeMap<String, Vec<Result<(), CiSqlExecutionError>>>,
) -> CiValidateReport {
    let mut report = empty_ci_report(Path::new("."), true);
    report.object_files.total = functions.len();
    let success = apply_ci_function_objects_with(&functions, &mut report, |object| {
        let queue = outcomes.entry(object.relative_path.clone()).or_default();
        if queue.is_empty() {
            Ok(())
        } else {
            queue.remove(0)
        }
    });
    report.success = success;
    report
}

#[cfg(test)]
pub(crate) fn ci_release_summary_for_test(root: &Path) -> CiReleaseArtifacts {
    let mut report = empty_ci_report(root, true);
    summarize_release_artifacts(root, &mut report);
    report.release_artifacts
}

#[cfg(test)]
pub(crate) fn ci_report_with_compare_back_details_for_test(
    repository_only: Vec<&str>,
    database_only: Vec<&str>,
    different: Vec<&str>,
) -> CiValidateReport {
    let mut compare = crate::repository::compare::empty_compare_report();
    compare
        .repo_only
        .extend(repository_only.into_iter().map(str::to_string));
    compare
        .database_only
        .extend(database_only.into_iter().map(str::to_string));
    compare
        .repo_different
        .extend(different.into_iter().map(str::to_string));

    let mut report = empty_ci_report(Path::new("."), true);
    apply_compare_back_result(&mut report, &compare);
    report
}

#[cfg(test)]
pub(crate) fn ci_reference_data_summary_for_test(root: &Path) -> (CiReferenceData, Vec<String>) {
    let mut report = empty_ci_report(root, true);
    validate_reference_data(root, &mut report);
    (report.reference_data, report.errors)
}

#[cfg(test)]
pub(crate) fn ci_skips_bootstrap_public_schema_for_test(object: &DesiredStateObject) -> bool {
    ci_should_skip_bootstrap_public_schema(object, true)
}
