use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Conventional,
    Plain,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Add,
    Modify,
    Delete,
    Rename,
}

struct Change {
    kind: Kind,
    path: String,
    object_type: String,
    object_name: String,
    old_name: Option<String>,
    schema: Option<String>,
    breaking_reason: Option<String>,
}

pub(crate) struct ResultView {
    pub(crate) format: OutputFormat,
    pub(crate) output: String,
    pub(crate) exit_code: u8,
}

pub(crate) fn usage() -> &'static str {
    "Usage:\n  dbstate commit-message [--style conventional|plain] [--intent <text>] [--format text|json|--json]\n\nGenerates a reviewable title and body from staged DbState-managed files. It never stages, commits, or pushes."
}

pub(crate) fn run(args: &[String], cwd: &Path) -> Result<ResultView, String> {
    let mut style = Style::Conventional;
    let mut format = OutputFormat::Text;
    let mut intent = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--style" => {
                style = match args.get(index + 1).map(String::as_str) {
                    Some("conventional") => Style::Conventional,
                    Some("plain") => Style::Plain,
                    _ => return Err("--style must be conventional or plain".to_string()),
                };
                index += 2;
            }
            "--format" => {
                format = match args.get(index + 1).map(String::as_str) {
                    Some("text") => OutputFormat::Text,
                    Some("json") => OutputFormat::Json,
                    _ => return Err("--format must be text or json".to_string()),
                };
                index += 2;
            }
            "--json" => {
                format = OutputFormat::Json;
                index += 1;
            }
            "--intent" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--intent requires a value".to_string())?
                    .trim();
                if value.is_empty() {
                    return Err("--intent cannot be empty".to_string());
                }
                intent = Some(value.to_string());
                index += 2;
            }
            "--help" | "-h" => return Err(usage().to_string()),
            value => return Err(format!("Unknown commit-message option: {value}")),
        }
    }

    let report = generate(cwd, style, intent.as_deref());
    let success = report.errors.is_empty();
    let output = match format {
        OutputFormat::Text => report.to_text(),
        OutputFormat::Json => report.to_json(),
    };

    Ok(ResultView {
        format,
        output,
        exit_code: if success { 0 } else { 2 },
    })
}

struct Report {
    style: Style,
    branch: Option<String>,
    ticket: Option<String>,
    title: String,
    body: String,
    staged: Vec<String>,
    analyzed: Vec<String>,
    ignored: Vec<String>,
    breaking: bool,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl Report {
    fn to_text(&self) -> String {
        if !self.errors.is_empty() {
            return self
                .errors
                .iter()
                .map(|error| format!("Error: {error}"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
        }
        format!("{}\n\n{}\n", self.title, self.body)
    }

    fn to_json(&self) -> String {
        let style = if self.style == Style::Conventional {
            "conventional"
        } else {
            "plain"
        };
        let message = format!("{}\n\n{}", self.title, self.body);
        format!(
            "{{\"command\":\"commit-message\",\"success\":{},\"scope\":\"staged\",\"style\":\"{}\",\"branch\":{},\"ticket\":{},\"title\":\"{}\",\"body\":\"{}\",\"message\":\"{}\",\"breakingChange\":{},\"stagedFiles\":{},\"analyzedFiles\":{},\"ignoredFiles\":{},\"warnings\":{},\"errors\":{}}}",
            self.errors.is_empty(),
            style,
            optional_json(self.branch.as_deref()),
            optional_json(self.ticket.as_deref()),
            escape_json(&self.title),
            escape_json(&self.body),
            escape_json(&message),
            self.breaking,
            json_array(&self.staged),
            json_array(&self.analyzed),
            json_array(&self.ignored),
            json_array(&self.warnings),
            json_array(&self.errors)
        )
    }
}

fn generate(cwd: &Path, style: Style, intent: Option<&str>) -> Report {
    let mut report = Report {
        style,
        branch: None,
        ticket: None,
        title: String::new(),
        body: String::new(),
        staged: Vec::new(),
        analyzed: Vec::new(),
        ignored: Vec::new(),
        breaking: false,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let root = match git_root(cwd) {
        Ok(root) => root,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    report.branch = git_line(&root, &["branch", "--show-current"]);
    report.ticket = report.branch.as_deref().and_then(ticket_from_branch);

    let rows = match staged_rows(&root) {
        Ok(rows) => rows,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    if rows.is_empty() {
        report.errors.push(
            "No staged files were found. Stage the intended DbState changes, then regenerate the commit message."
                .to_string(),
        );
        return report;
    }

    let mut changes = Vec::new();
    for (status, path, old_path) in rows {
        report.staged.push(path.clone());
        match classify(&status, &path, old_path.as_deref()) {
            Some(mut change) => {
                detect_breaking(&root, &mut change);
                report.analyzed.push(path);
                changes.push(change);
            }
            None => report.ignored.push(path),
        }
    }

    if changes.is_empty() {
        report.errors.push(
            "Staged files were found, but none are DbState-managed files under database/objects, database/reference-data, or database/releases."
                .to_string(),
        );
        return report;
    }
    if !report.ignored.is_empty() {
        report.warnings.push(format!(
            "Ignored {} staged non-DbState file(s).",
            report.ignored.len()
        ));
    }

    changes.sort_by(|left, right| {
        left.object_type
            .cmp(&right.object_type)
            .then(left.object_name.cmp(&right.object_name))
    });
    report.breaking = changes.iter().any(|change| change.breaking_reason.is_some());
    report.title = build_title(&changes, style, intent, report.breaking);
    report.body = build_body(&changes, report.ticket.as_deref());
    report
}

fn git_root(cwd: &Path) -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("Could not run Git: {error}"))?;
    if !output.status.success() {
        return Err("Current path is not inside a Git repository.".to_string());
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Err("Git did not return a repository root.".to_string())
    } else {
        Ok(value.into())
    }
}

fn git_line(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn staged_rows(root: &Path) -> Result<Vec<(String, String, Option<String>)>, String> {
    let output = Command::new("git")
        .args([
            "diff",
            "--cached",
            "--name-status",
            "--find-renames=50%",
            "--no-ext-diff",
            "--",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| format!("Could not inspect staged files: {error}"))?;
    if !output.status.success() {
        return Err("Git could not inspect staged files.".to_string());
    }

    let mut rows = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }
        if parts[0].starts_with('R') || parts[0].starts_with('C') {
            if parts.len() >= 3 {
                rows.push((
                    parts[0].to_string(),
                    parts[2].to_string(),
                    Some(parts[1].to_string()),
                ));
            }
        } else {
            rows.push((parts[0].to_string(), parts[1].to_string(), None));
        }
    }
    Ok(rows)
}

fn classify(status: &str, path: &str, old_path: Option<&str>) -> Option<Change> {
    let (object_type, object_name, schema) = dbstate_path(path)?;
    let kind = match status.chars().next()? {
        'A' => Kind::Add,
        'M' | 'T' => Kind::Modify,
        'D' => Kind::Delete,
        'R' | 'C' => Kind::Rename,
        _ => return None,
    };
    let old_name = old_path.and_then(dbstate_path).map(|(_, name, _)| name);
    let breaking_reason = match kind {
        Kind::Delete => Some(format!("removes {object_type} {object_name}")),
        Kind::Rename => Some(format!(
            "renames {object_type} {} to {object_name}",
            old_name.as_deref().unwrap_or("previous object")
        )),
        _ => None,
    };
    Some(Change {
        kind,
        path: path.to_string(),
        object_type,
        object_name,
        old_name,
        schema,
        breaking_reason,
    })
}

fn dbstate_path(path: &str) -> Option<(String, String, Option<String>)> {
    let normalized = path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').collect();
    if parts.len() >= 4 && parts[0] == "database" && parts[1] == "objects" {
        let object_type = match parts[2] {
            "schemas" => "schema",
            "extensions" => "extension",
            "enums" => "enum",
            "sequences" => "sequence",
            "tables" => "table",
            "indexes" => "index",
            "views" => "view",
            "materialized-views" => "materialized view",
            "functions" => "function",
            "triggers" => "trigger",
            "constraints" => "constraint",
            "grants" => "grant",
            "rls-policies" => "RLS policy",
            other => other.trim_end_matches('s'),
        }
        .to_string();
        let name = stem(parts.last().copied().unwrap_or("object"));
        let schema = name.split_once('.').map(|(schema, _)| schema.to_string());
        return Some((object_type, name, schema));
    }
    if normalized == "database/reference-data/dbstate.reference-data.yml" {
        return Some((
            "reference-data registry".to_string(),
            "dbstate.reference-data".to_string(),
            None,
        ));
    }
    if normalized.starts_with("database/reference-data/tables/") {
        let name = stem(parts.last().copied().unwrap_or("reference-data"));
        let schema = name.split_once('.').map(|(schema, _)| schema.to_string());
        return Some(("reference data".to_string(), name, schema));
    }
    if normalized.starts_with("database/releases/") {
        return Some((
            "release artifact".to_string(),
            parts.last().unwrap_or(&"artifact").to_string(),
            None,
        ));
    }
    None
}

fn stem(name: &str) -> String {
    for extension in [".sql", ".yml", ".yaml", ".json"] {
        if let Some(value) = name.strip_suffix(extension) {
            return value.to_string();
        }
    }
    name.to_string()
}

fn detect_breaking(root: &Path, change: &mut Change) {
    if change.breaking_reason.is_some()
        || change.kind != Kind::Modify
        || (change.object_type != "table" && change.object_type != "function")
    {
        return;
    }

    let output = Command::new("git")
        .args([
            "diff",
            "--cached",
            "--unified=0",
            "--no-ext-diff",
            "--",
            &change.path,
        ])
        .current_dir(root)
        .output();
    let Ok(output) = output else { return };
    if !output.status.success() {
        return;
    }

    let diff = String::from_utf8_lossy(&output.stdout);
    let removed: Vec<&str> = diff
        .lines()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .map(|line| line[1..].trim())
        .collect();
    let added: Vec<&str> = diff
        .lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .map(|line| line[1..].trim())
        .collect();

    if change.object_type == "function"
        && removed.iter().any(|line| {
            let upper = line.to_ascii_uppercase();
            upper.contains("CREATE FUNCTION") || upper.contains("CREATE OR REPLACE FUNCTION")
        })
    {
        change.breaking_reason = Some(format!(
            "changes the signature or definition header of function {}",
            change.object_name
        ));
        return;
    }

    if change.object_type == "table" {
        let old_columns = column_names(&removed);
        let new_columns = column_names(&added);
        let dropped: Vec<String> = old_columns.difference(&new_columns).cloned().collect();
        if !dropped.is_empty() {
            change.breaking_reason = Some(format!(
                "may remove table column(s) {} from {}",
                dropped.join(", "),
                change.object_name
            ));
        }
    }
}

fn column_names(lines: &[&str]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in lines {
        let value = line.trim().trim_end_matches(',');
        let upper = value.to_ascii_uppercase();
        if value.is_empty()
            || upper.starts_with("CREATE TABLE")
            || upper.starts_with("CONSTRAINT ")
            || value.starts_with(')')
        {
            continue;
        }
        let token = value
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches('"');
        if !token.is_empty()
            && token
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            names.insert(token.to_string());
        }
    }
    names
}

fn ticket_from_branch(branch: &str) -> Option<String> {
    let bytes = branch.as_bytes();
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_alphabetic() {
            continue;
        }
        let mut dash = start;
        while dash < bytes.len() && bytes[dash].is_ascii_alphabetic() {
            dash += 1;
        }
        if dash - start < 2 || dash >= bytes.len() || bytes[dash] != b'-' {
            continue;
        }
        let mut end = dash + 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end > dash + 1 {
            return Some(branch[start..end].to_ascii_uppercase());
        }
    }
    None
}

fn build_title(
    changes: &[Change],
    style: Style,
    intent: Option<&str>,
    breaking: bool,
) -> String {
    let summary = intent
        .map(normalize_intent)
        .unwrap_or_else(|| summarize(changes));
    if style == Style::Plain {
        return capitalize(&summary);
    }

    let commit_type = if changes
        .iter()
        .any(|change| change.kind == Kind::Add && change.object_type != "release artifact")
    {
        "feat"
    } else if changes
        .iter()
        .all(|change| change.object_type == "release artifact")
    {
        "chore"
    } else {
        "refactor"
    };
    let schemas: BTreeSet<String> = changes
        .iter()
        .filter_map(|change| change.schema.clone())
        .collect();
    let scope = if schemas.len() == 1 && changes.iter().all(|change| change.schema.is_some()) {
        schemas.iter().next().expect("one schema").clone()
    } else {
        "database".to_string()
    };
    format!(
        "{commit_type}({scope}){}: {summary}",
        if breaking { "!" } else { "" }
    )
}

fn summarize(changes: &[Change]) -> String {
    if changes.len() == 1 {
        let change = &changes[0];
        let action = match change.kind {
            Kind::Add => "add",
            Kind::Modify => "update",
            Kind::Delete => "remove",
            Kind::Rename => "rename",
        };
        return if change.kind == Kind::Rename {
            format!(
                "rename {} {} to {}",
                change.object_type,
                change.old_name.as_deref().unwrap_or("previous object"),
                change.object_name
            )
        } else {
            format!("{action} {} {}", change.object_type, change.object_name)
        };
    }

    let mut counts = BTreeMap::<&str, usize>::new();
    for change in changes {
        *counts.entry(&change.object_type).or_default() += 1;
    }
    let items = counts
        .iter()
        .take(3)
        .map(|(object_type, count)| {
            let label = if *count == 1 {
                (*object_type).to_string()
            } else if **object_type == "reference data" {
                "reference-data tables".to_string()
            } else {
                format!("{object_type}s")
            };
            format!("{count} {label}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("update {items}")
}

fn normalize_intent(value: &str) -> String {
    let value = value
        .trim()
        .trim_end_matches(|character: char| matches!(character, '.' | '!' | '?'));
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => format!(
            "{}{}",
            first.to_lowercase(),
            characters.collect::<String>()
        ),
        None => "update database state".to_string(),
    }
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => format!(
            "{}{}",
            first.to_uppercase(),
            characters.collect::<String>()
        ),
        None => String::new(),
    }
}

fn build_body(changes: &[Change], ticket: Option<&str>) -> String {
    let mut lines = Vec::new();
    for change in changes {
        let action = match change.kind {
            Kind::Add => "add",
            Kind::Modify => "update",
            Kind::Delete => "remove",
            Kind::Rename => "rename",
        };
        lines.push(if change.kind == Kind::Rename {
            format!(
                "- rename {} {} to {}",
                change.object_type,
                change.old_name.as_deref().unwrap_or("previous object"),
                change.object_name
            )
        } else {
            format!("- {action} {} {}", change.object_type, change.object_name)
        });
    }

    let breaking: Vec<&str> = changes
        .iter()
        .filter_map(|change| change.breaking_reason.as_deref())
        .collect();
    if !breaking.is_empty() {
        lines.push(String::new());
        lines.push("BREAKING CHANGE:".to_string());
        lines.extend(breaking.into_iter().map(|reason| format!("- {reason}")));
    }
    if let Some(ticket) = ticket {
        lines.push(String::new());
        lines.push(format!("Refs: {ticket}"));
    }
    lines.join("\n")
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn json_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", escape_json(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn optional_json(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", escape_json(value)))
        .unwrap_or_else(|| "null".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_ticket_from_branch() {
        assert_eq!(
            ticket_from_branch("feature/DB-184-customer-verification"),
            Some("DB-184".to_string())
        );
    }

    #[test]
    fn classifies_table_path() {
        let value = dbstate_path("database/objects/tables/public.customers.sql").unwrap();
        assert_eq!(value.0, "table");
        assert_eq!(value.1, "public.customers");
        assert_eq!(value.2, Some("public".to_string()));
    }
}
