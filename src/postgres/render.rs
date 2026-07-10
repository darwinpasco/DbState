use crate::postgres::inspect::{
    ColumnInfo, EnumInfo, ExtensionInfo, IndexInfo, SequenceInfo, ViewInfo,
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
