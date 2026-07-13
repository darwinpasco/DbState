use crate::postgres::inventory::PostgresInventory;
use crate::*;
use ::postgres::{Client, NoTls};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaInfo {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInfo {
    pub schema_name: String,
    pub table_name: String,
    pub table_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfo {
    pub schema_name: String,
    pub table_name: String,
    pub column_name: String,
    pub ordinal_position: i32,
    pub data_type: String,
    pub is_nullable: bool,
    pub has_default: bool,
    pub default_expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInfo {
    pub extension_name: String,
    pub schema_name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumInfo {
    pub schema_name: String,
    pub enum_name: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceInfo {
    pub schema_name: String,
    pub sequence_name: String,
    pub data_type: Option<String>,
    pub start_value: Option<i64>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub increment_by: Option<i64>,
    pub cycle: bool,
    pub cache_size: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexInfo {
    pub schema_name: String,
    pub table_name: String,
    pub index_name: String,
    pub is_unique: bool,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInfo {
    pub schema_name: String,
    pub view_name: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintInfo {
    pub schema_name: String,
    pub table_name: String,
    pub constraint_name: String,
    pub constraint_type: String,
    pub definition: String,
    pub columns: Vec<String>,
    pub referenced_schema: Option<String>,
    pub referenced_table: Option<String>,
    pub referenced_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInfo {
    pub schema_name: String,
    pub function_name: String,
    pub identity_arguments: String,
    pub result_type: Option<String>,
    pub language: Option<String>,
    pub volatility: Option<String>,
    pub security_definer: bool,
    pub is_strict: bool,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionCounts {
    pub schemas: usize,
    pub tables: usize,
    pub columns: usize,
    pub extensions: usize,
    pub enums: usize,
    pub sequences: usize,
    pub indexes: usize,
    pub views: usize,
    pub constraints: usize,
    pub functions: usize,
}

#[derive(Debug, Clone)]
pub struct InspectionReport {
    pub command: CommandKind,
    pub success: bool,
    pub database_type: String,
    pub inspection_scope: Vec<String>,
    pub schemas: Vec<SchemaInfo>,
    pub tables: Vec<TableInfo>,
    pub columns: Vec<ColumnInfo>,
    pub extensions: Vec<ExtensionInfo>,
    pub enums: Vec<EnumInfo>,
    pub sequences: Vec<SequenceInfo>,
    pub indexes: Vec<IndexInfo>,
    pub views: Vec<ViewInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub functions: Vec<FunctionInfo>,
    pub counts: InspectionCounts,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub deferred_object_types: Vec<String>,
}

pub fn inspect_postgres_command(
    cli_url: Option<String>,
    env_url: Option<String>,
) -> InspectionReport {
    inspect_postgres_scoped_command(cli_url, env_url, None, None)
}

pub(crate) fn inspect_postgres_scoped_command(
    cli_url: Option<String>,
    env_url: Option<String>,
    schema: Option<String>,
    table: Option<String>,
) -> InspectionReport {
    let mut report = empty_inspection_report(CommandKind::InspectPostgres);

    let Some(connection_url) = resolve_postgres_url(cli_url, env_url) else {
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
        Ok(inventory) => {
            report.schemas = inventory.schemas;
            report.tables = inventory.tables;
            report.columns = inventory.columns;
            report.extensions = inventory.extensions;
            report.enums = inventory.enums;
            report.sequences = inventory.sequences;
            report.indexes = inventory.indexes;
            report.views = inventory.views;
            report.constraints = inventory.constraints;
            report.functions = inventory.functions;
            if let Err(error) = apply_inspection_scope(&mut report, schema, table) {
                report.success = false;
                report.errors.push(error);
                return report;
            }
            report.counts = InspectionCounts {
                schemas: report.schemas.len(),
                tables: report.tables.len(),
                columns: report.columns.len(),
                extensions: report.extensions.len(),
                enums: report.enums.len(),
                sequences: report.sequences.len(),
                indexes: report.indexes.len(),
                views: report.views.len(),
                constraints: report.constraints.len(),
                functions: report.functions.len(),
            };
            report.success = true;
        }
        Err(error) => {
            report.errors.push(redact_message(&error, &connection_url));
        }
    }

    report
}

fn apply_inspection_scope(
    report: &mut InspectionReport,
    schema: Option<String>,
    table: Option<String>,
) -> Result<(), String> {
    if let Some(schema) = schema {
        if schema.trim().is_empty() {
            return Err("--schema cannot be empty.".to_string());
        }
        if !report.schemas.iter().any(|item| item.name == schema) {
            return Err(format!(
                "Selected schema '{schema}' was not found in the PostgreSQL inventory."
            ));
        }
        report.schemas.retain(|item| item.name == schema);
        report.tables.retain(|item| item.schema_name == schema);
        report.columns.retain(|item| item.schema_name == schema);
        report.enums.retain(|item| item.schema_name == schema);
        report.sequences.retain(|item| item.schema_name == schema);
        report.indexes.retain(|item| item.schema_name == schema);
        report.views.retain(|item| item.schema_name == schema);
        report.constraints.retain(|item| item.schema_name == schema);
        report.functions.retain(|item| item.schema_name == schema);
        report.inspection_scope = vec![format!("schema:{schema}")];
        return Ok(());
    }

    if let Some(table) = table {
        let Some((schema, table_name)) = table.split_once('.') else {
            return Err(
                "--table must use schema-qualified form such as public.example_table.".to_string(),
            );
        };
        if schema.trim().is_empty() || table_name.trim().is_empty() {
            return Err(
                "--table must use schema-qualified form such as public.example_table.".to_string(),
            );
        }
        if !report
            .tables
            .iter()
            .any(|item| item.schema_name == schema && item.table_name == table_name)
        {
            return Err(format!(
                "Selected table '{schema}.{table_name}' was not found in the PostgreSQL inventory."
            ));
        }
        report.schemas.retain(|item| item.name == schema);
        report
            .tables
            .retain(|item| item.schema_name == schema && item.table_name == table_name);
        report
            .columns
            .retain(|item| item.schema_name == schema && item.table_name == table_name);
        report.enums.clear();
        report.sequences.clear();
        report.views.clear();
        report
            .indexes
            .retain(|item| item.schema_name == schema && item.table_name == table_name);
        report
            .constraints
            .retain(|item| item.schema_name == schema && item.table_name == table_name);
        report.functions.clear();
        report.inspection_scope = vec![format!("table:{schema}.{table_name}")];
    }

    Ok(())
}

pub(crate) fn resolve_postgres_url(
    cli_url: Option<String>,
    env_url: Option<String>,
) -> Option<String> {
    cli_url
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env_url.filter(|value| !value.trim().is_empty()))
}

pub(crate) fn is_postgres_connection_url(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://")
}

pub(crate) fn invalid_postgres_url_message() -> String {
    "Invalid PostgreSQL connection URL. Provide a postgres:// or postgresql:// URL.".to_string()
}

pub fn inspect_postgres(connection_url: &str) -> Result<PostgresInventory, String> {
    let mut client = Client::connect(connection_url, NoTls).map_err(|_| {
        "PostgreSQL connection failed. Verify the session-only connection URL, credentials, network, and database availability.".to_string()
    })?;

    let schema_rows = client
        .query(
            "SELECT nspname
             FROM pg_catalog.pg_namespace
             WHERE nspname <> 'pg_catalog'
               AND nspname <> 'information_schema'
               AND nspname NOT LIKE 'pg_toast%'
               AND nspname NOT LIKE 'pg_%'
             ORDER BY nspname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading schemas.".to_string())?;

    let table_rows = client
        .query(
            "SELECT n.nspname,
                    c.relname,
                    CASE c.relkind
                        WHEN 'r' THEN 'BASE TABLE'
                        WHEN 'p' THEN 'PARTITIONED TABLE'
                        ELSE c.relkind::text
                    END
             FROM pg_catalog.pg_class c
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relkind IN ('r', 'p')
               AND n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, c.relname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading tables.".to_string())?;

    let column_rows = client
        .query(
            "SELECT n.nspname,
                    c.relname,
                    a.attname,
                    a.attnum::int4,
                    pg_catalog.format_type(a.atttypid, a.atttypmod),
                    NOT a.attnotnull,
                    pg_catalog.pg_get_expr(ad.adbin, ad.adrelid) IS NOT NULL,
                    pg_catalog.pg_get_expr(ad.adbin, ad.adrelid)
             FROM pg_catalog.pg_attribute a
             JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
             LEFT JOIN pg_catalog.pg_attrdef ad
               ON ad.adrelid = a.attrelid
              AND ad.adnum = a.attnum
             WHERE a.attnum > 0
               AND NOT a.attisdropped
               AND c.relkind IN ('r', 'p')
               AND n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, c.relname, a.attnum",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading columns.".to_string())?;

    let extension_rows = client
        .query(
            "SELECT e.extname::text, n.nspname::text, e.extversion::text
             FROM pg_catalog.pg_extension e
             LEFT JOIN pg_catalog.pg_namespace n ON n.oid = e.extnamespace
             WHERE e.extname <> 'plpgsql'
             ORDER BY e.extname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading extensions.".to_string())?;

    let enum_rows = client
        .query(
            "SELECT n.nspname::text, t.typname::text, e.enumlabel::text
             FROM pg_catalog.pg_type t
             JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
             JOIN pg_catalog.pg_enum e ON e.enumtypid = t.oid
             WHERE n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, t.typname, e.enumsortorder",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading enums.".to_string())?;

    let sequence_rows = client
        .query(
            "SELECT schemaname::text,
                    sequencename::text,
                    data_type::text,
                    start_value,
                    min_value,
                    max_value,
                    increment_by,
                    cycle,
                    cache_size
             FROM pg_catalog.pg_sequences
             WHERE schemaname <> 'pg_catalog'
               AND schemaname <> 'information_schema'
               AND schemaname NOT LIKE 'pg_toast%'
               AND schemaname NOT LIKE 'pg_%'
             ORDER BY schemaname, sequencename",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading sequences.".to_string())?;

    let index_rows = client
        .query(
            "SELECT ns.nspname::text,
                    tbl.relname::text,
                    idx.relname::text,
                    i.indisunique,
                    pg_catalog.pg_get_indexdef(idx.oid)
             FROM pg_catalog.pg_index i
             JOIN pg_catalog.pg_class idx ON idx.oid = i.indexrelid
             JOIN pg_catalog.pg_class tbl ON tbl.oid = i.indrelid
             JOIN pg_catalog.pg_namespace ns ON ns.oid = tbl.relnamespace
             WHERE tbl.relkind IN ('r', 'p')
               AND ns.nspname <> 'pg_catalog'
               AND ns.nspname <> 'information_schema'
               AND ns.nspname NOT LIKE 'pg_toast%'
               AND ns.nspname NOT LIKE 'pg_%'
               AND NOT EXISTS (
                   SELECT 1 FROM pg_catalog.pg_constraint c
                   WHERE c.conindid = idx.oid
               )
             ORDER BY ns.nspname, tbl.relname, idx.relname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading indexes.".to_string())?;

    let view_rows = client
        .query(
            "SELECT n.nspname::text,
                    c.relname::text,
                    pg_catalog.pg_get_viewdef(c.oid, true)::text
             FROM pg_catalog.pg_class c
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relkind = 'v'
               AND n.nspname <> 'pg_catalog'
               AND n.nspname <> 'information_schema'
               AND n.nspname NOT LIKE 'pg_toast%'
               AND n.nspname NOT LIKE 'pg_%'
             ORDER BY n.nspname, c.relname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading views.".to_string())?;

    let constraint_rows = client
        .query(
            "SELECT ns.nspname::text,
                    tbl.relname::text,
                    con.conname::text,
                    con.contype::text,
                    pg_catalog.pg_get_constraintdef(con.oid, true)::text,
                    COALESCE(array_agg(att.attname::text ORDER BY key_position.ordinality)
                        FILTER (WHERE att.attname IS NOT NULL), ARRAY[]::text[]),
                    ref_ns.nspname::text,
                    ref_tbl.relname::text,
                    COALESCE(array_agg(ref_att.attname::text ORDER BY key_position.ordinality)
                        FILTER (WHERE ref_att.attname IS NOT NULL), ARRAY[]::text[])
             FROM pg_catalog.pg_constraint con
             JOIN pg_catalog.pg_class tbl ON tbl.oid = con.conrelid
             JOIN pg_catalog.pg_namespace ns ON ns.oid = tbl.relnamespace
             LEFT JOIN pg_catalog.pg_class ref_tbl ON ref_tbl.oid = con.confrelid AND con.confrelid <> 0
             LEFT JOIN pg_catalog.pg_namespace ref_ns ON ref_ns.oid = ref_tbl.relnamespace
             LEFT JOIN LATERAL unnest(con.conkey) WITH ORDINALITY AS key_position(attnum, ordinality) ON true
             LEFT JOIN pg_catalog.pg_attribute att
               ON att.attrelid = con.conrelid
              AND att.attnum = key_position.attnum
             LEFT JOIN pg_catalog.pg_attribute ref_att
               ON ref_att.attrelid = con.confrelid
              AND ref_att.attnum = con.confkey[key_position.ordinality]
             WHERE con.contype IN ('p', 'u', 'f', 'c')
               AND tbl.relkind IN ('r', 'p')
               AND ns.nspname <> 'pg_catalog'
               AND ns.nspname <> 'information_schema'
               AND ns.nspname NOT LIKE 'pg_toast%'
               AND ns.nspname NOT LIKE 'pg_%'
             GROUP BY ns.nspname, tbl.relname, con.conname, con.contype, con.oid,
                      ref_ns.nspname, ref_tbl.relname
             ORDER BY ns.nspname, tbl.relname, con.contype, con.conname",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading constraints.".to_string())?;

    let function_rows = client
        .query(
            "SELECT ns.nspname::text,
                    proc.proname::text,
                    pg_catalog.pg_get_function_identity_arguments(proc.oid)::text,
                    pg_catalog.pg_get_function_result(proc.oid)::text,
                    lang.lanname::text,
                    CASE proc.provolatile
                        WHEN 'i' THEN 'immutable'
                        WHEN 's' THEN 'stable'
                        WHEN 'v' THEN 'volatile'
                        ELSE proc.provolatile::text
                    END::text,
                    proc.prosecdef::bool,
                    proc.proisstrict::bool,
                    pg_catalog.pg_get_functiondef(proc.oid)::text
             FROM pg_catalog.pg_proc proc
             JOIN pg_catalog.pg_namespace ns ON ns.oid = proc.pronamespace
             JOIN pg_catalog.pg_language lang ON lang.oid = proc.prolang
             WHERE proc.prokind = 'f'
               AND ns.nspname <> 'pg_catalog'
               AND ns.nspname <> 'information_schema'
               AND ns.nspname NOT LIKE 'pg_toast%'
               AND ns.nspname NOT LIKE 'pg_%'
             ORDER BY ns.nspname,
                      proc.proname,
                      pg_catalog.pg_get_function_identity_arguments(proc.oid)",
            &[],
        )
        .map_err(|_| "PostgreSQL schema inspection failed while reading functions.".to_string())?;

    let schemas = schema_rows
        .into_iter()
        .map(|row| SchemaInfo { name: row.get(0) })
        .collect();

    let tables = table_rows
        .into_iter()
        .map(|row| TableInfo {
            schema_name: row.get(0),
            table_name: row.get(1),
            table_type: row.get(2),
        })
        .collect();

    let columns = column_rows
        .into_iter()
        .map(|row| ColumnInfo {
            schema_name: row.get(0),
            table_name: row.get(1),
            column_name: row.get(2),
            ordinal_position: row.get(3),
            data_type: row.get(4),
            is_nullable: row.get(5),
            has_default: row.get(6),
            default_expression: row.get(7),
        })
        .collect();

    let mut extensions = Vec::new();
    for row in extension_rows {
        extensions.push(ExtensionInfo {
            extension_name: try_get_catalog_string(&row, 0, "extension name")?,
            schema_name: try_get_catalog_optional_string(&row, 1, "extension schema")?,
            version: try_get_catalog_optional_string(&row, 2, "extension version")?,
        });
    }

    let mut enum_map: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for row in enum_rows {
        enum_map
            .entry((
                try_get_catalog_string(&row, 0, "enum schema")?,
                try_get_catalog_string(&row, 1, "enum name")?,
            ))
            .or_default()
            .push(try_get_catalog_string(&row, 2, "enum label")?);
    }
    let enums = enum_map
        .into_iter()
        .map(|((schema_name, enum_name), labels)| EnumInfo {
            schema_name,
            enum_name,
            labels,
        })
        .collect();

    let mut sequences = Vec::new();
    for row in sequence_rows {
        sequences.push(SequenceInfo {
            schema_name: try_get_catalog_string(&row, 0, "sequence schema")?,
            sequence_name: try_get_catalog_string(&row, 1, "sequence name")?,
            data_type: try_get_catalog_optional_string(&row, 2, "sequence data type")?,
            start_value: try_get_catalog_optional_i64(&row, 3, "sequence start value")?,
            min_value: try_get_catalog_optional_i64(&row, 4, "sequence min value")?,
            max_value: try_get_catalog_optional_i64(&row, 5, "sequence max value")?,
            increment_by: try_get_catalog_optional_i64(&row, 6, "sequence increment")?,
            cycle: try_get_catalog_bool(&row, 7, "sequence cycle")?,
            cache_size: try_get_catalog_optional_i64(&row, 8, "sequence cache size")?,
        });
    }

    let mut indexes = Vec::new();
    for row in index_rows {
        indexes.push(IndexInfo {
            schema_name: try_get_catalog_string(&row, 0, "index schema")?,
            table_name: try_get_catalog_string(&row, 1, "index table")?,
            index_name: try_get_catalog_string(&row, 2, "index name")?,
            is_unique: try_get_catalog_bool(&row, 3, "index uniqueness")?,
            definition: try_get_catalog_string(&row, 4, "index definition")?,
        });
    }

    let mut views = Vec::new();
    for row in view_rows {
        views.push(ViewInfo {
            schema_name: try_get_catalog_string(&row, 0, "view schema")?,
            view_name: try_get_catalog_string(&row, 1, "view name")?,
            definition: try_get_catalog_string(&row, 2, "view definition")?,
        });
    }

    let mut constraints = Vec::new();
    for row in constraint_rows {
        let constraint_type = match try_get_catalog_string(&row, 3, "constraint type")?.as_str() {
            "p" => "primaryKey",
            "u" => "uniqueConstraint",
            "f" => "foreignKey",
            "c" => "checkConstraint",
            _ => "unknownConstraint",
        }
        .to_string();
        constraints.push(ConstraintInfo {
            schema_name: try_get_catalog_string(&row, 0, "constraint schema")?,
            table_name: try_get_catalog_string(&row, 1, "constraint table")?,
            constraint_name: try_get_catalog_string(&row, 2, "constraint name")?,
            constraint_type,
            definition: try_get_catalog_string(&row, 4, "constraint definition")?,
            columns: try_get_catalog_string_array(&row, 5, "constraint columns")?,
            referenced_schema: try_get_catalog_optional_string(&row, 6, "referenced schema")?,
            referenced_table: try_get_catalog_optional_string(&row, 7, "referenced table")?,
            referenced_columns: try_get_catalog_string_array(&row, 8, "referenced columns")?,
        });
    }

    let mut functions = Vec::new();
    for row in function_rows {
        functions.push(FunctionInfo {
            schema_name: try_get_catalog_string(&row, 0, "function schema")?,
            function_name: try_get_catalog_string(&row, 1, "function name")?,
            identity_arguments: try_get_catalog_string(&row, 2, "function identity arguments")?,
            result_type: try_get_catalog_optional_string(&row, 3, "function result type")?,
            language: try_get_catalog_optional_string(&row, 4, "function language")?,
            volatility: try_get_catalog_optional_string(&row, 5, "function volatility")?,
            security_definer: try_get_catalog_bool(&row, 6, "function security definer")?,
            is_strict: try_get_catalog_bool(&row, 7, "function strictness")?,
            definition: try_get_catalog_string(&row, 8, "function definition")?,
        });
    }

    Ok(PostgresInventory {
        schemas,
        tables,
        columns,
        extensions,
        enums,
        sequences,
        indexes,
        views,
        constraints,
        functions,
    })
}

fn try_get_catalog_string(
    row: &::postgres::Row,
    index: usize,
    field: &str,
) -> Result<String, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn try_get_catalog_optional_string(
    row: &::postgres::Row,
    index: usize,
    field: &str,
) -> Result<Option<String>, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn try_get_catalog_optional_i64(
    row: &::postgres::Row,
    index: usize,
    field: &str,
) -> Result<Option<i64>, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn try_get_catalog_bool(row: &::postgres::Row, index: usize, field: &str) -> Result<bool, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

fn try_get_catalog_string_array(
    row: &::postgres::Row,
    index: usize,
    field: &str,
) -> Result<Vec<String>, String> {
    row.try_get(index).map_err(|error| {
        format!("PostgreSQL schema inspection failed while decoding {field}: {error}")
    })
}

pub(crate) fn empty_inspection_report(command: CommandKind) -> InspectionReport {
    InspectionReport {
        command,
        success: false,
        database_type: "postgresql".to_string(),
        inspection_scope: vec![
            "schemas".to_string(),
            "tables".to_string(),
            "columns".to_string(),
            "extensions".to_string(),
            "enums".to_string(),
            "sequences".to_string(),
            "indexes".to_string(),
            "views".to_string(),
            "constraints".to_string(),
            "functions".to_string(),
        ],
        schemas: Vec::new(),
        tables: Vec::new(),
        columns: Vec::new(),
        extensions: Vec::new(),
        enums: Vec::new(),
        sequences: Vec::new(),
        indexes: Vec::new(),
        views: Vec::new(),
        constraints: Vec::new(),
        functions: Vec::new(),
        counts: InspectionCounts {
            schemas: 0,
            tables: 0,
            columns: 0,
            extensions: 0,
            enums: 0,
            sequences: 0,
            indexes: 0,
            views: 0,
            constraints: 0,
            functions: 0,
        },
        warnings: Vec::new(),
        errors: Vec::new(),
        deferred_object_types: DEFERRED_OBJECT_TYPES
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

pub fn is_user_schema(schema_name: &str) -> bool {
    schema_name != "pg_catalog"
        && schema_name != "information_schema"
        && !schema_name.starts_with("pg_toast")
        && !schema_name.starts_with("pg_")
}

impl InspectionReport {
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(text, "Command: {}", self.command.as_str()).ok();
        writeln!(text, "Success: {}", self.success).ok();
        writeln!(
            text,
            "Inspection scope: {}",
            self.inspection_scope.join(", ")
        )
        .ok();
        writeln!(text, "Schema count: {}", self.counts.schemas).ok();
        writeln!(text, "Table count: {}", self.counts.tables).ok();
        writeln!(text, "Column count: {}", self.counts.columns).ok();
        writeln!(text, "Extension count: {}", self.counts.extensions).ok();
        writeln!(text, "Enum count: {}", self.counts.enums).ok();
        writeln!(text, "Sequence count: {}", self.counts.sequences).ok();
        writeln!(text, "Index count: {}", self.counts.indexes).ok();
        writeln!(text, "View count: {}", self.counts.views).ok();
        writeln!(text, "Constraint count: {}", self.counts.constraints).ok();
        writeln!(text, "Function count: {}", self.counts.functions).ok();
        writeln!(text, "Schemas:").ok();
        for schema in &self.schemas {
            writeln!(text, "  - {}", schema.name).ok();
        }
        writeln!(text, "Tables:").ok();
        for table in &self.tables {
            writeln!(
                text,
                "  - {}.{} ({})",
                table.schema_name, table.table_name, table.table_type
            )
            .ok();
        }
        writeln!(text, "Extensions:").ok();
        for extension in &self.extensions {
            writeln!(text, "  - {}", extension.extension_name).ok();
        }
        writeln!(text, "Enums:").ok();
        for enum_info in &self.enums {
            writeln!(
                text,
                "  - {}.{}",
                enum_info.schema_name, enum_info.enum_name
            )
            .ok();
        }
        writeln!(text, "Sequences:").ok();
        for sequence in &self.sequences {
            writeln!(
                text,
                "  - {}.{}",
                sequence.schema_name, sequence.sequence_name
            )
            .ok();
        }
        writeln!(text, "Indexes:").ok();
        for index in &self.indexes {
            writeln!(
                text,
                "  - {}.{}.{}",
                index.schema_name, index.table_name, index.index_name
            )
            .ok();
        }
        writeln!(text, "Views:").ok();
        for view in &self.views {
            writeln!(text, "  - {}.{}", view.schema_name, view.view_name).ok();
        }
        writeln!(text, "Constraints:").ok();
        for constraint in &self.constraints {
            writeln!(
                text,
                "  - {}.{}.{} ({})",
                constraint.schema_name,
                constraint.table_name,
                constraint.constraint_name,
                constraint.constraint_type
            )
            .ok();
        }
        writeln!(text, "Functions:").ok();
        for function in &self.functions {
            writeln!(
                text,
                "  - {}.{}({})",
                function.schema_name, function.function_name, function.identity_arguments
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
        write_json_string_field(&mut json, "databaseType", &self.database_type, false);
        write_json_array_field(&mut json, "inspectionScope", &self.inspection_scope);
        write_schema_array_field(&mut json, "schemas", &self.schemas);
        write_table_array_field(&mut json, "tables", &self.tables);
        write_column_array_field(&mut json, "columns", &self.columns);
        write_extension_array_field(&mut json, "extensions", &self.extensions);
        write_enum_array_field(&mut json, "enums", &self.enums);
        write_sequence_array_field(&mut json, "sequences", &self.sequences);
        write_index_array_field(&mut json, "indexes", &self.indexes);
        write_view_array_field(&mut json, "views", &self.views);
        write_constraint_array_field(&mut json, "constraints", &self.constraints);
        write_function_array_field(&mut json, "functions", &self.functions);
        write_counts_field(&mut json, "counts", &self.counts);
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
