use crate::git::git_root;
use crate::repository::compare::compare_postgres_with_inventory;
use crate::repository::objects::*;
use crate::repository::sync::ExportSelection;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

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

pub(crate) fn plan_postgres_command(cwd: &Path, parsed: ParsedArgs) -> PlanReport {
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

    pub(crate) fn included_object_refs(&self) -> Vec<String> {
        self.includes.iter().map(ObjectRef::as_str).collect()
    }

    pub(crate) fn excluded_object_refs(&self) -> Vec<String> {
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

pub(crate) fn add_plan_candidates(
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

pub(crate) fn empty_plan_report() -> PlanReport {
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
