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
    Constraint,
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
    Constraint {
        schema: String,
        table: String,
        constraint: String,
    },
}

impl ObjectRef {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let Some((object_type, identity)) = value.split_once(':') else {
            return Err(format!(
                "Invalid object reference '{value}'. Use schema:<schema>, table:<schema>.<table>, extension:<name>, enum:<schema>.<name>, sequence:<schema>.<name>, index:<schema>.<table>.<name>, or view:<schema>.<name>."
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
            _ => Err(format!(
                "Invalid object reference '{value}'. Use schema:<schema>, table:<schema>.<table>, extension:<name>, enum:<schema>.<name>, sequence:<schema>.<name>, index:<schema>.<table>.<name>, view:<schema>.<name>, or constraint:<schema>.<table>.<name>."
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
            Self::Constraint {
                schema,
                table,
                constraint,
            } => {
                format!("constraint:{schema}.{table}.{constraint}")
            }
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
            Self::Constraint { .. } => "constraint",
        }
    }

    pub(crate) fn required_schema_ref(&self) -> Option<Self> {
        match self {
            Self::Schema(_) | Self::Extension(_) => None,
            Self::Table { schema, .. } => Some(Self::Schema(schema.clone())),
            Self::Enum { schema, .. }
            | Self::Sequence { schema, .. }
            | Self::View { schema, .. } => Some(Self::Schema(schema.clone())),
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
        RepositoryObjectType::Constraint => constraint_key(
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

pub(crate) fn constraint_key(schema: &str, table: &str, constraint: &str) -> String {
    format!("constraint:{schema}.{table}.{constraint}")
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
