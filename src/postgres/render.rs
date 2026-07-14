use crate::postgres::inspect::{
    ColumnInfo, ConstraintInfo, EnumInfo, ExtensionInfo, FunctionInfo, GrantInfo, IndexInfo,
    MaterializedViewInfo, SequenceInfo, TriggerInfo, ViewInfo,
};
use std::fmt::Write as _;

pub fn quote_postgres_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub fn render_schema_sql(schema: &str) -> String {
    format!(
        "-- DbState PostgreSQL desired-state object\n-- Object type: schema\n-- Object name: {schema}\n\nCREATE SCHEMA {};\n",
        quote_postgres_identifier(schema)
    )
}

pub fn render_table_sql(schema: &str, table: &str, columns: &[ColumnInfo]) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: table").ok();
    writeln!(sql, "-- Object name: {schema}.{table}").ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE TABLE {}.{} (",
        quote_postgres_identifier(schema),
        quote_postgres_identifier(table)
    )
    .ok();

    let mut sorted_columns = columns.to_vec();
    sorted_columns.sort_by_key(|column| column.ordinal_position);
    for (index, column) in sorted_columns.iter().enumerate() {
        let comma = if index + 1 == sorted_columns.len() {
            ""
        } else {
            ","
        };
        write!(
            sql,
            "    {} {}",
            quote_postgres_identifier(&column.column_name),
            column.data_type
        )
        .ok();
        if column.has_default {
            if let Some(default_expression) = &column.default_expression {
                write!(sql, " DEFAULT {default_expression}").ok();
            }
        }
        if !column.is_nullable {
            write!(sql, " NOT NULL").ok();
        }
        writeln!(sql, "{comma}").ok();
    }
    writeln!(sql, ");").ok();
    sql
}

pub fn render_extension_sql(extension: &ExtensionInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: extension").ok();
    writeln!(sql, "-- Object name: {}", extension.extension_name).ok();
    if let Some(schema) = &extension.schema_name {
        writeln!(sql, "-- Extension schema: {schema}").ok();
    }
    if let Some(version) = &extension.version {
        writeln!(sql, "-- Extension version observed: {version}").ok();
    }
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE EXTENSION IF NOT EXISTS {};",
        quote_postgres_identifier(&extension.extension_name)
    )
    .ok();
    sql
}

pub fn render_enum_sql(enum_info: &EnumInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: enum").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}",
        enum_info.schema_name, enum_info.enum_name
    )
    .ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE TYPE {}.{} AS ENUM (",
        quote_postgres_identifier(&enum_info.schema_name),
        quote_postgres_identifier(&enum_info.enum_name)
    )
    .ok();
    for (index, label) in enum_info.labels.iter().enumerate() {
        let comma = if index + 1 == enum_info.labels.len() {
            ""
        } else {
            ","
        };
        writeln!(sql, "    '{}'{comma}", label.replace('\'', "''")).ok();
    }
    writeln!(sql, ");").ok();
    sql
}

pub fn render_sequence_sql(sequence: &SequenceInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: sequence").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}",
        sequence.schema_name, sequence.sequence_name
    )
    .ok();
    writeln!(
        sql,
        "-- Owned-by relationship not captured in Private Beta."
    )
    .ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE SEQUENCE {}.{}",
        quote_postgres_identifier(&sequence.schema_name),
        quote_postgres_identifier(&sequence.sequence_name)
    )
    .ok();
    if let Some(data_type) = &sequence.data_type {
        writeln!(sql, "    AS {data_type}").ok();
    }
    if let Some(value) = sequence.start_value {
        writeln!(sql, "    START WITH {value}").ok();
    }
    if let Some(value) = sequence.increment_by {
        writeln!(sql, "    INCREMENT BY {value}").ok();
    }
    if let Some(value) = sequence.min_value {
        writeln!(sql, "    MINVALUE {value}").ok();
    }
    if let Some(value) = sequence.max_value {
        writeln!(sql, "    MAXVALUE {value}").ok();
    }
    if let Some(value) = sequence.cache_size {
        writeln!(sql, "    CACHE {value}").ok();
    }
    if sequence.cycle {
        writeln!(sql, "    CYCLE").ok();
    } else {
        writeln!(sql, "    NO CYCLE").ok();
    }
    sql.push_str(";\n");
    sql
}

pub fn render_index_sql(index: &IndexInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: index").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}.{}",
        index.schema_name, index.table_name, index.index_name
    )
    .ok();
    writeln!(sql).ok();
    let definition = index.definition.trim().trim_end_matches(';');
    writeln!(sql, "{definition};").ok();
    sql
}

pub fn render_view_sql(view: &ViewInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: view").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}",
        view.schema_name, view.view_name
    )
    .ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE VIEW {}.{} AS",
        quote_postgres_identifier(&view.schema_name),
        quote_postgres_identifier(&view.view_name)
    )
    .ok();
    writeln!(sql, "{};", view.definition.trim().trim_end_matches(';')).ok();
    sql
}

pub fn render_materialized_view_sql(materialized_view: &MaterializedViewInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: materializedView").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}",
        materialized_view.schema_name, materialized_view.materialized_view_name
    )
    .ok();
    if let Some(is_populated) = materialized_view.is_populated {
        writeln!(sql, "-- Observed populated: {is_populated}").ok();
    }
    if let Some(tablespace) = &materialized_view.tablespace {
        writeln!(sql, "-- Tablespace observed: {tablespace}").ok();
    }
    writeln!(
        sql,
        "-- Desired-state review SQL uses WITH NO DATA; DbState does not refresh materialized views."
    )
    .ok();
    writeln!(sql).ok();
    writeln!(
        sql,
        "CREATE MATERIALIZED VIEW {}.{} AS",
        quote_postgres_identifier(&materialized_view.schema_name),
        quote_postgres_identifier(&materialized_view.materialized_view_name)
    )
    .ok();
    writeln!(
        sql,
        "{}",
        materialized_view.definition.trim().trim_end_matches(';')
    )
    .ok();
    writeln!(sql, "WITH NO DATA;").ok();
    sql
}

pub fn render_constraint_sql(constraint: &ConstraintInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: constraint").ok();
    writeln!(sql, "-- Constraint type: {}", constraint.constraint_type).ok();
    writeln!(
        sql,
        "-- Object name: {}.{}.{}",
        constraint.schema_name, constraint.table_name, constraint.constraint_name
    )
    .ok();
    if let (Some(referenced_schema), Some(referenced_table)) =
        (&constraint.referenced_schema, &constraint.referenced_table)
    {
        writeln!(
            sql,
            "-- References: {}.{}",
            referenced_schema, referenced_table
        )
        .ok();
    }
    writeln!(sql).ok();
    let definition = constraint.definition.trim().trim_end_matches(';');
    writeln!(
        sql,
        "ALTER TABLE {}.{}",
        quote_postgres_identifier(&constraint.schema_name),
        quote_postgres_identifier(&constraint.table_name)
    )
    .ok();
    writeln!(
        sql,
        "    ADD CONSTRAINT {} {definition};",
        quote_postgres_identifier(&constraint.constraint_name)
    )
    .ok();
    sql
}

pub fn render_function_sql(function: &FunctionInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: function").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}({})",
        function.schema_name, function.function_name, function.identity_arguments
    )
    .ok();
    if let Some(result_type) = &function.result_type {
        writeln!(sql, "-- Result type: {result_type}").ok();
    }
    if let Some(language) = &function.language {
        writeln!(sql, "-- Language: {language}").ok();
    }
    if let Some(volatility) = &function.volatility {
        writeln!(sql, "-- Volatility: {volatility}").ok();
    }
    writeln!(
        sql,
        "-- Security: {}",
        if function.security_definer {
            "SECURITY DEFINER"
        } else {
            "SECURITY INVOKER"
        }
    )
    .ok();
    writeln!(
        sql,
        "-- Null input: {}",
        if function.is_strict {
            "STRICT"
        } else {
            "CALLED ON NULL INPUT"
        }
    )
    .ok();
    writeln!(sql).ok();
    let definition = function.definition.trim().trim_end_matches(';');
    writeln!(sql, "{definition};").ok();
    sql
}

pub fn render_trigger_sql(trigger: &TriggerInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: trigger").ok();
    writeln!(
        sql,
        "-- Object name: {}.{}.{}",
        trigger.schema_name, trigger.relation_name, trigger.trigger_name
    )
    .ok();
    if let (Some(function_schema), Some(function_name)) = (
        &trigger.trigger_function_schema,
        &trigger.trigger_function_name,
    ) {
        writeln!(
            sql,
            "-- Trigger function: {function_schema}.{function_name}"
        )
        .ok();
    }
    if let Some(timing) = &trigger.timing {
        writeln!(sql, "-- Timing: {timing}").ok();
    }
    if !trigger.events.is_empty() {
        writeln!(sql, "-- Events: {}", trigger.events.join(", ")).ok();
    }
    if let Some(orientation) = &trigger.orientation {
        writeln!(sql, "-- Orientation: {orientation}").ok();
    }
    writeln!(sql).ok();
    let definition = trigger.definition.trim().trim_end_matches(';');
    writeln!(sql, "{definition};").ok();
    sql
}

pub fn render_grant_sql(grant: &GrantInfo) -> String {
    let mut sql = String::new();
    writeln!(sql, "-- DbState PostgreSQL desired-state object").ok();
    writeln!(sql, "-- Object type: grant").ok();
    writeln!(sql, "-- Grant target kind: {}", grant.target_kind).ok();
    writeln!(sql, "-- Object name: {}", grant_object_display_name(grant)).ok();
    writeln!(sql, "-- Grantee: {}", grant.grantee).ok();
    if let Some(grantor) = &grant.grantor {
        writeln!(sql, "-- Grantor observed: {grantor}").ok();
    }
    if grant.with_grant_option {
        writeln!(sql, "-- With grant option: true").ok();
    }
    writeln!(sql).ok();

    let grantable: std::collections::BTreeSet<&str> = grant
        .grantable_privileges
        .iter()
        .map(String::as_str)
        .collect();
    let plain_privileges: Vec<String> = grant
        .privileges
        .iter()
        .filter(|privilege| !grantable.contains(privilege.as_str()))
        .cloned()
        .collect();
    if !plain_privileges.is_empty() {
        writeln!(
            sql,
            "{};",
            render_grant_statement(grant, &plain_privileges, false)
        )
        .ok();
    }
    if !grant.grantable_privileges.is_empty() {
        writeln!(
            sql,
            "{};",
            render_grant_statement(grant, &grant.grantable_privileges, true)
        )
        .ok();
    }
    sql
}

pub(crate) fn ordered_grant_privileges(target_kind: &str, privileges: Vec<String>) -> Vec<String> {
    let mut privileges = privileges;
    privileges.sort_by(|left, right| {
        grant_privilege_rank(target_kind, left)
            .cmp(&grant_privilege_rank(target_kind, right))
            .then(left.cmp(right))
    });
    privileges.dedup();
    privileges
}

fn grant_privilege_rank(target_kind: &str, privilege: &str) -> u8 {
    let normalized = privilege.to_ascii_uppercase();
    match target_kind {
        "schema" => match normalized.as_str() {
            "USAGE" => 1,
            "CREATE" => 2,
            _ => 99,
        },
        "table" | "view" | "materializedView" => match normalized.as_str() {
            "SELECT" => 1,
            "INSERT" => 2,
            "UPDATE" => 3,
            "DELETE" => 4,
            "TRUNCATE" => 5,
            "REFERENCES" => 6,
            "TRIGGER" => 7,
            _ => 99,
        },
        "sequence" => match normalized.as_str() {
            "USAGE" => 1,
            "SELECT" => 2,
            "UPDATE" => 3,
            _ => 99,
        },
        "function" => match normalized.as_str() {
            "EXECUTE" => 1,
            _ => 99,
        },
        _ => 99,
    }
}

fn render_grant_statement(
    grant: &GrantInfo,
    privileges: &[String],
    with_grant_option: bool,
) -> String {
    let mut statement = format!(
        "GRANT {} ON {} TO {}",
        ordered_grant_privileges(&grant.target_kind, privileges.to_vec())
            .iter()
            .map(|privilege| privilege.to_ascii_uppercase())
            .collect::<Vec<_>>()
            .join(", "),
        grant_target_sql(grant),
        grant_grantee_sql(&grant.grantee)
    );
    if with_grant_option {
        statement.push_str(" WITH GRANT OPTION");
    }
    statement
}

fn grant_target_sql(grant: &GrantInfo) -> String {
    match grant.target_kind.as_str() {
        "schema" => format!("SCHEMA {}", quote_postgres_identifier(&grant.schema_name)),
        "table" => format!(
            "TABLE {}.{}",
            quote_postgres_identifier(&grant.schema_name),
            quote_postgres_identifier(grant.object_name.as_deref().unwrap_or(""))
        ),
        "view" => format!(
            "TABLE {}.{}",
            quote_postgres_identifier(&grant.schema_name),
            quote_postgres_identifier(grant.object_name.as_deref().unwrap_or(""))
        ),
        "materializedView" => format!(
            "TABLE {}.{}",
            quote_postgres_identifier(&grant.schema_name),
            quote_postgres_identifier(grant.object_name.as_deref().unwrap_or(""))
        ),
        "sequence" => format!(
            "SEQUENCE {}.{}",
            quote_postgres_identifier(&grant.schema_name),
            quote_postgres_identifier(grant.object_name.as_deref().unwrap_or(""))
        ),
        "function" => format!(
            "FUNCTION {}.{}({})",
            quote_postgres_identifier(&grant.schema_name),
            quote_postgres_identifier(grant.object_name.as_deref().unwrap_or("")),
            grant.identity_arguments.as_deref().unwrap_or("")
        ),
        _ => format!(
            "{} {}",
            grant.target_kind.to_ascii_uppercase(),
            grant_object_display_name(grant)
        ),
    }
}

fn grant_grantee_sql(grantee: &str) -> String {
    if grantee.eq_ignore_ascii_case("PUBLIC") {
        "PUBLIC".to_string()
    } else {
        quote_postgres_identifier(grantee)
    }
}

fn grant_object_display_name(grant: &GrantInfo) -> String {
    match (&grant.object_name, &grant.identity_arguments) {
        (Some(object_name), Some(identity_arguments)) => {
            format!(
                "{}.{}({})",
                grant.schema_name, object_name, identity_arguments
            )
        }
        (Some(object_name), None) => format!("{}.{}", grant.schema_name, object_name),
        (None, _) => grant.schema_name.clone(),
    }
}

pub fn normalize_desired_state_text(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = normalized
        .split('\n')
        .map(|line| line.trim_end().to_string())
        .collect();
    while matches!(lines.last(), Some(line) if line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}
