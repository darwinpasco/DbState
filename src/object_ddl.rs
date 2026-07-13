use crate::git::git_root;
use crate::postgres::{
    inspect_postgres, invalid_postgres_url_message, is_postgres_connection_url,
    render_constraint_sql, render_enum_sql, render_extension_sql, render_function_sql,
    render_index_sql, render_schema_sql, render_sequence_sql, render_table_sql, render_trigger_sql,
    render_view_sql, ColumnInfo, ConstraintInfo, FunctionInfo, IndexInfo, TriggerInfo,
};
use crate::repository::{
    enum_file_path, extension_file_path, function_identity_slug, index_file_path,
    safe_file_component, schema_file_path, sequence_file_path, table_file_path, trigger_file_path,
    view_file_path,
};
use crate::service::{
    parse_service_request, request_string, resolve_service_postgres_connection,
    service_error_response, service_json_response, validate_service_request_is_safe,
    ServiceHttpResponse,
};
use crate::workspace::resolve_service_workspace;
use crate::*;
use std::fs;
use std::path::Path;

pub(crate) fn service_object_ddl_endpoint(body: &str, cwd: &Path) -> ServiceHttpResponse {
    let command = "object ddl";
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, command, &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, command, &error);
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, command, &error),
        };

    let object_type = request_string(&request, "objectType").unwrap_or_default();
    let schema = request_string(&request, "schema").unwrap_or_default();
    let object_name = request_string(&request, "objectName")
        .or_else(|| request_string(&request, "name"))
        .unwrap_or_default();
    if !matches!(
        object_type.as_str(),
        "schema"
            | "table"
            | "extension"
            | "enum"
            | "sequence"
            | "index"
            | "view"
            | "constraint"
            | "function"
            | "trigger"
    ) {
        return service_json_response(
            200,
            &ObjectDdlResponse::unsupported(&object_type, &schema, &object_name).to_json(),
        );
    }

    let root = match git_root(&workspace) {
        Some(root) => root,
        None => {
            return service_error_response(
                400,
                command,
                "repositoryPath must be inside a local Git working tree.",
            )
        }
    };

    let relative_path = request_string(&request, "relativePath")
        .or_else(|| default_object_relative_path(&object_type, &schema, &object_name).ok());
    let repository_ddl = match relative_path.as_deref() {
        Some(path) => match read_repository_object_ddl(&root, path) {
            Ok(content) => content,
            Err(error) => return service_error_response(400, command, &error),
        },
        None => None,
    };

    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let database_ddl = match resolve_service_postgres_connection(&request) {
        Ok(Some(connection)) => {
            match database_object_ddl(
                &connection.url,
                &object_type,
                &schema,
                &object_name,
                relative_path.as_deref(),
            ) {
                Ok(ddl) => ddl,
                Err(error) => {
                    warnings.push(error);
                    None
                }
            }
        }
        Ok(None) => {
            warnings.push(
                "Database DDL is unavailable because no PostgreSQL connection was provided."
                    .to_string(),
            );
            None
        }
        Err(error) => {
            errors.push(error);
            None
        }
    };

    let mut object_only = DdlSection {
        repository_ddl,
        database_ddl,
        notes: Vec::new(),
    };

    let (repository_full_context, repository_related, repository_notes) =
        match repository_full_context_ddl(
            &root,
            &object_type,
            &schema,
            &object_name,
            relative_path.as_deref(),
            object_only.repository_ddl.as_deref(),
        ) {
            Ok(result) => result,
            Err(error) => return service_error_response(400, command, &error),
        };
    let mut related_objects = RelatedObjectSet {
        repository: repository_related,
        database: Vec::new(),
    };
    let mut full_context = DdlSection {
        repository_ddl: repository_full_context,
        database_ddl: object_only.database_ddl.clone(),
        notes: repository_notes,
    };

    if let Ok(Some(connection)) = resolve_service_postgres_connection(&request) {
        match database_full_context_ddl(
            &connection.url,
            &object_type,
            &schema,
            &object_name,
            relative_path.as_deref(),
        ) {
            Ok((ddl, related, notes)) => {
                full_context.database_ddl = ddl.or_else(|| object_only.database_ddl.clone());
                related_objects.database = related;
                full_context.notes.extend(notes);
            }
            Err(error) => warnings.push(error),
        }
    }

    object_only.notes.push(
        "Object Only DDL is the normalized durable object representation used by repository files."
            .to_string(),
    );

    service_json_response(
        if errors.is_empty() { 200 } else { 400 },
        &ObjectDdlResponse {
            success: errors.is_empty(),
            object_type,
            schema,
            object_name,
            relative_path,
            object_only,
            full_context,
            related_objects,
            warnings,
            errors,
        }
        .to_json(),
    )
}

fn default_object_relative_path(
    object_type: &str,
    schema: &str,
    object_name: &str,
) -> Result<String, String> {
    match object_type {
        "schema" => schema_file_path(if schema.is_empty() {
            object_name
        } else {
            schema
        }),
        "table" => table_file_path(schema, object_name),
        "extension" => extension_file_path(object_name),
        "enum" => enum_file_path(schema, object_name),
        "sequence" => sequence_file_path(schema, object_name),
        "index" => {
            let parts: Vec<&str> = object_name.split('.').collect();
            if parts.len() == 2 {
                index_file_path(schema, parts[0], parts[1])
            } else {
                Err("Index DDL detail requires objectName as table.index.".to_string())
            }
        }
        "view" => view_file_path(schema, object_name),
        "function" => {
            let parts: Vec<&str> = object_name.split('.').collect();
            if parts.len() == 2 {
                Ok(format!(
                    "database/objects/functions/{}.{}.{}.sql",
                    safe_file_component(schema)?,
                    safe_file_component(parts[0])?,
                    safe_file_component(parts[1])?
                ))
            } else {
                Err("Function DDL detail requires a release/result relativePath.".to_string())
            }
        }
        "constraint" => {
            Err("Constraint DDL detail requires a release/result relativePath.".to_string())
        }
        "trigger" => {
            let parts: Vec<&str> = object_name.split('.').collect();
            if parts.len() == 2 {
                trigger_file_path(schema, parts[0], parts[1])
            } else {
                Err("Trigger DDL detail requires objectName as relation.trigger.".to_string())
            }
        }
        _ => Err("Unsupported object type for DDL detail.".to_string()),
    }
}

fn read_repository_object_ddl(root: &Path, relative_path: &str) -> Result<Option<String>, String> {
    validate_repository_object_relative_path(relative_path)?;
    let root = fs::canonicalize(root)
        .map_err(|_| "Could not resolve selected repository root.".to_string())?;
    let target = root.join(relative_path);
    if !target.exists() {
        return Ok(None);
    }
    let canonical = fs::canonicalize(&target)
        .map_err(|_| "Could not resolve repository object file path.".to_string())?;
    let objects_root = root.join("database").join("objects");
    if !canonical.starts_with(&objects_root) {
        return Err("Refusing to read outside database/objects/.".to_string());
    }
    fs::read_to_string(&canonical)
        .map(Some)
        .map_err(|error| format!("Could not read repository object file: {error}"))
}

fn repository_full_context_ddl(
    root: &Path,
    object_type: &str,
    schema: &str,
    object_name: &str,
    _relative_path: Option<&str>,
    object_only_ddl: Option<&str>,
) -> Result<DdlContextResult, String> {
    let mut related = Vec::new();
    let mut notes = Vec::new();
    if object_type != "table" {
        if object_type == "function" {
            related.push(RelatedObjectSummary::new(
                "Schema",
                schema,
                "Function schema",
            ));
            if let Some(ddl) = object_only_ddl {
                if let Some(language) = ddl
                    .lines()
                    .find_map(|line| line.strip_prefix("-- Language: ").map(str::trim))
                {
                    related.push(RelatedObjectSummary::new(
                        "Language",
                        language,
                        "Function language",
                    ));
                }
            }
            related.push(RelatedObjectSummary::new(
                "Comments",
                "Not available in Private Beta",
                "Function comment rendering is deferred.",
            ));
        }
        if object_type == "trigger" {
            related.push(RelatedObjectSummary::new(
                "Schema",
                schema,
                "Trigger schema",
            ));
            if let Some((relation, _trigger)) =
                trigger_identity_from_name_or_path(object_name, _relative_path)
            {
                related.push(RelatedObjectSummary::new(
                    "Parent Relation",
                    &relation,
                    "Trigger parent relation",
                ));
            }
            if let Some(ddl) = object_only_ddl {
                if let Some(function) = ddl
                    .lines()
                    .find_map(|line| line.strip_prefix("-- Trigger function: ").map(str::trim))
                {
                    related.push(RelatedObjectSummary::new(
                        "Trigger Function",
                        function,
                        "Trigger function",
                    ));
                }
            }
            related.push(RelatedObjectSummary::new(
                "Comments",
                "Not available in Private Beta",
                "Trigger comment rendering is deferred.",
            ));
        }
        notes.push("Full context is the same as object-only DDL for this object type.".to_string());
        return Ok((object_only_ddl.map(ToOwned::to_owned), related, notes));
    }

    let mut ddl_parts = Vec::new();
    if let Some(ddl) = object_only_ddl {
        ddl_parts.push(ddl.to_string());
    }

    let index_files = repository_index_files_for_table(root, schema, object_name)?;
    let constraint_files = repository_constraint_files_for_table(root, schema, object_name)?;
    let trigger_files = repository_trigger_files_for_relation(root, schema, object_name)?;
    if index_files.is_empty() {
        related.push(RelatedObjectSummary::new(
            "Indexes",
            "No related repository index files found.",
            "Not available in Private Beta",
        ));
        if !ddl_parts.is_empty() {
            ddl_parts.push(
                "-- No related repository index object files were found for this table."
                    .to_string(),
            );
        }
    } else {
        for (relative_path, index_name, content) in index_files {
            related.push(RelatedObjectSummary::new(
                "Indexes",
                &index_name,
                &relative_path,
            ));
            ddl_parts.push(format!(
                "-- Related repository index object: {relative_path}\n{}",
                content.trim()
            ));
        }
    }
    if constraint_files.is_empty() {
        related.push(RelatedObjectSummary::new(
            "Constraints",
            "No related repository constraint files found.",
            "Not available in repository context",
        ));
    } else {
        for (relative_path, constraint_name, content) in constraint_files {
            related.push(RelatedObjectSummary::new(
                "Constraints",
                &constraint_name,
                &relative_path,
            ));
            ddl_parts.push(format!(
                "-- Related repository constraint object: {relative_path}\n{}",
                content.trim()
            ));
        }
    }
    if trigger_files.is_empty() {
        related.push(RelatedObjectSummary::new(
            "Triggers",
            "No related repository trigger files found.",
            "Not available in repository context",
        ));
    } else {
        for (relative_path, trigger_name, content) in trigger_files {
            related.push(RelatedObjectSummary::new(
                "Triggers",
                &trigger_name,
                &relative_path,
            ));
            ddl_parts.push(format!(
                "-- Related repository trigger object: {relative_path}\n{}",
                content.trim()
            ));
        }
    }
    related.push(RelatedObjectSummary::new(
        "Comments",
        "Not available in Private Beta",
        "Durable comment object coverage is deferred.",
    ));

    Ok((join_ddl_parts(ddl_parts), related, notes))
}

fn repository_trigger_files_for_relation(
    root: &Path,
    schema: &str,
    relation: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let schema = safe_file_component(schema)?;
    let relation = safe_file_component(relation)?;
    let trigger_dir = root.join("database").join("objects").join("triggers");
    if !trigger_dir.exists() {
        return Ok(Vec::new());
    }
    let prefix = format!("{schema}.{relation}.");
    let mut files = Vec::new();
    for entry in fs::read_dir(&trigger_dir)
        .map_err(|error| format!("Could not read repository triggers folder: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Could not read repository trigger entry: {error}"))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.starts_with(&prefix) || !file_name.ends_with(".sql") {
            continue;
        }
        let trigger_name = file_name
            .trim_start_matches(&prefix)
            .trim_end_matches(".sql")
            .to_string();
        let relative_path = format!("database/objects/triggers/{file_name}");
        if let Some(content) = read_repository_object_ddl(root, &relative_path)? {
            files.push((relative_path, trigger_name, content));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn repository_constraint_files_for_table(
    root: &Path,
    schema: &str,
    table: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let schema = safe_file_component(schema)?;
    let table = safe_file_component(table)?;
    let mut files = Vec::new();
    for folder in [
        "primary-keys",
        "unique-constraints",
        "foreign-keys",
        "check-constraints",
    ] {
        let dir = root.join("database/objects/constraints").join(folder);
        if !dir.exists() {
            continue;
        }
        let prefix = format!("{schema}.{table}.");
        for entry in fs::read_dir(&dir)
            .map_err(|error| format!("Could not read repository constraints folder: {error}"))?
        {
            let entry = entry
                .map_err(|error| format!("Could not read repository constraint entry: {error}"))?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if !file_name.starts_with(&prefix) || !file_name.ends_with(".sql") {
                continue;
            }
            let constraint_name = file_name
                .trim_start_matches(&prefix)
                .trim_end_matches(".sql")
                .to_string();
            let relative_path = format!("database/objects/constraints/{folder}/{file_name}");
            if let Some(content) = read_repository_object_ddl(root, &relative_path)? {
                files.push((relative_path, constraint_name, content));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn repository_index_files_for_table(
    root: &Path,
    schema: &str,
    table: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let schema = safe_file_component(schema)?;
    let table = safe_file_component(table)?;
    let index_dir = root.join("database").join("objects").join("indexes");
    if !index_dir.exists() {
        return Ok(Vec::new());
    }
    let prefix = format!("{schema}.{table}.");
    let mut files = Vec::new();
    for entry in fs::read_dir(&index_dir)
        .map_err(|error| format!("Could not read repository indexes folder: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Could not read repository index entry: {error}"))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.starts_with(&prefix) || !file_name.ends_with(".sql") {
            continue;
        }
        let index_name = file_name
            .trim_start_matches(&prefix)
            .trim_end_matches(".sql")
            .to_string();
        let relative_path = format!("database/objects/indexes/{file_name}");
        if let Some(content) = read_repository_object_ddl(root, &relative_path)? {
            files.push((relative_path, index_name, content));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn database_full_context_ddl(
    connection_url: &str,
    object_type: &str,
    schema: &str,
    object_name: &str,
    relative_path: Option<&str>,
) -> Result<DdlContextResult, String> {
    if object_type != "table" {
        let ddl = database_object_ddl(
            connection_url,
            object_type,
            schema,
            object_name,
            relative_path,
        )?;
        let related = match object_type {
            "function" => database_function_related_objects(
                connection_url,
                schema,
                object_name,
                relative_path,
            )?,
            "trigger" => database_trigger_related_objects(
                connection_url,
                schema,
                object_name,
                relative_path,
            )?,
            _ => Vec::new(),
        };
        return Ok((
            ddl,
            related,
            vec!["Full context is the same as object-only DDL for this object type.".to_string()],
        ));
    }
    if !is_postgres_connection_url(connection_url) {
        return Err(invalid_postgres_url_message());
    }
    let inventory =
        inspect_postgres(connection_url).map_err(|error| redact_message(&error, connection_url))?;
    let table = inventory
        .tables
        .iter()
        .find(|candidate| candidate.schema_name == schema && candidate.table_name == object_name);
    if table.is_none() {
        return Ok((None, Vec::new(), Vec::new()));
    }
    let columns: Vec<ColumnInfo> = inventory
        .columns
        .iter()
        .filter(|column| column.schema_name == schema && column.table_name == object_name)
        .cloned()
        .collect();
    let mut ddl_parts = vec![render_table_sql(schema, object_name, &columns)];
    let mut related = Vec::new();
    let mut indexes: Vec<IndexInfo> = inventory
        .indexes
        .iter()
        .filter(|index| index.schema_name == schema && index.table_name == object_name)
        .cloned()
        .collect();
    indexes.sort_by(|left, right| left.index_name.cmp(&right.index_name));
    if indexes.is_empty() {
        related.push(RelatedObjectSummary::new(
            "Indexes",
            "No related database indexes found.",
            "Not available in Private Beta",
        ));
    } else {
        for index in indexes {
            related.push(RelatedObjectSummary::new(
                "Indexes",
                &index.index_name,
                &index.definition,
            ));
            ddl_parts.push(render_index_sql(&index));
        }
    }
    let mut constraints: Vec<ConstraintInfo> = inventory
        .constraints
        .iter()
        .filter(|constraint| {
            constraint.schema_name == schema && constraint.table_name == object_name
        })
        .cloned()
        .collect();
    constraints.sort_by(|left, right| left.constraint_name.cmp(&right.constraint_name));
    if constraints.is_empty() {
        related.push(RelatedObjectSummary::new(
            "Constraints",
            "No related database constraints found.",
            "Not available in database context",
        ));
    } else {
        for constraint in constraints {
            related.push(RelatedObjectSummary::new(
                "Constraints",
                &constraint.constraint_name,
                &constraint.definition,
            ));
            ddl_parts.push(render_constraint_sql(&constraint));
        }
    }
    let mut triggers: Vec<TriggerInfo> = inventory
        .triggers
        .iter()
        .filter(|trigger| trigger.schema_name == schema && trigger.relation_name == object_name)
        .cloned()
        .collect();
    triggers.sort_by(|left, right| left.trigger_name.cmp(&right.trigger_name));
    if triggers.is_empty() {
        related.push(RelatedObjectSummary::new(
            "Triggers",
            "No related database triggers found.",
            "Not available in database context",
        ));
    } else {
        for trigger in triggers {
            related.push(RelatedObjectSummary::new(
                "Triggers",
                &trigger.trigger_name,
                &trigger.definition,
            ));
            ddl_parts.push(render_trigger_sql(&trigger));
        }
    }
    related.push(RelatedObjectSummary::new(
        "Comments",
        "Not available in Private Beta",
        "Comment rendering in full context is deferred.",
    ));

    Ok((join_ddl_parts(ddl_parts), related, Vec::new()))
}

fn join_ddl_parts(parts: Vec<String>) -> Option<String> {
    let cleaned: Vec<String> = parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(format!("{}\n", cleaned.join("\n\n")))
    }
}

fn validate_repository_object_relative_path(relative_path: &str) -> Result<(), String> {
    if relative_path.contains('\0')
        || relative_path.contains("..")
        || relative_path.contains('\\')
        || relative_path.starts_with('/')
        || relative_path.contains(':')
        || !relative_path.ends_with(".sql")
    {
        return Err("Unsafe repository object file path.".to_string());
    }
    if relative_path.starts_with("database/objects/schemas/")
        || relative_path.starts_with("database/objects/tables/")
        || relative_path.starts_with("database/objects/extensions/")
        || relative_path.starts_with("database/objects/enums/")
        || relative_path.starts_with("database/objects/sequences/")
        || relative_path.starts_with("database/objects/indexes/")
        || relative_path.starts_with("database/objects/views/")
        || relative_path.starts_with("database/objects/functions/")
        || relative_path.starts_with("database/objects/triggers/")
        || relative_path.starts_with("database/objects/constraints/primary-keys/")
        || relative_path.starts_with("database/objects/constraints/unique-constraints/")
        || relative_path.starts_with("database/objects/constraints/foreign-keys/")
        || relative_path.starts_with("database/objects/constraints/check-constraints/")
    {
        Ok(())
    } else {
        Err("DDL detail can read only supported SQL files under database/objects/.".to_string())
    }
}

fn database_object_ddl(
    connection_url: &str,
    object_type: &str,
    schema: &str,
    object_name: &str,
    relative_path: Option<&str>,
) -> Result<Option<String>, String> {
    if !is_postgres_connection_url(connection_url) {
        return Err(invalid_postgres_url_message());
    }
    let inventory =
        inspect_postgres(connection_url).map_err(|error| redact_message(&error, connection_url))?;
    match object_type {
        "schema" => {
            let schema_name = if schema.is_empty() {
                object_name
            } else {
                schema
            };
            if inventory
                .schemas
                .iter()
                .any(|candidate| candidate.name == schema_name)
            {
                Ok(Some(render_schema_sql(schema_name)))
            } else {
                Ok(None)
            }
        }
        "table" => {
            let table = inventory.tables.iter().find(|candidate| {
                candidate.schema_name == schema && candidate.table_name == object_name
            });
            if table.is_none() {
                return Ok(None);
            }
            let columns: Vec<ColumnInfo> = inventory
                .columns
                .iter()
                .filter(|column| column.schema_name == schema && column.table_name == object_name)
                .cloned()
                .collect();
            Ok(Some(render_table_sql(schema, object_name, &columns)))
        }
        "extension" => Ok(inventory
            .extensions
            .iter()
            .find(|candidate| candidate.extension_name == object_name)
            .map(render_extension_sql)),
        "enum" => Ok(inventory
            .enums
            .iter()
            .find(|candidate| candidate.schema_name == schema && candidate.enum_name == object_name)
            .map(render_enum_sql)),
        "sequence" => Ok(inventory
            .sequences
            .iter()
            .find(|candidate| {
                candidate.schema_name == schema && candidate.sequence_name == object_name
            })
            .map(render_sequence_sql)),
        "index" => {
            let index_name = object_name
                .rsplit_once('.')
                .map(|(_, index)| index)
                .unwrap_or(object_name);
            Ok(inventory
                .indexes
                .iter()
                .find(|candidate| {
                    candidate.schema_name == schema && candidate.index_name == index_name
                })
                .map(render_index_sql))
        }
        "view" => Ok(inventory
            .views
            .iter()
            .find(|candidate| candidate.schema_name == schema && candidate.view_name == object_name)
            .map(render_view_sql)),
        "function" => {
            Ok(
                find_function_for_object(&inventory.functions, schema, object_name, relative_path)
                    .map(render_function_sql),
            )
        }
        "trigger" => {
            Ok(
                find_trigger_for_object(&inventory.triggers, schema, object_name, relative_path)
                    .map(render_trigger_sql),
            )
        }
        "constraint" => {
            let constraint_name = object_name
                .rsplit_once('.')
                .map(|(_, name)| name)
                .unwrap_or(object_name);
            Ok(inventory
                .constraints
                .iter()
                .find(|candidate| {
                    candidate.schema_name == schema && candidate.constraint_name == constraint_name
                })
                .map(render_constraint_sql))
        }
        _ => Ok(None),
    }
}

fn find_trigger_for_object<'a>(
    triggers: &'a [TriggerInfo],
    schema: &str,
    object_name: &str,
    relative_path: Option<&str>,
) -> Option<&'a TriggerInfo> {
    let (relation_name, trigger_name) =
        trigger_identity_from_name_or_path(object_name, relative_path)?;
    triggers.iter().find(|candidate| {
        candidate.schema_name == schema
            && candidate.relation_name == relation_name
            && candidate.trigger_name == trigger_name
    })
}

fn trigger_identity_from_name_or_path(
    object_name: &str,
    relative_path: Option<&str>,
) -> Option<(String, String)> {
    if let Some(path) = relative_path {
        let file_name = path.strip_prefix("database/objects/triggers/")?;
        let stem = file_name.strip_suffix(".sql")?;
        let parts: Vec<&str> = stem.split('.').collect();
        if parts.len() == 3 && !parts.iter().any(|part| part.is_empty()) {
            return Some((parts[1].to_string(), parts[2].to_string()));
        }
    }
    let parts: Vec<&str> = object_name.split('.').collect();
    if parts.len() == 2 && !parts.iter().any(|part| part.is_empty()) {
        return Some((parts[0].to_string(), parts[1].to_string()));
    }
    None
}

fn find_function_for_object<'a>(
    functions: &'a [FunctionInfo],
    schema: &str,
    object_name: &str,
    relative_path: Option<&str>,
) -> Option<&'a FunctionInfo> {
    let (function_name, signature_slug) =
        function_identity_from_name_or_path(object_name, relative_path)?;
    functions.iter().find(|candidate| {
        candidate.schema_name == schema
            && candidate.function_name == function_name
            && function_identity_slug(&candidate.identity_arguments)
                .ok()
                .as_deref()
                == Some(signature_slug.as_str())
    })
}

fn function_identity_from_name_or_path(
    object_name: &str,
    relative_path: Option<&str>,
) -> Option<(String, String)> {
    if let Some(path) = relative_path {
        let file_name = path.strip_prefix("database/objects/functions/")?;
        let stem = file_name.strip_suffix(".sql")?;
        let parts: Vec<&str> = stem.split('.').collect();
        if parts.len() == 3 && !parts.iter().any(|part| part.is_empty()) {
            return Some((parts[1].to_string(), parts[2].to_string()));
        }
    }
    let parts: Vec<&str> = object_name.split('.').collect();
    if parts.len() == 2 && !parts.iter().any(|part| part.is_empty()) {
        return Some((parts[0].to_string(), parts[1].to_string()));
    }
    None
}

fn database_trigger_related_objects(
    connection_url: &str,
    schema: &str,
    object_name: &str,
    relative_path: Option<&str>,
) -> Result<Vec<RelatedObjectSummary>, String> {
    if !is_postgres_connection_url(connection_url) {
        return Err(invalid_postgres_url_message());
    }
    let inventory =
        inspect_postgres(connection_url).map_err(|error| redact_message(&error, connection_url))?;
    let Some(trigger) =
        find_trigger_for_object(&inventory.triggers, schema, object_name, relative_path)
    else {
        return Ok(Vec::new());
    };
    let mut related = vec![
        RelatedObjectSummary::new("Schema", &trigger.schema_name, "Trigger schema"),
        RelatedObjectSummary::new(
            "Parent Relation",
            &trigger.relation_name,
            "Trigger parent relation",
        ),
    ];
    if let (Some(function_schema), Some(function_name)) = (
        &trigger.trigger_function_schema,
        &trigger.trigger_function_name,
    ) {
        related.push(RelatedObjectSummary::new(
            "Trigger Function",
            &format!("{function_schema}.{function_name}"),
            "Trigger function",
        ));
    }
    related.push(RelatedObjectSummary::new(
        "Comments",
        "Not available in Private Beta",
        "Trigger comment rendering is deferred.",
    ));
    Ok(related)
}

fn database_function_related_objects(
    connection_url: &str,
    schema: &str,
    object_name: &str,
    relative_path: Option<&str>,
) -> Result<Vec<RelatedObjectSummary>, String> {
    if !is_postgres_connection_url(connection_url) {
        return Err(invalid_postgres_url_message());
    }
    let inventory =
        inspect_postgres(connection_url).map_err(|error| redact_message(&error, connection_url))?;
    let Some(function) =
        find_function_for_object(&inventory.functions, schema, object_name, relative_path)
    else {
        return Ok(Vec::new());
    };
    let mut related = vec![RelatedObjectSummary::new(
        "Schema",
        &function.schema_name,
        "Function schema",
    )];
    if let Some(language) = &function.language {
        related.push(RelatedObjectSummary::new(
            "Language",
            language,
            "Function language",
        ));
    }
    related.push(RelatedObjectSummary::new(
        "Comments",
        "Not available in Private Beta",
        "Function comment rendering is deferred.",
    ));
    Ok(related)
}

#[derive(Debug, Clone)]
struct DdlSection {
    repository_ddl: Option<String>,
    database_ddl: Option<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct RelatedObjectSummary {
    group: String,
    name: String,
    detail: String,
}

impl RelatedObjectSummary {
    fn new(group: &str, name: &str, detail: &str) -> Self {
        Self {
            group: group.to_string(),
            name: name.to_string(),
            detail: detail.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct RelatedObjectSet {
    repository: Vec<RelatedObjectSummary>,
    database: Vec<RelatedObjectSummary>,
}

type DdlContextResult = (Option<String>, Vec<RelatedObjectSummary>, Vec<String>);

#[derive(Debug, Clone)]
struct ObjectDdlResponse {
    success: bool,
    object_type: String,
    schema: String,
    object_name: String,
    relative_path: Option<String>,
    object_only: DdlSection,
    full_context: DdlSection,
    related_objects: RelatedObjectSet,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl ObjectDdlResponse {
    fn unsupported(object_type: &str, schema: &str, object_name: &str) -> Self {
        Self {
            success: false,
            object_type: object_type.to_string(),
            schema: schema.to_string(),
            object_name: object_name.to_string(),
            relative_path: None,
            object_only: DdlSection {
                repository_ddl: None,
                database_ddl: None,
                notes: Vec::new(),
            },
            full_context: DdlSection {
                repository_ddl: None,
                database_ddl: None,
                notes: Vec::new(),
            },
            related_objects: RelatedObjectSet {
                repository: Vec::new(),
                database: Vec::new(),
            },
            warnings: vec!["DDL is available only for supported Private Beta objects.".to_string()],
            errors: Vec::new(),
        }
    }

    fn to_json(&self) -> String {
        let mut json = String::new();
        json.push('{');
        write_json_string_field(&mut json, "command", "object ddl", true);
        write_json_bool_field(&mut json, "success", self.success);
        write_json_string_field(&mut json, "databaseType", "postgresql", false);
        write_json_string_field(&mut json, "objectType", &self.object_type, false);
        write_json_string_field(&mut json, "schema", &self.schema, false);
        write_json_string_field(&mut json, "objectName", &self.object_name, false);
        write_json_optional_string_field(&mut json, "relativePath", self.relative_path.as_deref());
        write_json_optional_string_field(
            &mut json,
            "repositoryDdl",
            self.object_only.repository_ddl.as_deref(),
        );
        write_json_optional_string_field(
            &mut json,
            "databaseDdl",
            self.object_only.database_ddl.as_deref(),
        );
        write_ddl_section_field(&mut json, "objectOnly", &self.object_only);
        write_ddl_section_field(&mut json, "fullContext", &self.full_context);
        write_related_objects_field(&mut json, "relatedObjects", &self.related_objects);
        write_json_array_field(&mut json, "warnings", &self.warnings);
        write_json_array_field(&mut json, "errors", &self.errors);
        json.push('}');
        json
    }
}

fn write_ddl_section_field(json: &mut String, name: &str, section: &DdlSection) {
    json.push(',');
    write!(json, "\"{}\":{{", escape_json(name)).ok();
    write_json_optional_string_member(
        json,
        "repositoryDdl",
        section.repository_ddl.as_deref(),
        true,
    );
    write_json_optional_string_member(json, "databaseDdl", section.database_ddl.as_deref(), false);
    write_json_array_field(json, "notes", &section.notes);
    json.push('}');
}

fn write_related_objects_field(json: &mut String, name: &str, related: &RelatedObjectSet) {
    json.push(',');
    write!(json, "\"{}\":{{", escape_json(name)).ok();
    write_related_object_array_field(json, "repository", &related.repository, true);
    write_related_object_array_field(json, "database", &related.database, false);
    json.push('}');
}

fn write_related_object_array_field(
    json: &mut String,
    name: &str,
    values: &[RelatedObjectSummary],
    first: bool,
) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('{');
        write_json_string_field(json, "group", &value.group, true);
        write_json_string_field(json, "name", &value.name, false);
        write_json_string_field(json, "detail", &value.detail, false);
        json.push('}');
    }
    json.push(']');
}

fn write_json_optional_string_member(
    json: &mut String,
    name: &str,
    value: Option<&str>,
    first: bool,
) {
    if !first {
        json.push(',');
    }
    match value {
        Some(value) => write!(json, "\"{}\":\"{}\"", escape_json(name), escape_json(value)).ok(),
        None => write!(json, "\"{}\":null", escape_json(name)).ok(),
    };
}
