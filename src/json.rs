use std::fmt::Write as _;

pub(crate) fn write_json_string_field(json: &mut String, name: &str, value: &str, first: bool) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":\"{}\"", escape_json(name), escape_json(value)).ok();
}

pub(crate) fn write_json_optional_string_field(json: &mut String, name: &str, value: Option<&str>) {
    json.push(',');
    match value {
        Some(value) => write!(json, "\"{}\":\"{}\"", escape_json(name), escape_json(value)).ok(),
        None => write!(json, "\"{}\":null", escape_json(name)).ok(),
    };
}

pub(crate) fn write_json_bool_field(json: &mut String, name: &str, value: bool) {
    json.push(',');
    write!(json, "\"{}\":{}", escape_json(name), value).ok();
}

pub(crate) fn write_json_i32_field(json: &mut String, name: &str, value: i32, first: bool) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":{}", escape_json(name), value).ok();
}

pub(crate) fn write_json_i64_field(json: &mut String, name: &str, value: i64, first: bool) {
    if !first {
        json.push(',');
    }
    write!(json, "\"{}\":{}", escape_json(name), value).ok();
}

pub(crate) fn write_json_array_field(json: &mut String, name: &str, values: &[String]) {
    json.push(',');
    write!(json, "\"{}\":[", escape_json(name)).ok();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(json, "\"{}\"", escape_json(value)).ok();
    }
    json.push(']');
}

pub(crate) fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                write!(escaped, "\\u{:04x}", character as u32).ok();
            }
            character => escaped.push(character),
        }
    }
    escaped
}
