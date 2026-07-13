use crate::git::git_root;
use crate::postgres::{
    invalid_postgres_url_message, is_postgres_connection_url, resolve_postgres_url,
};
use crate::repository::compare::compare_postgres_with_inventory;
use crate::repository::objects::*;
use crate::repository::sync::ExportSelection;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
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
    pub operation_kind: String,
    pub operation_label: String,
    pub safety_badge: String,
    pub safety_level: String,
    pub operation_explanation: String,
    pub operation_reasons: Vec<String>,
    pub selected: bool,
    pub blocked: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlanOperationMetadata {
    pub(crate) operation_kind: String,
    pub(crate) operation_label: String,
    pub(crate) safety_badge: String,
    pub(crate) safety_level: String,
    pub(crate) operation_explanation: String,
    pub(crate) operation_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanColumn {
    pub(crate) name: String,
    pub(crate) data_type: String,
    pub(crate) is_nullable: bool,
    pub(crate) has_default: bool,
    pub(crate) default_expression: Option<String>,
}

impl From<&ColumnInfo> for PlanColumn {
    fn from(column: &ColumnInfo) -> Self {
        Self {
            name: column.column_name.clone(),
            data_type: column.data_type.clone(),
            is_nullable: column.is_nullable,
            has_default: column.has_default,
            default_expression: column.default_expression.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TableDifferenceAnalysis {
    Additive {
        safe_adds: Vec<PlanColumn>,
        unsafe_adds: Vec<PlanColumn>,
    },
    NotClearlyAdditive(Vec<String>),
    NoSafeSuggestion,
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

        let operation = classify_plan_operation(PlanOperationInput {
            root: &root,
            inventory,
            object_ref,
            relative_path,
            classification,
            intent: if blocked { "blocked" } else { intent },
            blocked,
            warnings: &item_warnings,
        });
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
            operation_kind: operation.operation_kind,
            operation_label: operation.operation_label,
            safety_badge: operation.safety_badge,
            safety_level: operation.safety_level,
            operation_explanation: operation.operation_explanation,
            operation_reasons: operation.operation_reasons,
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

struct PlanOperationInput<'a> {
    root: &'a Path,
    inventory: &'a PostgresInventory,
    object_ref: &'a ObjectRef,
    relative_path: &'a str,
    classification: &'a str,
    intent: &'a str,
    blocked: bool,
    warnings: &'a [String],
}

fn classify_plan_operation(input: PlanOperationInput<'_>) -> PlanOperationMetadata {
    if input.blocked || input.intent == "blocked" {
        return PlanOperationMetadata {
            operation_kind: "blocked".to_string(),
            operation_label: "Blocked".to_string(),
            safety_badge: "Blocked".to_string(),
            safety_level: "blocked".to_string(),
            operation_explanation:
                "Release artifact generation is blocked for this item until dependency warnings are resolved."
                    .to_string(),
            operation_reasons: if input.warnings.is_empty() {
                vec!["Selected plan item is blocked by dependency warnings.".to_string()]
            } else {
                input.warnings.to_vec()
            },
        };
    }

    if input.classification == "repoOnly" && input.intent == "createInDatabaseLater" {
        return PlanOperationMetadata {
            operation_kind: "createReviewSql".to_string(),
            operation_label: "Review SQL".to_string(),
            safety_badge: "Review SQL".to_string(),
            safety_level: "reviewOnly".to_string(),
            operation_explanation:
                "DbState can generate review-only SQL for this new repository object. DbState does not execute SQL."
                    .to_string(),
            operation_reasons: vec![
                "Object exists in repository desired state and is missing from the target database."
                    .to_string(),
            ],
        };
    }

    if input.classification == "databaseOnly" || input.intent == "reviewDatabaseOnly" {
        return PlanOperationMetadata {
            operation_kind: "databaseOnlyReview".to_string(),
            operation_label: "Database Only".to_string(),
            safety_badge: "Database Only".to_string(),
            safety_level: "manualReview".to_string(),
            operation_explanation:
                "Object exists only in the target database. DbState will not generate destructive SQL to remove database-only objects."
                    .to_string(),
            operation_reasons: vec![
                "DROP generation is not available in Private Beta.".to_string(),
            ],
        };
    }

    if input.classification == "repoDifferent" && input.intent == "updateDatabaseLater" {
        if let ObjectRef::Table { schema, table } = input.object_ref {
            return classify_table_difference_operation(
                input.root,
                input.inventory,
                schema,
                table,
                input.relative_path,
            );
        }
        return PlanOperationMetadata {
            operation_kind: "manualReviewRequired".to_string(),
            operation_label: "Manual Review".to_string(),
            safety_badge: "Manual Review".to_string(),
            safety_level: "manualReview".to_string(),
            operation_explanation:
                "Object differs, but DbState does not generate automatic update SQL for this object type in Private Beta."
                    .to_string(),
            operation_reasons: vec![
                "Only clearly additive table column differences can produce review SQL in Private Beta."
                    .to_string(),
            ],
        };
    }

    PlanOperationMetadata {
        operation_kind: "unsupportedOrDeferred".to_string(),
        operation_label: "Deferred".to_string(),
        safety_badge: "Deferred".to_string(),
        safety_level: "informational".to_string(),
        operation_explanation: "This object type or operation is not generated in Private Beta."
            .to_string(),
        operation_reasons: vec!["Unsupported or deferred release operation.".to_string()],
    }
}

fn classify_table_difference_operation(
    root: &Path,
    inventory: &PostgresInventory,
    schema: &str,
    table: &str,
    relative_path: &str,
) -> PlanOperationMetadata {
    let analysis = match table_difference_analysis_for_file(root, inventory, schema, table, relative_path)
    {
        Ok(analysis) => analysis,
        Err(reason) => {
            return PlanOperationMetadata {
                operation_kind: "manualReviewRequired".to_string(),
                operation_label: "Manual Review".to_string(),
                safety_badge: "Manual Review".to_string(),
                safety_level: "manualReview".to_string(),
                operation_explanation:
                    "Table differs, but DbState cannot safely generate review SQL for this difference in Private Beta."
                        .to_string(),
                operation_reasons: vec![reason],
            }
        }
    };
    match analysis {
        TableDifferenceAnalysis::Additive {
            safe_adds,
            unsafe_adds,
        } if !safe_adds.is_empty() => {
            let mut reasons: Vec<String> = safe_adds
                .iter()
                .map(|column| {
                    format!(
                        "Column {} is missing from the target database and is eligible for review-only ADD COLUMN SQL.",
                        quote_postgres_identifier(&column.name)
                    )
                })
                .collect();
            reasons.extend(unsafe_adds.iter().map(|column| {
                format!(
                    "Column {} is NOT NULL with no default and remains manual-review only because applying it to a populated table can fail.",
                    quote_postgres_identifier(&column.name)
                )
            }));
            PlanOperationMetadata {
                operation_kind: "additiveAddColumnReviewSql".to_string(),
                operation_label: "Additive ADD COLUMN".to_string(),
                safety_badge: "Additive ADD COLUMN".to_string(),
                safety_level: "reviewOnly".to_string(),
                operation_explanation:
                    "DbState can generate review-only ADD COLUMN SQL for eligible additive nullable/defaulted columns. Unsafe or ambiguous differences may still require manual review."
                        .to_string(),
                operation_reasons: reasons,
            }
        }
        TableDifferenceAnalysis::Additive { unsafe_adds, .. } if !unsafe_adds.is_empty() => {
            PlanOperationMetadata {
                operation_kind: "manualReviewRequired".to_string(),
                operation_label: "Manual Review".to_string(),
                safety_badge: "Manual Review".to_string(),
                safety_level: "manualReview".to_string(),
                operation_explanation:
                    "Table differs, but DbState cannot safely generate review SQL for this difference in Private Beta."
                        .to_string(),
                operation_reasons: unsafe_adds
                    .iter()
                    .map(|column| {
                        format!(
                            "Column {} is NOT NULL with no default; manual review is required because applying it to a populated table can fail.",
                            quote_postgres_identifier(&column.name)
                        )
                    })
                    .collect(),
            }
        }
        TableDifferenceAnalysis::NotClearlyAdditive(reasons) => PlanOperationMetadata {
            operation_kind: "manualReviewRequired".to_string(),
            operation_label: "Manual Review".to_string(),
            safety_badge: "Manual Review".to_string(),
            safety_level: "manualReview".to_string(),
            operation_explanation:
                "Table differs, but DbState cannot safely generate review SQL for this difference in Private Beta."
                    .to_string(),
            operation_reasons: reasons,
        },
        TableDifferenceAnalysis::NoSafeSuggestion => PlanOperationMetadata {
            operation_kind: "manualReviewRequired".to_string(),
            operation_label: "Manual Review".to_string(),
            safety_badge: "Manual Review".to_string(),
            safety_level: "manualReview".to_string(),
            operation_explanation:
                "Table differs, but DbState cannot safely generate review SQL for this difference in Private Beta."
                    .to_string(),
            operation_reasons: vec!["No safe additive column suggestion was identified.".to_string()],
        },
        TableDifferenceAnalysis::Additive { .. } => PlanOperationMetadata {
            operation_kind: "manualReviewRequired".to_string(),
            operation_label: "Manual Review".to_string(),
            safety_badge: "Manual Review".to_string(),
            safety_level: "manualReview".to_string(),
            operation_explanation:
                "Table differs, but DbState cannot safely generate review SQL for this difference in Private Beta."
                    .to_string(),
            operation_reasons: vec!["No safe additive column suggestion was identified.".to_string()],
        },
    }
}

pub(crate) fn table_difference_analysis_for_file(
    root: &Path,
    inventory: &PostgresInventory,
    schema: &str,
    table: &str,
    relative_path: &str,
) -> Result<TableDifferenceAnalysis, String> {
    ensure_database_object_path(relative_path)?;
    let content = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
    let Some(repository_columns) = parse_repository_table_columns(&content) else {
        return Err(format!(
            "Repository table DDL for {schema}.{table} is not in the supported additive-column review shape."
        ));
    };
    let database_columns: Vec<PlanColumn> = inventory
        .columns
        .iter()
        .filter(|column| column.schema_name == schema && column.table_name == table)
        .map(PlanColumn::from)
        .collect();
    if database_columns.is_empty() {
        return Err(format!(
            "Target database column model for {schema}.{table} was not available."
        ));
    }
    Ok(analyze_table_difference(
        &repository_columns,
        &database_columns,
    ))
}

pub(crate) fn analyze_table_difference(
    repository_columns: &[PlanColumn],
    database_columns: &[PlanColumn],
) -> TableDifferenceAnalysis {
    let repository_by_name: BTreeMap<&str, &PlanColumn> = repository_columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect();
    let database_by_name: BTreeMap<&str, &PlanColumn> = database_columns
        .iter()
        .map(|column| (column.name.as_str(), column))
        .collect();

    let mut manual_reasons = Vec::new();
    for database_column in database_columns {
        match repository_by_name.get(database_column.name.as_str()) {
            Some(repository_column)
                if existing_columns_match(repository_column, database_column) => {}
            Some(_) => manual_reasons.push(format!(
                "existing column {} differs between repository and target database; existing-column changes are manual-review only in Private Beta",
                quote_postgres_identifier(&database_column.name)
            )),
            None => manual_reasons.push(format!(
                "Target database column {} is not present in repository desired state",
                quote_postgres_identifier(&database_column.name)
            )),
        }
    }

    if !manual_reasons.is_empty() {
        return TableDifferenceAnalysis::NotClearlyAdditive(manual_reasons);
    }

    let mut safe_adds = Vec::new();
    let mut unsafe_adds = Vec::new();
    for repository_column in repository_columns {
        if database_by_name.contains_key(repository_column.name.as_str()) {
            continue;
        }
        if !repository_column.is_nullable && repository_column.default_expression.is_none() {
            unsafe_adds.push(repository_column.clone());
        } else {
            safe_adds.push(repository_column.clone());
        }
    }

    if safe_adds.is_empty() && unsafe_adds.is_empty() {
        TableDifferenceAnalysis::NoSafeSuggestion
    } else {
        TableDifferenceAnalysis::Additive {
            safe_adds,
            unsafe_adds,
        }
    }
}

fn existing_columns_match(repository_column: &PlanColumn, database_column: &PlanColumn) -> bool {
    repository_column
        .data_type
        .trim()
        .eq_ignore_ascii_case(database_column.data_type.trim())
        && repository_column.is_nullable == database_column.is_nullable
        && repository_column.has_default == database_column.has_default
        && normalize_default_expression(repository_column.default_expression.as_deref())
            == normalize_default_expression(database_column.default_expression.as_deref())
}

fn normalize_default_expression(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn parse_repository_table_columns(content: &str) -> Option<Vec<PlanColumn>> {
    let mut in_create_table = false;
    let mut columns = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with("CREATE TABLE ") {
            in_create_table = true;
            continue;
        }
        if !in_create_table {
            continue;
        }
        if line == ");" {
            break;
        }
        if line.is_empty() || line.starts_with("--") {
            continue;
        }
        columns.push(parse_repository_column_line(
            line.trim_end_matches(',').trim(),
        )?);
    }
    if columns.is_empty() {
        None
    } else {
        Some(columns)
    }
}

fn parse_repository_column_line(line: &str) -> Option<PlanColumn> {
    let (name, remainder) = parse_quoted_identifier(line)?;
    let mut definition = remainder.trim();
    let is_nullable = if let Some(without_not_null) = definition.strip_suffix(" NOT NULL") {
        definition = without_not_null.trim_end();
        false
    } else {
        true
    };
    let (data_type, default_expression) =
        if let Some((data_type, default_expression)) = definition.split_once(" DEFAULT ") {
            (
                data_type.trim(),
                Some(default_expression.trim().to_string()),
            )
        } else {
            (definition.trim(), None)
        };
    if data_type.is_empty() {
        return None;
    }
    Some(PlanColumn {
        name,
        data_type: data_type.to_string(),
        is_nullable,
        has_default: default_expression.is_some(),
        default_expression,
    })
}

fn parse_quoted_identifier(value: &str) -> Option<(String, &str)> {
    if !value.starts_with('"') {
        return None;
    }
    let mut index = 1;
    let mut identifier = String::new();
    while index < value.len() {
        let remaining = &value[index..];
        if remaining.starts_with("\"\"") {
            identifier.push('"');
            index += 2;
            continue;
        }
        if remaining.starts_with('"') {
            return Some((identifier, &value[index + 1..]));
        }
        let character = remaining.chars().next()?;
        identifier.push(character);
        index += character.len_utf8();
    }
    None
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
