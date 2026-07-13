use crate::postgres::{
    inspect_postgres, invalid_postgres_url_message, is_postgres_connection_url,
    quote_postgres_identifier, resolve_postgres_url,
};
use crate::repository::plan::{analyze_table_difference, parse_repository_table_columns};
use crate::repository::{
    ensure_database_object_path, plan_postgres_with_inventory, DependencyWarning, ExportSelection,
    ObjectRef, PlanColumn, PlanItem, PlanSelection, TableDifferenceAnalysis,
};
use crate::*;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ReleaseReport {
    pub command: CommandKind,
    pub success: bool,
    pub repository_path: String,
    pub git_root: Option<String>,
    pub is_git_repository: bool,
    pub branch: Option<String>,
    pub working_tree_status: WorkingTreeStatus,
    pub is_dirty: bool,
    pub database_type: String,
    pub release_name: String,
    pub release_scope: String,
    pub dry_run: bool,
    pub selected_schemas: Vec<String>,
    pub selected_tables: Vec<String>,
    pub included_objects: Vec<String>,
    pub excluded_objects: Vec<String>,
    pub plan_items: Vec<PlanItem>,
    pub blocked_items: Vec<PlanItem>,
    pub dependency_warnings: Vec<DependencyWarning>,
    pub planned_artifacts: Vec<String>,
    pub created_artifacts: Vec<String>,
    pub risk_level: String,
    pub risk_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

pub(crate) fn release_postgres_command(cwd: &Path, parsed: ParsedArgs) -> ReleaseReport {
    let mut report = empty_release_report(parsed.dry_run);
    let release_name = match parsed.release_name.clone() {
        Some(name) => name,
        None => {
            report
                .errors
                .push("Release name is required. Provide --name <release-name>.".to_string());
            return report;
        }
    };
    report.release_name = release_name.clone();

    if let Err(error) = release_slug(&release_name) {
        report.errors.push(error);
        return report;
    }

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
            report.release_scope = selection.scope_name();
            report.selected_schemas = selection.selected_schemas();
            report.selected_tables = selection.selected_tables();
            report.errors.push(error);
            return report;
        }
    };

    report.release_scope = selection.scope_name();
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
        Ok(inventory) => release_postgres_with_inventory(
            cwd,
            &inventory,
            &selection,
            &plan_selection,
            &release_name,
            parsed.dry_run,
        ),
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
            report
        }
    }
}

pub fn release_postgres_with_inventory(
    cwd: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
    plan_selection: &PlanSelection,
    release_name: &str,
    dry_run: bool,
) -> ReleaseReport {
    let mut report = empty_release_report(dry_run);
    report.release_name = release_name.to_string();
    report.release_scope = selection.scope_name();
    report.selected_schemas = selection.selected_schemas();
    report.selected_tables = selection.selected_tables();
    report.included_objects = plan_selection.included_object_refs();
    report.excluded_objects = plan_selection.excluded_object_refs();

    let slug = match release_slug(release_name) {
        Ok(slug) => slug,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    let project = status_report(cwd, CommandKind::ReleasePostgres);
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
    if project.is_dirty && !dry_run {
        report.errors.push(
            "Release artifact generation is blocked because the working tree has changes. Commit/stash changes or use --dry-run."
                .to_string(),
        );
        return report;
    }

    let root = PathBuf::from(project.git_root.expect("git root exists for repository"));
    if !root.join("database/releases").is_dir() {
        report.errors.push(
            "database/releases is missing. Run dbstate init before generating release artifacts."
                .to_string(),
        );
        return report;
    }

    let plan = plan_postgres_with_inventory(cwd, inventory, selection, plan_selection);
    report.included_objects = plan.included_objects.clone();
    report.excluded_objects = plan.excluded_objects.clone();
    report.plan_items = plan.plan_items.clone();
    report.blocked_items = plan.blocked_items.clone();
    report.dependency_warnings = plan.dependency_warnings.clone();
    report.warnings = plan.warnings.clone();
    if report.is_dirty && dry_run {
        report.warnings.push(
            "Release artifact generation will be blocked because the working tree has changes. Commit or stash changes before generating release artifacts."
                .to_string(),
        );
    }
    report.errors = plan.errors.clone();
    report.deferred_object_types = plan.deferred_object_types.clone();
    report.repository_path = plan.repository_path.clone();
    report.git_root = plan.git_root.clone();
    report.is_git_repository = plan.is_git_repository;
    report.branch = plan.branch.clone();
    report.working_tree_status = plan.working_tree_status;
    report.is_dirty = plan.is_dirty;

    if !report.errors.is_empty() {
        update_release_risk(&mut report);
        return report;
    }
    if !report.blocked_items.is_empty() {
        report.errors.push(
            "Release artifact generation is blocked because selected plan items have dependency blockers."
                .to_string(),
        );
        update_release_risk(&mut report);
        return report;
    }

    let artifacts = match plan_release_artifact_paths(&root, &slug) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    report.planned_artifacts = artifacts.relative_paths();
    update_release_risk(&mut report);

    if dry_run {
        report.success = report.errors.is_empty();
        return report;
    }

    let mut artifact_report = report.clone();
    artifact_report.success = true;

    let sql = match render_release_sql(&root, &artifact_report, &artifacts, inventory) {
        Ok(sql) => sql,
        Err(error) => {
            report.errors.push(error);
            update_release_risk(&mut report);
            return report;
        }
    };
    let summary = render_release_summary(&artifact_report, &artifacts);
    let risk = render_release_risk_json(&artifact_report, &artifacts);
    let manifest = render_release_manifest_json(
        &artifact_report,
        &artifacts,
        &[
            (&artifacts.sql, &sql),
            (&artifacts.summary, &summary),
            (&artifacts.risk, &risk),
        ],
    );

    let generated = [
        (&artifacts.sql, sql),
        (&artifacts.summary, summary),
        (&artifacts.risk, risk),
        (&artifacts.manifest, manifest),
    ];
    for (relative_path, _) in &generated {
        if let Err(error) = ensure_database_release_path(relative_path) {
            report.errors.push(error);
        }
        if root.join(relative_path).exists() {
            report.errors.push(format!(
                "Refusing to overwrite existing release artifact {relative_path}."
            ));
        }
    }
    if !report.errors.is_empty() {
        update_release_risk(&mut report);
        return report;
    }

    for (relative_path, content) in generated {
        let target = root.join(relative_path);
        if let Err(error) = fs::write(&target, content) {
            report
                .errors
                .push(format!("Could not write {relative_path}: {error}"));
            continue;
        }
        report.created_artifacts.push(relative_path.to_string());
    }

    report.success = report.errors.is_empty();
    update_release_risk(&mut report);
    report
}

#[derive(Debug, Clone)]
struct ReleaseArtifactPaths {
    sql: String,
    summary: String,
    risk: String,
    manifest: String,
}

impl ReleaseArtifactPaths {
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

fn release_slug(release_name: &str) -> Result<String, String> {
    let trimmed = release_name.trim();
    if trimmed.is_empty() {
        return Err("Release name cannot be empty.".to_string());
    }
    if trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
        || trimmed.contains(' ')
        || trimmed.contains('\t')
    {
        return Err(
            "Release name must use only letters, numbers, hyphen, or underscore.".to_string(),
        );
    }
    let slug = trimmed.to_ascii_lowercase();
    if slug
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        Ok(slug)
    } else {
        Err("Release name must use only letters, numbers, hyphen, or underscore.".to_string())
    }
}

fn plan_release_artifact_paths(root: &Path, slug: &str) -> Result<ReleaseArtifactPaths, String> {
    for sequence in 1..=9999 {
        let prefix = format!("{sequence:04}_{slug}");
        let artifacts = ReleaseArtifactPaths {
            sql: format!("database/releases/{prefix}.sql"),
            summary: format!("database/releases/{prefix}.summary.md"),
            risk: format!("database/releases/{prefix}.risk.json"),
            manifest: format!("database/releases/{prefix}.manifest.json"),
        };
        for relative_path in artifacts.relative_paths() {
            ensure_database_release_path(&relative_path)?;
        }
        if !root.join(&artifacts.sql).exists()
            && !root.join(&artifacts.summary).exists()
            && !root.join(&artifacts.risk).exists()
            && !root.join(&artifacts.manifest).exists()
        {
            return Ok(artifacts);
        }
    }
    Err("Could not choose a release artifact sequence from 0001 to 9999.".to_string())
}

fn ensure_database_release_path(relative_path: &str) -> Result<(), String> {
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

#[derive(Debug, Clone, Default)]
struct ReleaseCounts {
    create: usize,
    review_required: usize,
    blocked: usize,
    skipped: usize,
    in_sync: usize,
    database_only: usize,
}

fn update_release_risk(report: &mut ReleaseReport) {
    let (level, reasons) = release_risk(report);
    report.risk_level = level;
    report.risk_reasons = reasons;
}

fn release_risk(report: &ReleaseReport) -> (String, Vec<String>) {
    let counts = release_counts(report);
    let mut reasons = Vec::new();
    if !report.blocked_items.is_empty() || !report.errors.is_empty() {
        if !report.blocked_items.is_empty() {
            reasons.push(format!(
                "{} blocked item(s) prevent release artifact generation.",
                report.blocked_items.len()
            ));
        }
        if !report.errors.is_empty() {
            reasons.push(format!("{} error(s) are present.", report.errors.len()));
        }
        return ("blocked".to_string(), reasons);
    }
    if counts.review_required > 5 || counts.database_only > 0 || counts.skipped > 0 {
        if counts.review_required > 5 {
            reasons.push(format!(
                "{} review-required item(s) need manual assessment.",
                counts.review_required
            ));
        }
        if counts.database_only > 0 {
            reasons.push(format!(
                "{} database-only item(s) are not dropped by DbState.",
                counts.database_only
            ));
        }
        if counts.skipped > 0 {
            reasons.push(format!(
                "{} skipped item(s) require review.",
                counts.skipped
            ));
        }
        return ("high".to_string(), reasons);
    }
    if counts.review_required > 0
        || !report.dependency_warnings.is_empty()
        || !report.warnings.is_empty()
    {
        if counts.review_required > 0 {
            reasons.push(format!(
                "{} review-required item(s) exist.",
                counts.review_required
            ));
        }
        if !report.dependency_warnings.is_empty() {
            reasons.push(format!(
                "{} dependency warning(s) exist.",
                report.dependency_warnings.len()
            ));
        }
        if !report.warnings.is_empty() {
            reasons.push(format!("{} warning(s) exist.", report.warnings.len()));
        }
        return ("medium".to_string(), reasons);
    }
    reasons.push("Only safe create statements are planned.".to_string());
    ("low".to_string(), reasons)
}

fn release_counts(report: &ReleaseReport) -> ReleaseCounts {
    let mut counts = ReleaseCounts {
        blocked: report.blocked_items.len(),
        ..ReleaseCounts::default()
    };
    for item in &report.plan_items {
        match item.plan_intent.as_str() {
            "createInDatabaseLater" => counts.create += 1,
            "updateDatabaseLater" => counts.review_required += 1,
            "reviewDatabaseOnly" => counts.database_only += 1,
            _ => counts.skipped += 1,
        }
        if item.compare_classification == "inSync" {
            counts.in_sync += 1;
        }
        if item.compare_classification == "skipped" {
            counts.skipped += 1;
        }
    }
    counts
}

fn release_object_type_counts(report: &ReleaseReport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for item in &report.plan_items {
        *counts.entry(item.object_type.clone()).or_insert(0) += 1;
    }
    for item in &report.blocked_items {
        *counts.entry(item.object_type.clone()).or_insert(0) += 1;
    }
    counts
}

fn render_release_sql(
    root: &Path,
    report: &ReleaseReport,
    artifacts: &ReleaseArtifactPaths,
    inventory: &PostgresInventory,
) -> Result<String, String> {
    let mut sql = String::new();
    writeln!(sql, "-- DbState Release Artifact").ok();
    writeln!(sql, "-- Release: {}", report.release_name).ok();
    writeln!(sql, "-- Sequence: {}", artifacts.sequence()).ok();
    writeln!(sql, "-- Artifact: {}", artifacts.sql).ok();
    writeln!(sql, "-- Generated by: DbState PostgreSQL v0.1").ok();
    writeln!(
        sql,
        "-- Safety: Review-only. DbState does not execute this SQL."
    )
    .ok();
    writeln!(sql, "-- Source: repository desired state").ok();
    writeln!(sql, "-- Target: PostgreSQL database").ok();
    writeln!(sql, "-- Direct apply: not available through DbState.").ok();
    writeln!(sql, "-- Scope: {}", report.release_scope).ok();
    writeln!(sql, "-- Risk level: {}", report.risk_level).ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Summary").ok();
    writeln!(sql, "-- Selected objects:").ok();
    if report.plan_items.is_empty() {
        writeln!(sql, "--   none").ok();
    } else {
        for item in &report.plan_items {
            writeln!(sql, "--   {}", item.object_ref).ok();
        }
    }
    let counts = release_counts(report);
    writeln!(sql, "-- Counts:").ok();
    writeln!(sql, "--   create: {}", counts.create).ok();
    writeln!(sql, "--   review required: {}", counts.review_required).ok();
    writeln!(sql, "--   database only: {}", counts.database_only).ok();
    writeln!(sql, "--   skipped: {}", counts.skipped).ok();
    writeln!(sql, "--   blocked: {}", counts.blocked).ok();
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Creates").ok();
    writeln!(
        sql,
        "-- WARNING: Full dependency ordering is not implemented in Private Beta."
    )
    .ok();
    let mut creates: Vec<&PlanItem> = report
        .plan_items
        .iter()
        .filter(|item| item.plan_intent == "createInDatabaseLater")
        .collect();
    creates.sort_by_key(release_object_order);
    if creates.is_empty() {
        writeln!(sql, "-- none").ok();
    }
    for item in creates {
        writeln!(sql, "-- Object: {}", item.object_ref).ok();
        sql.push_str(&render_create_later_sql(root, item)?);
        writeln!(sql).ok();
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Review Required").ok();
    let mut review_items: Vec<&PlanItem> = report
        .plan_items
        .iter()
        .filter(|item| item.plan_intent != "createInDatabaseLater")
        .collect();
    review_items.sort_by_key(release_object_order);
    if review_items.is_empty() {
        writeln!(sql, "-- none").ok();
    }
    for item in review_items {
        writeln!(sql, "-- Object: {}", item.object_ref).ok();
        match item.plan_intent.as_str() {
            "updateDatabaseLater" => {
                sql.push_str(&render_update_database_later_sql(root, item, inventory)?);
            }
            "reviewDatabaseOnly" => {
                writeln!(
                    sql,
                    "-- REVIEW REQUIRED: object exists only in target database. DbState does not generate DROP."
                )
                .ok();
            }
            _ => {
                writeln!(
                    sql,
                    "-- REVIEW REQUIRED: no SQL generated for object with intent {}.",
                    item.plan_intent
                )
                .ok();
            }
        }
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Blocked Items").ok();
    if report.blocked_items.is_empty() {
        writeln!(sql, "-- none").ok();
    } else {
        for item in &report.blocked_items {
            writeln!(
                sql,
                "-- BLOCKED: {} with intent {}.",
                item.object_ref, item.plan_intent
            )
            .ok();
        }
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    writeln!(sql, "-- BEGIN REVIEW SECTION: Deferred Object Types").ok();
    if report.deferred_object_types.is_empty() {
        writeln!(sql, "-- none").ok();
    } else {
        for object_type in &report.deferred_object_types {
            writeln!(sql, "-- DEFERRED: {object_type}").ok();
        }
    }
    writeln!(sql, "-- END REVIEW SECTION").ok();
    writeln!(sql).ok();

    if !report.dependency_warnings.is_empty() {
        writeln!(sql, "-- BEGIN REVIEW SECTION: Warnings").ok();
        writeln!(sql, "-- Dependency warnings:").ok();
        for warning in &report.dependency_warnings {
            writeln!(
                sql,
                "--   [{}] {}: {}",
                warning.severity, warning.warning_type, warning.message
            )
            .ok();
        }
        writeln!(sql, "-- END REVIEW SECTION").ok();
        writeln!(sql).ok();
    }

    Ok(sql)
}

fn release_object_order(item: &&PlanItem) -> (u8, String) {
    let order = match item.object_type.as_str() {
        "schema" => 1,
        "extension" => 2,
        "enum" => 3,
        "sequence" => 4,
        "table" => 5,
        "index" => 6,
        "view" => 7,
        _ => 99,
    };
    (order, item.object_ref.clone())
}

fn render_create_later_sql(root: &Path, item: &PlanItem) -> Result<String, String> {
    let object_ref = ObjectRef::parse(&item.object_ref)?;
    match object_ref {
        ObjectRef::Schema(schema) => Ok(format!(
            "CREATE SCHEMA IF NOT EXISTS {};\n",
            quote_postgres_identifier(&schema)
        )),
        ObjectRef::Table { .. } => {
            ensure_database_object_path(&item.relative_path)?;
            let content = fs::read_to_string(root.join(&item.relative_path))
                .map_err(|error| format!("Could not read {}: {error}", item.relative_path))?;
            Ok(content.replacen("CREATE TABLE ", "CREATE TABLE IF NOT EXISTS ", 1))
        }
        ObjectRef::Extension(_)
        | ObjectRef::Enum { .. }
        | ObjectRef::Sequence { .. }
        | ObjectRef::Index { .. }
        | ObjectRef::View { .. } => {
            ensure_database_object_path(&item.relative_path)?;
            fs::read_to_string(root.join(&item.relative_path))
                .map_err(|error| format!("Could not read {}: {error}", item.relative_path))
        }
    }
}

fn render_update_database_later_sql(
    root: &Path,
    item: &PlanItem,
    inventory: &PostgresInventory,
) -> Result<String, String> {
    let object_ref = ObjectRef::parse(&item.object_ref)?;
    let ObjectRef::Table { schema, table } = object_ref else {
        return Ok(
            "-- REVIEW REQUIRED: object differs; automatic ALTER is not generated in Private Beta.\n"
                .to_string(),
        );
    };

    ensure_database_object_path(&item.relative_path)?;
    let content = fs::read_to_string(root.join(&item.relative_path))
        .map_err(|error| format!("Could not read {}: {error}", item.relative_path))?;
    let Some(repository_columns) = parse_repository_table_columns(&content) else {
        return Ok(format!(
            "-- REVIEW REQUIRED: table {schema}.{table} differs; repository table DDL is not in the supported additive-column review shape.\n"
        ));
    };
    let database_columns: Vec<PlanColumn> = inventory
        .columns
        .iter()
        .filter(|column| column.schema_name == schema && column.table_name == table)
        .map(PlanColumn::from)
        .collect();
    if database_columns.is_empty() {
        return Ok(format!(
            "-- REVIEW REQUIRED: table {schema}.{table} differs; target database column model was not available.\n"
        ));
    }

    Ok(render_additive_table_difference_sql(
        &schema,
        &table,
        &analyze_table_difference(&repository_columns, &database_columns),
    ))
}

fn render_additive_table_difference_sql(
    schema: &str,
    table: &str,
    analysis: &TableDifferenceAnalysis,
) -> String {
    let mut sql = String::new();
    if let TableDifferenceAnalysis::NotClearlyAdditive(manual_reasons) = analysis {
        writeln!(
            sql,
            "-- REVIEW REQUIRED: table {schema}.{table} differs; difference is not clearly additive."
        )
        .ok();
        for reason in manual_reasons {
            writeln!(sql, "-- Manual review reason: {reason}.").ok();
        }
        return sql;
    }

    if matches!(analysis, TableDifferenceAnalysis::NoSafeSuggestion) {
        writeln!(
            sql,
            "-- REVIEW REQUIRED: table {schema}.{table} differs; no safe additive column suggestion was identified."
        )
        .ok();
        return sql;
    }

    let TableDifferenceAnalysis::Additive {
        safe_adds,
        unsafe_adds,
    } = analysis
    else {
        return sql;
    };

    for column in safe_adds {
        writeln!(sql, "-- Review-only additive column suggestion.").ok();
        writeln!(sql, "-- DbState does not execute this SQL.").ok();
        writeln!(sql, "-- Review before applying manually outside DbState.").ok();
        writeln!(
            sql,
            "ALTER TABLE {}.{}",
            quote_postgres_identifier(schema),
            quote_postgres_identifier(table)
        )
        .ok();
        writeln!(sql, "    ADD COLUMN {};", render_column_definition(column)).ok();
    }

    for column in unsafe_adds {
        writeln!(
            sql,
            "-- REVIEW REQUIRED: additive column {} is NOT NULL with no default; manual review is required and DbState did not generate ADD COLUMN because applying it to a populated table can fail.",
            quote_postgres_identifier(&column.name)
        )
        .ok();
    }

    sql
}

fn render_column_definition(column: &PlanColumn) -> String {
    let mut definition = format!(
        "{} {}",
        quote_postgres_identifier(&column.name),
        column.data_type
    );
    if column.has_default {
        if let Some(default_expression) = &column.default_expression {
            write!(definition, " DEFAULT {default_expression}").ok();
        }
    }
    if !column.is_nullable {
        definition.push_str(" NOT NULL");
    }
    definition
}

fn render_release_summary(report: &ReleaseReport, artifacts: &ReleaseArtifactPaths) -> String {
    let mut summary = String::new();
    writeln!(summary, "# DbState PostgreSQL Release Summary").ok();
    writeln!(summary).ok();
    writeln!(summary, "- Release name: {}", report.release_name).ok();
    writeln!(summary, "- Release sequence: {}", artifacts.sequence()).ok();
    writeln!(summary, "- Command: {}", report.command.as_str()).ok();
    writeln!(summary, "- Scope: {}", report.release_scope).ok();
    writeln!(summary, "- Dry run: {}", report.dry_run).ok();
    writeln!(summary, "- Risk level: {}", report.risk_level).ok();
    writeln!(summary, "- Repository path: {}", report.repository_path).ok();
    writeln!(
        summary,
        "- Git root: {}",
        report.git_root.as_deref().unwrap_or("")
    )
    .ok();
    writeln!(
        summary,
        "- Branch: {}",
        report.branch.as_deref().unwrap_or("unknown")
    )
    .ok();
    writeln!(
        summary,
        "- Working tree: {}",
        report.working_tree_status.as_str()
    )
    .ok();
    writeln!(summary).ok();
    writeln!(summary, "## Artifacts").ok();
    for artifact in artifacts.relative_paths() {
        writeln!(summary, "- {artifact}").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Object Counts By Status").ok();
    let counts = release_counts(report);
    writeln!(summary, "- create: {}", counts.create).ok();
    writeln!(summary, "- review required: {}", counts.review_required).ok();
    writeln!(summary, "- blocked: {}", counts.blocked).ok();
    writeln!(summary, "- skipped: {}", counts.skipped).ok();
    writeln!(summary, "- in sync: {}", counts.in_sync).ok();
    writeln!(summary, "- database only: {}", counts.database_only).ok();
    writeln!(summary).ok();
    writeln!(summary, "## Object Counts By Type").ok();
    let type_counts = release_object_type_counts(report);
    if type_counts.is_empty() {
        writeln!(summary, "- none").ok();
    } else {
        for (object_type, count) in type_counts {
            writeln!(summary, "- {object_type}: {count}").ok();
        }
    }
    writeln!(summary).ok();
    writeln!(summary, "## Risk Summary").ok();
    writeln!(summary, "- Risk level: {}", report.risk_level).ok();
    write_markdown_list(&mut summary, &report.risk_reasons);
    writeln!(summary).ok();
    writeln!(summary, "## Included Objects").ok();
    write_markdown_list(&mut summary, &report.included_objects);
    writeln!(summary).ok();
    writeln!(summary, "## Excluded Objects").ok();
    write_markdown_list(&mut summary, &report.excluded_objects);
    writeln!(summary).ok();
    writeln!(summary, "## Plan Items").ok();
    for item in &report.plan_items {
        writeln!(
            summary,
            "- {}: {}, {}, intent {}",
            item.object_ref, item.object_type, item.compare_classification, item.plan_intent
        )
        .ok();
    }
    if report.plan_items.is_empty() {
        writeln!(summary, "- none").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Blocked Items").ok();
    for item in &report.blocked_items {
        writeln!(summary, "- {}: {}", item.object_ref, item.plan_intent).ok();
    }
    if report.blocked_items.is_empty() {
        writeln!(summary, "- none").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Skipped Items").ok();
    let skipped_items: Vec<String> = report
        .plan_items
        .iter()
        .filter(|item| item.compare_classification == "skipped")
        .map(|item| item.object_ref.clone())
        .collect();
    write_markdown_list(&mut summary, &skipped_items);
    writeln!(summary).ok();
    writeln!(summary, "## Dependency Warnings").ok();
    for warning in &report.dependency_warnings {
        writeln!(
            summary,
            "- [{}] {}: {}",
            warning.severity, warning.warning_type, warning.message
        )
        .ok();
    }
    if report.dependency_warnings.is_empty() {
        writeln!(summary, "- none").ok();
    }
    writeln!(summary).ok();
    writeln!(summary, "## Deferred Object Types").ok();
    write_markdown_list(&mut summary, &report.deferred_object_types);
    writeln!(summary).ok();
    writeln!(summary, "## Warnings").ok();
    write_markdown_list(&mut summary, &report.warnings);
    writeln!(summary).ok();
    writeln!(summary, "## Safety").ok();
    writeln!(
        summary,
        "DbState generated these artifacts for review. DbState did not execute SQL, did not apply changes, did not mutate PostgreSQL, and did not stage, commit, push, pull, fetch, or tag Git changes."
    )
    .ok();
    writeln!(summary).ok();
    writeln!(summary, "## Reviewer Checklist").ok();
    writeln!(summary, "1. Review all REVIEW REQUIRED comments.").ok();
    writeln!(summary, "2. Review blocked and skipped items.").ok();
    writeln!(summary, "3. Review deferred object type warnings.").ok();
    writeln!(summary, "4. Confirm no destructive SQL is present.").ok();
    writeln!(summary, "5. Confirm object ordering is acceptable.").ok();
    writeln!(
        summary,
        "6. Have a DBA or responsible engineer review before any manual execution outside DbState."
    )
    .ok();
    writeln!(summary).ok();
    writeln!(summary, "## Current Limitations").ok();
    writeln!(
        summary,
        "- Full dependency ordering is not implemented in Private Beta."
    )
    .ok();
    writeln!(
        summary,
        "- Changed tables may include review-only additive ADD COLUMN suggestions when safe; other changed objects remain manual review comments."
    )
    .ok();
    writeln!(
        summary,
        "- Database-only objects produce review comments instead of DROP statements."
    )
    .ok();
    writeln!(
        summary,
        "- Unsupported and deferred objects remain review-only."
    )
    .ok();
    summary
}

fn write_markdown_list(markdown: &mut String, values: &[String]) {
    if values.is_empty() {
        writeln!(markdown, "- none").ok();
        return;
    }
    for value in values {
        writeln!(markdown, "- {value}").ok();
    }
}

fn render_release_risk_json(report: &ReleaseReport, artifacts: &ReleaseArtifactPaths) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", report.command.as_str(), true);
    write_json_bool_field(&mut json, "success", report.success);
    write_json_string_field(&mut json, "releaseName", &report.release_name, false);
    write_json_string_field(&mut json, "releaseSequence", &artifacts.sequence(), false);
    write_json_string_field(&mut json, "databaseType", &report.database_type, false);
    write_json_array_field(&mut json, "generatedArtifacts", &artifacts.relative_paths());
    write_release_repository_context_field(&mut json, report);
    write_json_string_field(&mut json, "scope", &report.release_scope, false);
    write_release_counts_field(&mut json, "counts", &release_counts(report));
    write_object_type_counts_field(
        &mut json,
        "objectTypeCounts",
        &release_object_type_counts(report),
    );
    write_json_string_field(&mut json, "riskLevel", &report.risk_level, false);
    write_json_array_field(&mut json, "riskReasons", &report.risk_reasons);
    write_json_array_field(&mut json, "selectedObjects", &report.included_objects);
    write_plan_item_array_field(&mut json, "planItems", &report.plan_items);
    write_plan_item_array_field(&mut json, "blockedItems", &report.blocked_items);
    write_plan_item_array_field(&mut json, "skippedItems", &release_skipped_items(report));
    write_dependency_warning_array_field(
        &mut json,
        "dependencyWarnings",
        &report.dependency_warnings,
    );
    write_json_array_field(
        &mut json,
        "deferredObjectTypes",
        &report.deferred_object_types,
    );
    write_json_bool_field(&mut json, "destructiveSqlGenerated", false);
    write_json_bool_field(&mut json, "directApplyAvailable", false);
    write_json_bool_field(&mut json, "generatedSqlExecutionSupported", false);
    write_json_bool_field(&mut json, "databaseMutationPerformed", false);
    write_json_bool_field(&mut json, "gitMutationPerformed", false);
    write_json_bool_field(&mut json, "credentialPersistencePerformed", false);
    write_json_array_field(&mut json, "warnings", &report.warnings);
    write_json_array_field(&mut json, "errors", &report.errors);
    json.push('}');
    json
}

fn render_release_manifest_json(
    report: &ReleaseReport,
    artifacts: &ReleaseArtifactPaths,
    generated: &[(&String, &String)],
) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "releaseName", &report.release_name, true);
    write_json_string_field(&mut json, "releaseSequence", &artifacts.sequence(), false);
    write_json_string_field(
        &mut json,
        "dbstateVersion",
        env!("CARGO_PKG_VERSION"),
        false,
    );
    write_release_manifest_artifacts_field(&mut json, "artifacts", generated, artifacts);
    write_json_bool_field(&mut json, "directApplyAvailable", false);
    write_json_bool_field(&mut json, "generatedSqlExecutionSupported", false);
    json.push('}');
    json
}

fn release_skipped_items(report: &ReleaseReport) -> Vec<PlanItem> {
    report
        .plan_items
        .iter()
        .filter(|item| item.compare_classification == "skipped")
        .cloned()
        .collect()
}

fn write_release_repository_context_field(json: &mut String, report: &ReleaseReport) {
    json.push(',');
    write!(json, "\"repositoryContext\":{{").ok();
    write_json_string_field(json, "repositoryPath", &report.repository_path, true);
    write_json_optional_string_field(json, "gitRoot", report.git_root.as_deref());
    write_json_bool_field(json, "isGitRepository", report.is_git_repository);
    write_json_optional_string_field(json, "branch", report.branch.as_deref());
    write_json_string_field(
        json,
        "workingTreeStatus",
        report.working_tree_status.as_str(),
        false,
    );
    write_json_bool_field(json, "isDirty", report.is_dirty);
    json.push('}');
}

fn write_release_counts_field(json: &mut String, name: &str, counts: &ReleaseCounts) {
    json.push(',');
    write!(
        json,
        "\"{}\":{{\"create\":{},\"reviewRequired\":{},\"blocked\":{},\"skipped\":{},\"inSync\":{},\"databaseOnly\":{}}}",
        escape_json(name),
        counts.create,
        counts.review_required,
        counts.blocked,
        counts.skipped,
        counts.in_sync,
        counts.database_only
    )
    .ok();
}

fn write_object_type_counts_field(json: &mut String, name: &str, counts: &BTreeMap<String, usize>) {
    json.push(',');
    write!(json, "\"{}\":{{", escape_json(name)).ok();
    for (index, (object_type, count)) in counts.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(json, "\"{}\":{}", escape_json(object_type), count).ok();
    }
    json.push('}');
}

fn write_release_manifest_artifacts_field(
    json: &mut String,
    name: &str,
    generated: &[(&String, &String)],
    artifacts: &ReleaseArtifactPaths,
) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, (path, content)) in generated.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        let artifact_type = if *path == &artifacts.sql {
            "sql"
        } else if *path == &artifacts.summary {
            "summary"
        } else if *path == &artifacts.risk {
            "risk"
        } else {
            "manifest"
        };
        json.push('{');
        write_json_string_field(json, "artifactType", artifact_type, true);
        write_json_string_field(json, "relativePath", path, false);
        write_json_i64_field(json, "sizeBytes", content.len() as i64, false);
        json.push('}');
    }
    if !generated.is_empty() {
        json.push(',');
    }
    json.push('{');
    write_json_string_field(json, "artifactType", "manifest", true);
    write_json_string_field(json, "relativePath", &artifacts.manifest, false);
    write_json_i64_field(json, "sizeBytes", 0, false);
    json.push('}');
    json.push(']');
}

pub(crate) fn empty_release_report(dry_run: bool) -> ReleaseReport {
    ReleaseReport {
        command: CommandKind::ReleasePostgres,
        success: false,
        repository_path: String::new(),
        git_root: None,
        is_git_repository: false,
        branch: None,
        working_tree_status: WorkingTreeStatus::Unknown,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        release_name: String::new(),
        release_scope: "<none>".to_string(),
        dry_run,
        selected_schemas: Vec::new(),
        selected_tables: Vec::new(),
        included_objects: Vec::new(),
        excluded_objects: Vec::new(),
        plan_items: Vec::new(),
        blocked_items: Vec::new(),
        dependency_warnings: Vec::new(),
        planned_artifacts: Vec::new(),
        created_artifacts: Vec::new(),
        risk_level: "medium".to_string(),
        risk_reasons: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

impl ReleaseReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(text, "Database type: {}", self.database_type).ok();
        writeln!(text, "Release name: {}", self.release_name).ok();
        writeln!(text, "Release scope: {}", self.release_scope).ok();
        writeln!(text, "Dry run: {}", self.dry_run).ok();
        writeln!(text, "Working tree: {}", self.working_tree_status.as_str()).ok();
        writeln!(text, "Risk level: {}", self.risk_level).ok();
        write_path_list(&mut text, "Included objects", &self.included_objects);
        write_path_list(&mut text, "Excluded objects", &self.excluded_objects);
        write_plan_item_text_list(&mut text, "Plan items", &self.plan_items);
        write_plan_item_text_list(&mut text, "Blocked items", &self.blocked_items);
        write_path_list(&mut text, "Planned artifacts", &self.planned_artifacts);
        write_path_list(&mut text, "Created artifacts", &self.created_artifacts);
        writeln!(text, "Dependency warnings:").ok();
        for warning in &self.dependency_warnings {
            writeln!(
                text,
                "  - [{}] {}: {}",
                warning.severity, warning.warning_type, warning.message
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
        write_json_string_field(&mut json, "releaseName", &self.release_name, false);
        write_json_string_field(&mut json, "releaseScope", &self.release_scope, false);
        write_json_bool_field(&mut json, "dryRun", self.dry_run);
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
        write_json_array_field(&mut json, "plannedArtifacts", &self.planned_artifacts);
        write_json_array_field(&mut json, "createdArtifacts", &self.created_artifacts);
        write_json_string_field(&mut json, "riskLevel", &self.risk_level, false);
        write_json_array_field(&mut json, "riskReasons", &self.risk_reasons);
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
