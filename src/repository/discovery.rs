use crate::postgres::render_rls_policy_sql;
use crate::repository::objects::*;
use crate::repository::sync::ExportSelection;
use crate::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn discover_repository_objects(root: &Path) -> Result<RepositoryImport, String> {
    let mut import = RepositoryImport {
        objects: BTreeMap::new(),
        skipped: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    discover_schema_files(root, &mut import)?;
    discover_table_files(root, &mut import)?;
    discover_extension_files(root, &mut import)?;
    discover_enum_files(root, &mut import)?;
    discover_sequence_files(root, &mut import)?;
    discover_index_files(root, &mut import)?;
    discover_view_files(root, &mut import)?;
    discover_materialized_view_files(root, &mut import)?;
    discover_function_files(root, &mut import)?;
    discover_trigger_files(root, &mut import)?;
    discover_constraint_files(root, &mut import)?;
    discover_grant_files(root, &mut import)?;
    discover_rls_policy_files(root, &mut import)?;
    import.skipped.sort();
    import.skipped.dedup();
    Ok(import)
}

fn discover_schema_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/schemas");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/schemas: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read schema file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/schemas/{file_name}");
        let Some(schema) = schema_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            schema_key(&schema),
            DesiredStateObject {
                object_type: RepositoryObjectType::Schema,
                schema_name: schema.clone(),
                table_name: None,
                object_name: schema,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_table_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/tables");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/tables: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read table file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/tables/{file_name}");
        let Some((schema, table)) = table_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err() || safe_file_component(&table).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            table_key(&schema, &table),
            DesiredStateObject {
                object_type: RepositoryObjectType::Table,
                schema_name: schema,
                table_name: Some(table.clone()),
                object_name: table,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_extension_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_one_part_object_files(
        root,
        import,
        "database/objects/extensions",
        RepositoryObjectType::Extension,
        extension_key,
    )
}

fn discover_enum_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_two_part_object_files(
        root,
        import,
        "database/objects/enums",
        RepositoryObjectType::Enum,
        enum_key,
    )
}

fn discover_sequence_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_two_part_object_files(
        root,
        import,
        "database/objects/sequences",
        RepositoryObjectType::Sequence,
        sequence_key,
    )
}

fn discover_view_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    discover_two_part_object_files(
        root,
        import,
        "database/objects/views",
        RepositoryObjectType::View,
        view_key,
    )
}

fn discover_materialized_view_files(
    root: &Path,
    import: &mut RepositoryImport,
) -> Result<(), String> {
    discover_two_part_object_files(
        root,
        import,
        "database/objects/materialized-views",
        RepositoryObjectType::MaterializedView,
        materialized_view_key,
    )
}

fn discover_function_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/functions");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/functions: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Could not read function file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/functions/{file_name}");
        let Some((schema, function, signature)) = three_part_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err()
            || safe_file_component(&function).is_err()
            || safe_file_component(&signature).is_err()
        {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        let object_name = format!("{function}.{signature}");
        import.objects.insert(
            function_key(&schema, &object_name),
            DesiredStateObject {
                object_type: RepositoryObjectType::Function,
                schema_name: schema,
                table_name: None,
                object_name,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_trigger_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/triggers");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/triggers: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read trigger file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/triggers/{file_name}");
        let Some((schema, relation, trigger)) = three_part_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err()
            || safe_file_component(&relation).is_err()
            || safe_file_component(&trigger).is_err()
        {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            trigger_key(&schema, &relation, &trigger),
            DesiredStateObject {
                object_type: RepositoryObjectType::Trigger,
                schema_name: schema,
                table_name: Some(relation.clone()),
                object_name: trigger,
                parent_name: Some(relation),
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_index_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/indexes");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/indexes: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read index file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/indexes/{file_name}");
        let Some((schema, table, index)) = three_part_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err()
            || safe_file_component(&table).is_err()
            || safe_file_component(&index).is_err()
        {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            index_key(&schema, &table, &index),
            DesiredStateObject {
                object_type: RepositoryObjectType::Index,
                schema_name: schema,
                table_name: Some(table.clone()),
                object_name: index,
                parent_name: Some(table),
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_constraint_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    for folder in [
        "primary-keys",
        "unique-constraints",
        "foreign-keys",
        "check-constraints",
    ] {
        let dir = root.join("database/objects/constraints").join(folder);
        for entry in fs::read_dir(&dir).map_err(|error| {
            format!("Could not read database/objects/constraints/{folder}: {error}")
        })? {
            let entry =
                entry.map_err(|error| format!("Could not read constraint file entry: {error}"))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            let relative_path = format!("database/objects/constraints/{folder}/{file_name}");
            let Some((schema, table, constraint)) = three_part_name_from_file(&file_name) else {
                import.skipped.push(relative_path);
                continue;
            };
            if safe_file_component(&schema).is_err()
                || safe_file_component(&table).is_err()
                || safe_file_component(&constraint).is_err()
            {
                import.skipped.push(relative_path);
                continue;
            }
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
            import.objects.insert(
                constraint_key(&schema, &table, &constraint),
                DesiredStateObject {
                    object_type: RepositoryObjectType::Constraint,
                    schema_name: schema,
                    table_name: Some(table.clone()),
                    object_name: constraint,
                    parent_name: Some(table),
                    relative_path,
                    content,
                },
            );
        }
    }
    Ok(())
}

fn discover_grant_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    for (folder, target_kind) in [
        ("schemas", "schema"),
        ("tables", "table"),
        ("views", "view"),
        ("materialized-views", "materializedView"),
        ("sequences", "sequence"),
        ("functions", "function"),
    ] {
        let dir = root.join("database/objects/grants").join(folder);
        for entry in fs::read_dir(&dir)
            .map_err(|error| format!("Could not read database/objects/grants/{folder}: {error}"))?
        {
            let entry =
                entry.map_err(|error| format!("Could not read grant file entry: {error}"))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            let relative_path = format!("database/objects/grants/{folder}/{file_name}");
            let Some(object_ref) = grant_ref_from_file_name(target_kind, &file_name) else {
                import.skipped.push(relative_path);
                continue;
            };
            let ObjectRef::Grant {
                target_kind,
                schema,
                object,
                signature,
                grantee,
            } = object_ref
            else {
                import.skipped.push(relative_path);
                continue;
            };
            if safe_file_component(&target_kind).is_err()
                || safe_file_component(&schema).is_err()
                || object
                    .as_deref()
                    .is_some_and(|value| safe_file_component(value).is_err())
                || signature
                    .as_deref()
                    .is_some_and(|value| safe_file_component(value).is_err())
                || safe_file_component(&grantee).is_err()
            {
                import.skipped.push(relative_path);
                continue;
            }
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
            let identity = grant_identity(
                &target_kind,
                &schema,
                object.as_deref(),
                signature.as_deref(),
                &grantee,
            );
            import.objects.insert(
                grant_key(&identity),
                DesiredStateObject {
                    object_type: RepositoryObjectType::Grant,
                    schema_name: schema,
                    table_name: object,
                    object_name: identity,
                    parent_name: Some(target_kind),
                    relative_path,
                    content,
                },
            );
        }
    }
    Ok(())
}

fn discover_rls_policy_files(root: &Path, import: &mut RepositoryImport) -> Result<(), String> {
    let dir = root.join("database/objects/rls-policies");
    for entry in fs::read_dir(&dir)
        .map_err(|error| format!("Could not read database/objects/rls-policies: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Could not read RLS policy file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("database/objects/rls-policies/{file_name}");
        let Some((schema, table, policy)) = three_part_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err()
            || safe_file_component(&table).is_err()
            || safe_file_component(&policy).is_err()
        {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            rls_policy_key(&schema, &table, &policy),
            DesiredStateObject {
                object_type: RepositoryObjectType::RlsPolicy,
                schema_name: schema,
                table_name: Some(table.clone()),
                object_name: policy,
                parent_name: Some(table),
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_one_part_object_files(
    root: &Path,
    import: &mut RepositoryImport,
    folder: &str,
    object_type: RepositoryObjectType,
    key_fn: fn(&str) -> String,
) -> Result<(), String> {
    let dir = root.join(folder);
    for entry in fs::read_dir(&dir).map_err(|error| format!("Could not read {folder}: {error}"))? {
        let entry =
            entry.map_err(|error| format!("Could not read desired-state file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("{folder}/{file_name}");
        let Some(name) = schema_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&name).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            key_fn(&name),
            DesiredStateObject {
                object_type: object_type.clone(),
                schema_name: String::new(),
                table_name: None,
                object_name: name,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn discover_two_part_object_files(
    root: &Path,
    import: &mut RepositoryImport,
    folder: &str,
    object_type: RepositoryObjectType,
    key_fn: fn(&str, &str) -> String,
) -> Result<(), String> {
    let dir = root.join(folder);
    for entry in fs::read_dir(&dir).map_err(|error| format!("Could not read {folder}: {error}"))? {
        let entry =
            entry.map_err(|error| format!("Could not read desired-state file entry: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_path = format!("{folder}/{file_name}");
        let Some((schema, name)) = table_name_from_file(&file_name) else {
            import.skipped.push(relative_path);
            continue;
        };
        if safe_file_component(&schema).is_err() || safe_file_component(&name).is_err() {
            import.skipped.push(relative_path);
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {relative_path}: {error}"))?;
        import.objects.insert(
            key_fn(&schema, &name),
            DesiredStateObject {
                object_type: object_type.clone(),
                schema_name: schema,
                table_name: None,
                object_name: name,
                parent_name: None,
                relative_path,
                content,
            },
        );
    }
    Ok(())
}

fn schema_name_from_file(file_name: &str) -> Option<String> {
    file_name
        .strip_suffix(".sql")
        .filter(|stem| !stem.is_empty())
        .map(|stem| stem.to_string())
}

fn table_name_from_file(file_name: &str) -> Option<(String, String)> {
    let stem = file_name.strip_suffix(".sql")?;
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some((parts[0].to_string(), parts[1].to_string()))
}

fn three_part_name_from_file(file_name: &str) -> Option<(String, String, String)> {
    let stem = file_name.strip_suffix(".sql")?;
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
    ))
}

fn grant_identity(
    target_kind: &str,
    schema: &str,
    object_name: Option<&str>,
    signature: Option<&str>,
    grantee: &str,
) -> String {
    match (object_name, signature) {
        (Some(object_name), Some(signature)) => {
            format!("{target_kind}.{schema}.{object_name}.{signature}.{grantee}")
        }
        (Some(object_name), None) => format!("{target_kind}.{schema}.{object_name}.{grantee}"),
        (None, _) => format!("{target_kind}.{schema}.{grantee}"),
    }
}

pub(crate) fn render_database_objects_for_selection(
    _root: &Path,
    inventory: &PostgresInventory,
    selection: &ExportSelection,
) -> Result<BTreeMap<String, DesiredStateObject>, String> {
    let mut objects = BTreeMap::new();

    let mut schema_names = Vec::new();
    let mut table_names = Vec::new();
    let mut include_extensions = false;
    let mut enum_names = Vec::new();
    let mut sequence_names = Vec::new();
    let mut index_names = Vec::new();
    let mut view_names = Vec::new();
    let mut materialized_view_names = Vec::new();
    let mut constraint_names = Vec::new();
    let mut function_names = Vec::new();
    let mut trigger_names = Vec::new();
    let mut grant_names = Vec::new();
    let mut rls_policy_names = Vec::new();
    match selection {
        ExportSelection::All => {
            include_extensions = true;
            schema_names.extend(inventory.schemas.iter().map(|schema| schema.name.clone()));
            table_names.extend(
                inventory
                    .tables
                    .iter()
                    .filter(|table| table.table_type == "BASE TABLE")
                    .map(|table| (table.schema_name.clone(), table.table_name.clone())),
            );
            enum_names.extend(
                inventory
                    .enums
                    .iter()
                    .map(|item| (item.schema_name.clone(), item.enum_name.clone())),
            );
            sequence_names.extend(
                inventory
                    .sequences
                    .iter()
                    .map(|item| (item.schema_name.clone(), item.sequence_name.clone())),
            );
            index_names.extend(inventory.indexes.iter().map(|item| {
                (
                    item.schema_name.clone(),
                    item.table_name.clone(),
                    item.index_name.clone(),
                )
            }));
            view_names.extend(
                inventory
                    .views
                    .iter()
                    .map(|item| (item.schema_name.clone(), item.view_name.clone())),
            );
            materialized_view_names.extend(inventory.materialized_views.iter().map(|item| {
                (
                    item.schema_name.clone(),
                    item.materialized_view_name.clone(),
                )
            }));
            constraint_names.extend(inventory.constraints.iter().map(|item| {
                (
                    item.schema_name.clone(),
                    item.table_name.clone(),
                    item.constraint_name.clone(),
                )
            }));
            function_names.extend(inventory.functions.iter().map(|item| {
                (
                    item.schema_name.clone(),
                    item.function_name.clone(),
                    item.identity_arguments.clone(),
                )
            }));
            trigger_names.extend(inventory.triggers.iter().map(|item| {
                (
                    item.schema_name.clone(),
                    item.relation_name.clone(),
                    item.trigger_name.clone(),
                )
            }));
            grant_names.extend(inventory.grants.iter().map(|item| {
                (
                    item.target_kind.clone(),
                    item.schema_name.clone(),
                    item.object_name.clone(),
                    item.identity_arguments.clone(),
                    item.grantee.clone(),
                )
            }));
            rls_policy_names.extend(inventory.rls_policies.iter().map(|item| {
                (
                    item.schema_name.clone(),
                    item.table_name.clone(),
                    item.policy_name.clone(),
                )
            }));
        }
        ExportSelection::Schema(schema) => {
            if inventory
                .schemas
                .iter()
                .any(|candidate| candidate.name == *schema)
            {
                schema_names.push(schema.clone());
            }
            table_names.extend(
                inventory
                    .tables
                    .iter()
                    .filter(|table| {
                        table.schema_name == *schema && table.table_type == "BASE TABLE"
                    })
                    .map(|table| (table.schema_name.clone(), table.table_name.clone())),
            );
            enum_names.extend(
                inventory
                    .enums
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| (item.schema_name.clone(), item.enum_name.clone())),
            );
            sequence_names.extend(
                inventory
                    .sequences
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| (item.schema_name.clone(), item.sequence_name.clone())),
            );
            index_names.extend(
                inventory
                    .indexes
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.index_name.clone(),
                        )
                    }),
            );
            view_names.extend(
                inventory
                    .views
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| (item.schema_name.clone(), item.view_name.clone())),
            );
            materialized_view_names.extend(
                inventory
                    .materialized_views
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.materialized_view_name.clone(),
                        )
                    }),
            );
            constraint_names.extend(
                inventory
                    .constraints
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.constraint_name.clone(),
                        )
                    }),
            );
            function_names.extend(
                inventory
                    .functions
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.function_name.clone(),
                            item.identity_arguments.clone(),
                        )
                    }),
            );
            trigger_names.extend(
                inventory
                    .triggers
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.relation_name.clone(),
                            item.trigger_name.clone(),
                        )
                    }),
            );
            grant_names.extend(
                inventory
                    .grants
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.target_kind.clone(),
                            item.schema_name.clone(),
                            item.object_name.clone(),
                            item.identity_arguments.clone(),
                            item.grantee.clone(),
                        )
                    }),
            );
            rls_policy_names.extend(
                inventory
                    .rls_policies
                    .iter()
                    .filter(|item| item.schema_name == *schema)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.policy_name.clone(),
                        )
                    }),
            );
        }
        ExportSelection::Table { schema, table } => {
            if inventory.tables.iter().any(|candidate| {
                candidate.schema_name == *schema
                    && candidate.table_name == *table
                    && candidate.table_type == "BASE TABLE"
            }) {
                table_names.push((schema.clone(), table.clone()));
            }
            index_names.extend(
                inventory
                    .indexes
                    .iter()
                    .filter(|item| item.schema_name == *schema && item.table_name == *table)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.index_name.clone(),
                        )
                    }),
            );
            constraint_names.extend(
                inventory
                    .constraints
                    .iter()
                    .filter(|item| item.schema_name == *schema && item.table_name == *table)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.constraint_name.clone(),
                        )
                    }),
            );
            trigger_names.extend(
                inventory
                    .triggers
                    .iter()
                    .filter(|item| item.schema_name == *schema && item.relation_name == *table)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.relation_name.clone(),
                            item.trigger_name.clone(),
                        )
                    }),
            );
            grant_names.extend(
                inventory
                    .grants
                    .iter()
                    .filter(|item| {
                        item.schema_name == *schema
                            && item.object_name.as_deref() == Some(table.as_str())
                            && item.target_kind == "table"
                    })
                    .map(|item| {
                        (
                            item.target_kind.clone(),
                            item.schema_name.clone(),
                            item.object_name.clone(),
                            item.identity_arguments.clone(),
                            item.grantee.clone(),
                        )
                    }),
            );
            rls_policy_names.extend(
                inventory
                    .rls_policies
                    .iter()
                    .filter(|item| item.schema_name == *schema && item.table_name == *table)
                    .map(|item| {
                        (
                            item.schema_name.clone(),
                            item.table_name.clone(),
                            item.policy_name.clone(),
                        )
                    }),
            );
        }
    }

    schema_names.sort();
    schema_names.dedup();
    table_names.sort();
    table_names.dedup();
    enum_names.sort();
    enum_names.dedup();
    sequence_names.sort();
    sequence_names.dedup();
    index_names.sort();
    index_names.dedup();
    view_names.sort();
    view_names.dedup();
    materialized_view_names.sort();
    materialized_view_names.dedup();
    constraint_names.sort();
    constraint_names.dedup();
    function_names.sort();
    function_names.dedup();
    trigger_names.sort();
    trigger_names.dedup();
    grant_names.sort();
    grant_names.dedup();
    rls_policy_names.sort();
    rls_policy_names.dedup();

    for schema in schema_names {
        let relative_path = schema_file_path(&schema)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Schema,
            schema_name: schema.clone(),
            table_name: None,
            object_name: schema.clone(),
            parent_name: None,
            relative_path,
            content: render_schema_sql(&schema),
        };
        objects.insert(object_key(&object), object);
    }

    for (schema, table) in table_names {
        let relative_path = table_file_path(&schema, &table)?;
        let columns: Vec<ColumnInfo> = inventory
            .columns
            .iter()
            .filter(|column| column.schema_name == schema && column.table_name == table)
            .cloned()
            .collect();
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Table,
            schema_name: schema.clone(),
            table_name: Some(table.clone()),
            object_name: table.clone(),
            parent_name: None,
            relative_path,
            content: render_table_sql(&schema, &table, &columns),
        };
        objects.insert(object_key(&object), object);
    }
    if include_extensions {
        for extension in &inventory.extensions {
            let relative_path = extension_file_path(&extension.extension_name)?;
            let object = DesiredStateObject {
                object_type: RepositoryObjectType::Extension,
                schema_name: String::new(),
                table_name: None,
                object_name: extension.extension_name.clone(),
                parent_name: None,
                relative_path,
                content: render_extension_sql(extension),
            };
            objects.insert(object_key(&object), object);
        }
    }
    for (schema, enum_name) in enum_names {
        let Some(enum_info) = inventory
            .enums
            .iter()
            .find(|item| item.schema_name == schema && item.enum_name == enum_name)
        else {
            continue;
        };
        let relative_path = enum_file_path(&schema, &enum_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Enum,
            schema_name: schema.clone(),
            table_name: None,
            object_name: enum_name.clone(),
            parent_name: None,
            relative_path,
            content: render_enum_sql(enum_info),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, sequence_name) in sequence_names {
        let Some(sequence) = inventory
            .sequences
            .iter()
            .find(|item| item.schema_name == schema && item.sequence_name == sequence_name)
        else {
            continue;
        };
        let relative_path = sequence_file_path(&schema, &sequence_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Sequence,
            schema_name: schema.clone(),
            table_name: None,
            object_name: sequence_name.clone(),
            parent_name: None,
            relative_path,
            content: render_sequence_sql(sequence),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, table, index_name) in index_names {
        let Some(index) = inventory.indexes.iter().find(|item| {
            item.schema_name == schema && item.table_name == table && item.index_name == index_name
        }) else {
            continue;
        };
        let relative_path = index_file_path(&schema, &table, &index_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Index,
            schema_name: schema.clone(),
            table_name: Some(table.clone()),
            object_name: index_name.clone(),
            parent_name: Some(table),
            relative_path,
            content: render_index_sql(index),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, view_name) in view_names {
        let Some(view) = inventory
            .views
            .iter()
            .find(|item| item.schema_name == schema && item.view_name == view_name)
        else {
            continue;
        };
        let relative_path = view_file_path(&schema, &view_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::View,
            schema_name: schema.clone(),
            table_name: None,
            object_name: view_name.clone(),
            parent_name: None,
            relative_path,
            content: render_view_sql(view),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, materialized_view_name) in materialized_view_names {
        let Some(materialized_view) = inventory.materialized_views.iter().find(|item| {
            item.schema_name == schema && item.materialized_view_name == materialized_view_name
        }) else {
            continue;
        };
        let relative_path = materialized_view_file_path(&schema, &materialized_view_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::MaterializedView,
            schema_name: schema.clone(),
            table_name: None,
            object_name: materialized_view_name.clone(),
            parent_name: None,
            relative_path,
            content: render_materialized_view_sql(materialized_view),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, function_name, identity_arguments) in function_names {
        let Some(function) = inventory.functions.iter().find(|item| {
            item.schema_name == schema
                && item.function_name == function_name
                && item.identity_arguments == identity_arguments
        }) else {
            continue;
        };
        let relative_path = function_file_path(&schema, &function_name, &identity_arguments)?;
        let signature = function_identity_slug(&identity_arguments)?;
        let object_name = format!("{function_name}.{signature}");
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Function,
            schema_name: schema.clone(),
            table_name: None,
            object_name,
            parent_name: None,
            relative_path,
            content: render_function_sql(function),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, relation, trigger_name) in trigger_names {
        let Some(trigger) = inventory.triggers.iter().find(|item| {
            item.schema_name == schema
                && item.relation_name == relation
                && item.trigger_name == trigger_name
        }) else {
            continue;
        };
        let relative_path = trigger_file_path(&schema, &relation, &trigger_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Trigger,
            schema_name: schema.clone(),
            table_name: Some(relation.clone()),
            object_name: trigger_name.clone(),
            parent_name: Some(relation),
            relative_path,
            content: render_trigger_sql(trigger),
        };
        objects.insert(object_key(&object), object);
    }
    for (target_kind, schema, object_name, identity_arguments, grantee) in grant_names {
        let Some(grant) = inventory.grants.iter().find(|item| {
            item.target_kind == target_kind
                && item.schema_name == schema
                && item.object_name == object_name
                && item.identity_arguments == identity_arguments
                && item.grantee == grantee
        }) else {
            continue;
        };
        let signature = if target_kind == "function" {
            Some(function_identity_slug(
                identity_arguments.as_deref().unwrap_or(""),
            )?)
        } else {
            None
        };
        let relative_path = grant_file_path(
            &target_kind,
            &schema,
            object_name.as_deref(),
            signature.as_deref(),
            &grantee,
        )?;
        let identity = grant_identity(
            &target_kind,
            &schema,
            object_name.as_deref(),
            signature.as_deref(),
            &grant_grantee_file_token(&grantee)?,
        );
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Grant,
            schema_name: schema.clone(),
            table_name: object_name.clone(),
            object_name: identity,
            parent_name: Some(target_kind),
            relative_path,
            content: render_grant_sql(grant),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, table, policy_name) in rls_policy_names {
        let Some(policy) = inventory.rls_policies.iter().find(|item| {
            item.schema_name == schema
                && item.table_name == table
                && item.policy_name == policy_name
        }) else {
            continue;
        };
        let relative_path = rls_policy_file_path(&schema, &table, &policy_name)?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::RlsPolicy,
            schema_name: schema.clone(),
            table_name: Some(table.clone()),
            object_name: policy_name.clone(),
            parent_name: Some(table),
            relative_path,
            content: render_rls_policy_sql(policy),
        };
        objects.insert(object_key(&object), object);
    }
    for (schema, table, constraint_name) in constraint_names {
        let Some(constraint) = inventory.constraints.iter().find(|item| {
            item.schema_name == schema
                && item.table_name == table
                && item.constraint_name == constraint_name
        }) else {
            continue;
        };
        let relative_path = constraint_file_path(
            &constraint.constraint_type,
            &schema,
            &table,
            &constraint_name,
        )?;
        let object = DesiredStateObject {
            object_type: RepositoryObjectType::Constraint,
            schema_name: schema.clone(),
            table_name: Some(table.clone()),
            object_name: constraint_name.clone(),
            parent_name: Some(table),
            relative_path,
            content: render_constraint_sql(constraint),
        };
        objects.insert(object_key(&object), object);
    }
    Ok(objects)
}

pub(crate) fn select_repository_objects(
    objects: &BTreeMap<String, DesiredStateObject>,
    selection: &ExportSelection,
) -> BTreeMap<String, DesiredStateObject> {
    let mut selected = BTreeMap::new();
    for (key, object) in objects {
        let include = match selection {
            ExportSelection::All => true,
            ExportSelection::Schema(schema) => object.schema_name == *schema,
            ExportSelection::Table { schema, table } => {
                object.schema_name == *schema
                    && (object.object_type == RepositoryObjectType::Table
                        && object.table_name.as_deref() == Some(table.as_str())
                        || object.object_type == RepositoryObjectType::Index
                            && object.parent_name.as_deref() == Some(table.as_str())
                        || object.object_type == RepositoryObjectType::Constraint
                            && object.parent_name.as_deref() == Some(table.as_str())
                        || object.object_type == RepositoryObjectType::Trigger
                            && object.parent_name.as_deref() == Some(table.as_str())
                        || object.object_type == RepositoryObjectType::Grant
                            && object.parent_name.as_deref() == Some("table")
                            && object.table_name.as_deref() == Some(table.as_str())
                        || object.object_type == RepositoryObjectType::RlsPolicy
                            && object.parent_name.as_deref() == Some(table.as_str()))
            }
        };
        if include {
            selected.insert(key.clone(), object.clone());
        }
    }
    selected
}
