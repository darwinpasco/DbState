use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepositoryObjectType {
    Schema,
    Table,
    Extension,
    Enum,
    Sequence,
    Index,
    View,
    MaterializedView,
    Constraint,
    Function,
    Trigger,
    Grant,
    RlsPolicy,
}

#[derive(Debug, Clone)]
pub(crate) struct DesiredStateObject {
    pub(crate) object_type: RepositoryObjectType,
    pub(crate) schema_name: String,
    pub(crate) table_name: Option<String>,
    pub(crate) object_name: String,
    pub(crate) parent_name: Option<String>,
    pub(crate) relative_path: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RepositoryImport {
    pub(crate) objects: BTreeMap<String, DesiredStateObject>,
    pub(crate) skipped: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ObjectRef {
    Schema(String),
    Table {
        schema: String,
        table: String,
    },
    Extension(String),
    Enum {
        schema: String,
        enum_name: String,
    },
    Sequence {
        schema: String,
        sequence: String,
    },
    Index {
        schema: String,
        table: String,
        index: String,
    },
    View {
        schema: String,
        view: String,
    },
    MaterializedView {
        schema: String,
        materialized_view: String,
    },
    Constraint {
        schema: String,
        table: String,
        constraint: String,
    },
    Function {
        schema: String,
        function: String,
        signature: String,
    },
    Trigger {
        schema: String,
        relation: String,
        trigger: String,
    },
    Grant {
        target_kind: String,
        schema: String,
        object: Option<String>,
        signature: Option<String>,
        grantee: String,
    },
    RlsPolicy {
        schema: String,
        table: String,
        policy: String,
    },
}

impl ObjectRef {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let Some((object_type, identity)) = value.split_once(':') else {
            return Err(format!(
                "Invalid object reference '{value}'. Use schema:<schema>, table:<schema>.<table>, extension:<name>, enum:<schema>.<name>, sequence:<schema>.<name>, index:<schema>.<table>.<name>, view:<schema>.<name>, materializedView:<schema>.<name>, constraint:<schema>.<table>.<name>, function:<schema>.<function>.<signature>, trigger:<schema>.<relation>.<trigger>, grant:<target-kind>.<target-identity>.<grantee>, or rlsPolicy:<schema>.<table>.<policy>."
            ));
        };
        match object_type {
            "schema" => {
                if identity.trim().is_empty() {
                    return Err("Schema object reference cannot be empty.".to_string());
                }
                safe_file_component(identity)?;
                Ok(Self::Schema(identity.to_string()))
            }
            "table" => {
                let Some((schema, table)) = identity.split_once('.') else {
                    return Err(format!(
                        "Invalid table object reference '{value}'. Use table:<schema>.<table>."
                    ));
                };
                if schema.trim().is_empty() || table.trim().is_empty() {
                    return Err(format!(
                        "Invalid table object reference '{value}'. Use table:<schema>.<table>."
                    ));
                }
                safe_file_component(schema)?;
                safe_file_component(table)?;
                Ok(Self::Table {
                    schema: schema.to_string(),
                    table: table.to_string(),
                })
            }
            "extension" => {
                if identity.trim().is_empty() {
                    return Err("Extension object reference cannot be empty.".to_string());
                }
                safe_file_component(identity)?;
                Ok(Self::Extension(identity.to_string()))
            }
            "enum" => {
                let (schema, enum_name) = parse_two_part_object_ref(value, identity, "enum")?;
                Ok(Self::Enum { schema, enum_name })
            }
            "sequence" => {
                let (schema, sequence) = parse_two_part_object_ref(value, identity, "sequence")?;
                Ok(Self::Sequence { schema, sequence })
            }
            "index" => {
                let parts: Vec<&str> = identity.split('.').collect();
                if parts.len() != 3 || parts.iter().any(|part| part.trim().is_empty()) {
                    return Err(format!(
                        "Invalid index object reference '{value}'. Use index:<schema>.<table>.<index>."
                    ));
                }
                safe_file_component(parts[0])?;
                safe_file_component(parts[1])?;
                safe_file_component(parts[2])?;
                Ok(Self::Index {
                    schema: parts[0].to_string(),
                    table: parts[1].to_string(),
                    index: parts[2].to_string(),
                })
            }
            "view" => {
                let (schema, view) = parse_two_part_object_ref(value, identity, "view")?;
                Ok(Self::View { schema, view })
            }
            "materializedView" => {
                let (schema, materialized_view) =
                    parse_two_part_object_ref(value, identity, "materializedView")?;
                Ok(Self::MaterializedView {
                    schema,
                    materialized_view,
                })
            }
            "constraint" => {
                let parts: Vec<&str> = identity.split('.').collect();
                if parts.len() != 3 || parts.iter().any(|part| part.trim().is_empty()) {
                    return Err(format!(
                        "Invalid constraint object reference '{value}'. Use constraint:<schema>.<table>.<constraint>."
                    ));
                }
                safe_file_component(parts[0])?;
                safe_file_component(parts[1])?;
                safe_file_component(parts[2])?;
                Ok(Self::Constraint {
                    schema: parts[0].to_string(),
                    table: parts[1].to_string(),
                    constraint: parts[2].to_string(),
                })
            }
            "function" => {
                let parts: Vec<&str> = identity.split('.').collect();
                if parts.len() != 3 || parts.iter().any(|part| part.trim().is_empty()) {
                    return Err(format!(
                        "Invalid function object reference '{value}'. Use function:<schema>.<function>.<signature>."
                    ));
                }
                safe_file_component(parts[0])?;
                safe_file_component(parts[1])?;
                safe_file_component(parts[2])?;
                Ok(Self::Function {
                    schema: parts[0].to_string(),
                    function: parts[1].to_string(),
                    signature: parts[2].to_string(),
                })
            }
            "trigger" => {
                let parts: Vec<&str> = identity.split('.').collect();
                if parts.len() != 3 || parts.iter().any(|part| part.trim().is_empty()) {
                    return Err(format!(
                        "Invalid trigger object reference '{value}'. Use trigger:<schema>.<relation>.<trigger>."
                    ));
                }
                safe_file_component(parts[0])?;
                safe_file_component(parts[1])?;
                safe_file_component(parts[2])?;
                Ok(Self::Trigger {
                    schema: parts[0].to_string(),
                    relation: parts[1].to_string(),
                    trigger: parts[2].to_string(),
                })
            }
            "grant" => {
                let parts: Vec<&str> = identity.split('.').collect();
                if parts.len() < 3 || parts.iter().any(|part| part.trim().is_empty()) {
                    return Err(format!(
                        "Invalid grant object reference '{value}'. Use grant:<target-kind>.<target-identity>.<grantee>."
                    ));
                }
                let target_kind = parts[0];
                safe_file_component(target_kind)?;
                match target_kind {
                    "schema" if parts.len() == 3 => {
                        safe_file_component(parts[1])?;
                        safe_file_component(parts[2])?;
                        Ok(Self::Grant {
                            target_kind: target_kind.to_string(),
                            schema: parts[1].to_string(),
                            object: None,
                            signature: None,
                            grantee: parts[2].to_string(),
                        })
                    }
                    "table" | "view" | "materializedView" | "sequence" if parts.len() == 4 => {
                        safe_file_component(parts[1])?;
                        safe_file_component(parts[2])?;
                        safe_file_component(parts[3])?;
                        Ok(Self::Grant {
                            target_kind: target_kind.to_string(),
                            schema: parts[1].to_string(),
                            object: Some(parts[2].to_string()),
                            signature: None,
                            grantee: parts[3].to_string(),
                        })
                    }
                    "function" if parts.len() == 5 => {
                        safe_file_component(parts[1])?;
                        safe_file_component(parts[2])?;
                        safe_file_component(parts[3])?;
                        safe_file_component(parts[4])?;
                        Ok(Self::Grant {
                            target_kind: target_kind.to_string(),
                            schema: parts[1].to_string(),
                            object: Some(parts[2].to_string()),
                            signature: Some(parts[3].to_string()),
                            grantee: parts[4].to_string(),
                        })
                    }
                    _ => Err(format!(
                        "Invalid grant object reference '{value}'. Use grant:<target-kind>.<target-identity>.<grantee>."
                    )),
                }
            }
            "rlsPolicy" => {
                let parts: Vec<&str> = identity.split('.').collect();
                if parts.len() != 3 || parts.iter().any(|part| part.trim().is_empty()) {
                    return Err(format!(
                        "Invalid RLS policy object reference '{value}'. Use rlsPolicy:<schema>.<table>.<policy>."
                    ));
                }
                safe_file_component(parts[0])?;
                safe_file_component(parts[1])?;
                safe_file_component(parts[2])?;
                Ok(Self::RlsPolicy {
                    schema: parts[0].to_string(),
                    table: parts[1].to_string(),
                    policy: parts[2].to_string(),
                })
            }
            _ => Err(format!(
                "Invalid object reference '{value}'. Use schema:<schema>, table:<schema>.<table>, extension:<name>, enum:<schema>.<name>, sequence:<schema>.<name>, index:<schema>.<table>.<name>, view:<schema>.<name>, materializedView:<schema>.<name>, constraint:<schema>.<table>.<name>, function:<schema>.<function>.<signature>, trigger:<schema>.<relation>.<trigger>, grant:<target-kind>.<target-identity>.<grantee>, or rlsPolicy:<schema>.<table>.<policy>."
            )),
        }
    }

    pub(crate) fn as_str(&self) -> String {
        match self {
            Self::Schema(schema) => format!("schema:{schema}"),
            Self::Table { schema, table } => format!("table:{schema}.{table}"),
            Self::Extension(extension) => format!("extension:{extension}"),
            Self::Enum { schema, enum_name } => format!("enum:{schema}.{enum_name}"),
            Self::Sequence { schema, sequence } => format!("sequence:{schema}.{sequence}"),
            Self::Index {
                schema,
                table,
                index,
            } => format!("index:{schema}.{table}.{index}"),
            Self::View { schema, view } => format!("view:{schema}.{view}"),
            Self::MaterializedView {
                schema,
                materialized_view,
            } => format!("materializedView:{schema}.{materialized_view}"),
            Self::Constraint {
                schema,
                table,
                constraint,
            } => {
                format!("constraint:{schema}.{table}.{constraint}")
            }
            Self::Function {
                schema,
                function,
                signature,
            } => format!("function:{schema}.{function}.{signature}"),
            Self::Trigger {
                schema,
                relation,
                trigger,
            } => format!("trigger:{schema}.{relation}.{trigger}"),
            Self::Grant {
                target_kind,
                schema,
                object,
                signature,
                grantee,
            } => match (object, signature) {
                (Some(object), Some(signature)) => {
                    format!("grant:{target_kind}.{schema}.{object}.{signature}.{grantee}")
                }
                (Some(object), None) => format!("grant:{target_kind}.{schema}.{object}.{grantee}"),
                (None, _) => format!("grant:{target_kind}.{schema}.{grantee}"),
            },
            Self::RlsPolicy {
                schema,
                table,
                policy,
            } => format!("rlsPolicy:{schema}.{table}.{policy}"),
        }
    }

    pub(crate) fn object_type(&self) -> &'static str {
        match self {
            Self::Schema(_) => "schema",
            Self::Table { .. } => "table",
            Self::Extension(_) => "extension",
            Self::Enum { .. } => "enum",
            Self::Sequence { .. } => "sequence",
            Self::Index { .. } => "index",
            Self::View { .. } => "view",
            Self::MaterializedView { .. } => "materializedView",
            Self::Constraint { .. } => "constraint",
            Self::Function { .. } => "function",
            Self::Trigger { .. } => "trigger",
            Self::Grant { .. } => "grant",
            Self::RlsPolicy { .. } => "rlsPolicy",
        }
    }

    pub(crate) fn required_schema_ref(&self) -> Option<Self> {
        match self {
            Self::Schema(_) | Self::Extension(_) => None,
            Self::Table { schema, .. } => Some(Self::Schema(schema.clone())),
            Self::Enum { schema, .. }
            | Self::Sequence { schema, .. }
            | Self::View { schema, .. }
            | Self::MaterializedView { schema, .. }
            | Self::Function { schema, .. }
            | Self::Trigger { schema, .. }
            | Self::Grant { schema, .. }
            | Self::RlsPolicy { schema, .. } => Some(Self::Schema(schema.clone())),
            Self::Index { schema, .. } | Self::Constraint { schema, .. } => {
                Some(Self::Schema(schema.clone()))
            }
        }
    }
}

pub(crate) fn parse_two_part_object_ref(
    value: &str,
    identity: &str,
    object_type: &str,
) -> Result<(String, String), String> {
    let Some((schema, name)) = identity.split_once('.') else {
        return Err(format!(
            "Invalid {object_type} object reference '{value}'. Use {object_type}:<schema>.<name>."
        ));
    };
    if schema.trim().is_empty() || name.trim().is_empty() {
        return Err(format!(
            "Invalid {object_type} object reference '{value}'. Use {object_type}:<schema>.<name>."
        ));
    }
    safe_file_component(schema)?;
    safe_file_component(name)?;
    Ok((schema.to_string(), name.to_string()))
}

pub(crate) fn object_ref_from_relative_path(relative_path: &str) -> Result<ObjectRef, String> {
    ensure_database_object_path(relative_path)?;
    if let Some(file_name) = relative_path.strip_prefix("database/objects/schemas/") {
        let Some(schema) = schema_name_from_file(file_name) else {
            return Err(format!(
                "Invalid schema desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Schema(schema));
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/tables/") {
        let Some((schema, table)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid table desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Table { schema, table });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/extensions/") {
        let Some(extension) = schema_name_from_file(file_name) else {
            return Err(format!(
                "Invalid extension desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Extension(extension));
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/enums/") {
        let Some((schema, enum_name)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid enum desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Enum { schema, enum_name });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/sequences/") {
        let Some((schema, sequence)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid sequence desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Sequence { schema, sequence });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/indexes/") {
        let Some((schema, table, index)) = three_part_name_from_file(file_name) else {
            return Err(format!(
                "Invalid index desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Index {
            schema,
            table,
            index,
        });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/views/") {
        let Some((schema, view)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid view desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::View { schema, view });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/materialized-views/") {
        let Some((schema, materialized_view)) = table_name_from_file(file_name) else {
            return Err(format!(
                "Invalid materialized view desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::MaterializedView {
            schema,
            materialized_view,
        });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/functions/") {
        let Some((schema, function, signature)) = three_part_name_from_file(file_name) else {
            return Err(format!(
                "Invalid function desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Function {
            schema,
            function,
            signature,
        });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/triggers/") {
        let Some((schema, relation, trigger)) = three_part_name_from_file(file_name) else {
            return Err(format!(
                "Invalid trigger desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::Trigger {
            schema,
            relation,
            trigger,
        });
    }
    if let Some(file_name) = relative_path.strip_prefix("database/objects/rls-policies/") {
        let Some((schema, table, policy)) = three_part_name_from_file(file_name) else {
            return Err(format!(
                "Invalid RLS policy desired-state file path: {relative_path}"
            ));
        };
        return Ok(ObjectRef::RlsPolicy {
            schema,
            table,
            policy,
        });
    }
    for (folder, target_kind) in [
        ("schemas", "schema"),
        ("tables", "table"),
        ("views", "view"),
        ("materialized-views", "materializedView"),
        ("sequences", "sequence"),
        ("functions", "function"),
    ] {
        let prefix = format!("database/objects/grants/{folder}/");
        if let Some(file_name) = relative_path.strip_prefix(&prefix) {
            return grant_ref_from_file_name(target_kind, file_name)
                .ok_or_else(|| format!("Invalid grant desired-state file path: {relative_path}"));
        }
    }
    for folder in [
        "primary-keys",
        "unique-constraints",
        "foreign-keys",
        "check-constraints",
    ] {
        let prefix = format!("database/objects/constraints/{folder}/");
        if let Some(file_name) = relative_path.strip_prefix(&prefix) {
            let Some((schema, table, constraint)) = three_part_name_from_file(file_name) else {
                return Err(format!(
                    "Invalid constraint desired-state file path: {relative_path}"
                ));
            };
            return Ok(ObjectRef::Constraint {
                schema,
                table,
                constraint,
            });
        }
    }
    Err(format!(
        "Unsupported desired-state file path: {relative_path}"
    ))
}

pub(crate) fn schema_file_path(schema: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/schemas/{}.sql",
        safe_file_component(schema)?
    ))
}

pub(crate) fn table_file_path(schema: &str, table: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/tables/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(table)?
    ))
}

pub(crate) fn extension_file_path(extension: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/extensions/{}.sql",
        safe_file_component(extension)?
    ))
}

pub(crate) fn enum_file_path(schema: &str, enum_name: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/enums/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(enum_name)?
    ))
}

pub(crate) fn sequence_file_path(schema: &str, sequence: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/sequences/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(sequence)?
    ))
}

pub(crate) fn index_file_path(schema: &str, table: &str, index: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/indexes/{}.{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(table)?,
        safe_file_component(index)?
    ))
}

pub(crate) fn view_file_path(schema: &str, view: &str) -> Result<String, String> {
    Ok(format!(
        "database/objects/views/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(view)?
    ))
}

pub(crate) fn materialized_view_file_path(
    schema: &str,
    materialized_view: &str,
) -> Result<String, String> {
    Ok(format!(
        "database/objects/materialized-views/{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(materialized_view)?
    ))
}

pub(crate) fn function_file_path(
    schema: &str,
    function: &str,
    identity_arguments: &str,
) -> Result<String, String> {
    Ok(format!(
        "database/objects/functions/{}.{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(function)?,
        function_identity_slug(identity_arguments)?
    ))
}

pub(crate) fn trigger_file_path(
    schema: &str,
    relation: &str,
    trigger: &str,
) -> Result<String, String> {
    Ok(format!(
        "database/objects/triggers/{}.{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(relation)?,
        safe_file_component(trigger)?
    ))
}

pub(crate) fn grant_file_path(
    target_kind: &str,
    schema: &str,
    object_name: Option<&str>,
    identity_arguments: Option<&str>,
    grantee: &str,
) -> Result<String, String> {
    let grantee = grant_grantee_file_token(grantee)?;
    match target_kind {
        "schema" => Ok(format!(
            "database/objects/grants/schemas/{}.{}.sql",
            safe_file_component(schema)?,
            grantee
        )),
        "table" => Ok(format!(
            "database/objects/grants/tables/{}.{}.{}.sql",
            safe_file_component(schema)?,
            safe_file_component(object_name.unwrap_or(""))?,
            grantee
        )),
        "view" => Ok(format!(
            "database/objects/grants/views/{}.{}.{}.sql",
            safe_file_component(schema)?,
            safe_file_component(object_name.unwrap_or(""))?,
            grantee
        )),
        "materializedView" => Ok(format!(
            "database/objects/grants/materialized-views/{}.{}.{}.sql",
            safe_file_component(schema)?,
            safe_file_component(object_name.unwrap_or(""))?,
            grantee
        )),
        "sequence" => Ok(format!(
            "database/objects/grants/sequences/{}.{}.{}.sql",
            safe_file_component(schema)?,
            safe_file_component(object_name.unwrap_or(""))?,
            grantee
        )),
        "function" => Ok(format!(
            "database/objects/grants/functions/{}.{}.{}.{}.sql",
            safe_file_component(schema)?,
            safe_file_component(object_name.unwrap_or(""))?,
            safe_file_component(identity_arguments.unwrap_or(""))?,
            grantee
        )),
        _ => Err(format!(
            "Unsupported PostgreSQL grant target kind for desired-state path: {target_kind}"
        )),
    }
}

pub(crate) fn rls_policy_file_path(
    schema: &str,
    table: &str,
    policy: &str,
) -> Result<String, String> {
    Ok(format!(
        "database/objects/rls-policies/{}.{}.{}.sql",
        safe_file_component(schema)?,
        safe_file_component(table)?,
        safe_file_component(policy)?
    ))
}

pub(crate) fn grant_grantee_file_token(grantee: &str) -> Result<String, String> {
    if grantee.eq_ignore_ascii_case("PUBLIC") {
        Ok("public".to_string())
    } else {
        safe_file_component(grantee)
    }
}

pub(crate) fn constraint_file_path(
    constraint_type: &str,
    schema: &str,
    table: &str,
    constraint: &str,
) -> Result<String, String> {
    let folder = constraint_folder(constraint_type)?;
    Ok(format!(
        "database/objects/constraints/{}/{}.{}.{}.sql",
        folder,
        safe_file_component(schema)?,
        safe_file_component(table)?,
        safe_file_component(constraint)?
    ))
}

pub(crate) fn constraint_folder(constraint_type: &str) -> Result<&'static str, String> {
    match constraint_type {
        "primaryKey" => Ok("primary-keys"),
        "uniqueConstraint" => Ok("unique-constraints"),
        "foreignKey" => Ok("foreign-keys"),
        "checkConstraint" => Ok("check-constraints"),
        _ => Err(format!(
            "Unsupported PostgreSQL constraint type for desired-state path: {constraint_type}"
        )),
    }
}

pub(crate) fn safe_file_component(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
    {
        return Err(format!(
            "Unsafe PostgreSQL object name for file path: {trimmed}"
        ));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn function_identity_slug(identity_arguments: &str) -> Result<String, String> {
    let trimmed = identity_arguments.trim();
    if trimmed.is_empty() {
        return Ok("no_args".to_string());
    }
    let mut slug = String::new();
    let mut last_was_separator = false;
    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            slug.push('_');
            last_was_separator = true;
        }
    }
    let slug = slug.trim_matches('_');
    if slug.is_empty() {
        return Err(format!(
            "Could not create a safe function identity slug for arguments: {trimmed}"
        ));
    }
    Ok(slug.chars().take(80).collect())
}

pub(crate) fn ensure_database_object_path(relative_path: &str) -> Result<(), String> {
    if relative_path.starts_with("database/objects/") {
        Ok(())
    } else {
        Err(format!(
            "Refusing to write outside database/objects/: {relative_path}"
        ))
    }
}

pub(crate) fn object_key(object: &DesiredStateObject) -> String {
    match object.object_type {
        RepositoryObjectType::Schema => schema_key(&object.schema_name),
        RepositoryObjectType::Table => table_key(
            &object.schema_name,
            object.table_name.as_deref().unwrap_or(""),
        ),
        RepositoryObjectType::Extension => extension_key(&object.object_name),
        RepositoryObjectType::Enum => enum_key(&object.schema_name, &object.object_name),
        RepositoryObjectType::Sequence => sequence_key(&object.schema_name, &object.object_name),
        RepositoryObjectType::Index => index_key(
            &object.schema_name,
            object.parent_name.as_deref().unwrap_or(""),
            &object.object_name,
        ),
        RepositoryObjectType::View => view_key(&object.schema_name, &object.object_name),
        RepositoryObjectType::MaterializedView => {
            materialized_view_key(&object.schema_name, &object.object_name)
        }
        RepositoryObjectType::Constraint => constraint_key(
            &object.schema_name,
            object.parent_name.as_deref().unwrap_or(""),
            &object.object_name,
        ),
        RepositoryObjectType::Function => function_key(&object.schema_name, &object.object_name),
        RepositoryObjectType::Trigger => trigger_key(
            &object.schema_name,
            object.parent_name.as_deref().unwrap_or(""),
            &object.object_name,
        ),
        RepositoryObjectType::Grant => grant_key(&object.object_name),
        RepositoryObjectType::RlsPolicy => rls_policy_key(
            &object.schema_name,
            object.parent_name.as_deref().unwrap_or(""),
            &object.object_name,
        ),
    }
}

pub(crate) fn schema_key(schema: &str) -> String {
    format!("schema:{schema}")
}

pub(crate) fn table_key(schema: &str, table: &str) -> String {
    format!("table:{schema}.{table}")
}

pub(crate) fn extension_key(extension: &str) -> String {
    format!("extension:{extension}")
}

pub(crate) fn enum_key(schema: &str, enum_name: &str) -> String {
    format!("enum:{schema}.{enum_name}")
}

pub(crate) fn sequence_key(schema: &str, sequence: &str) -> String {
    format!("sequence:{schema}.{sequence}")
}

pub(crate) fn index_key(schema: &str, table: &str, index: &str) -> String {
    format!("index:{schema}.{table}.{index}")
}

pub(crate) fn view_key(schema: &str, view: &str) -> String {
    format!("view:{schema}.{view}")
}

pub(crate) fn materialized_view_key(schema: &str, materialized_view: &str) -> String {
    format!("materializedView:{schema}.{materialized_view}")
}

pub(crate) fn constraint_key(schema: &str, table: &str, constraint: &str) -> String {
    format!("constraint:{schema}.{table}.{constraint}")
}

pub(crate) fn function_key(schema: &str, object_name: &str) -> String {
    format!("function:{schema}.{object_name}")
}

pub(crate) fn trigger_key(schema: &str, relation: &str, trigger: &str) -> String {
    format!("trigger:{schema}.{relation}.{trigger}")
}

pub(crate) fn grant_key(identity: &str) -> String {
    format!("grant:{identity}")
}

pub(crate) fn rls_policy_key(schema: &str, table: &str, policy: &str) -> String {
    format!("rlsPolicy:{schema}.{table}.{policy}")
}

pub(crate) fn grant_ref_from_file_name(target_kind: &str, file_name: &str) -> Option<ObjectRef> {
    let stem = file_name.strip_suffix(".sql")?;
    let parts: Vec<&str> = stem.split('.').collect();
    match target_kind {
        "schema" if parts.len() == 2 => Some(ObjectRef::Grant {
            target_kind: target_kind.to_string(),
            schema: parts[0].to_string(),
            object: None,
            signature: None,
            grantee: parts[1].to_string(),
        }),
        "table" | "view" | "materializedView" | "sequence" if parts.len() == 3 => {
            Some(ObjectRef::Grant {
                target_kind: target_kind.to_string(),
                schema: parts[0].to_string(),
                object: Some(parts[1].to_string()),
                signature: None,
                grantee: parts[2].to_string(),
            })
        }
        "function" if parts.len() == 4 => Some(ObjectRef::Grant {
            target_kind: target_kind.to_string(),
            schema: parts[0].to_string(),
            object: Some(parts[1].to_string()),
            signature: Some(parts[2].to_string()),
            grantee: parts[3].to_string(),
        }),
        _ => None,
    }
}

pub(crate) fn schema_name_from_file(file_name: &str) -> Option<String> {
    file_name
        .strip_suffix(".sql")
        .filter(|stem| !stem.is_empty())
        .map(|stem| stem.to_string())
}

pub(crate) fn table_name_from_file(file_name: &str) -> Option<(String, String)> {
    let stem = file_name.strip_suffix(".sql")?;
    let parts: Vec<&str> = stem.split('.').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some((parts[0].to_string(), parts[1].to_string()))
}

pub(crate) fn three_part_name_from_file(file_name: &str) -> Option<(String, String, String)> {
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
