use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Conventional,
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Add,
    Modify,
    Delete,
    Rename,
}

#[derive(Clone, Debug)]
struct Change {
    kind: Kind,
    path: String,
    object_type: String,
    object_name: String,
    schema: Option<String>,
    details: Vec<String>,
    breaking_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StagedRow {
    status: String,
    path: String,
    old_path: Option<String>,
}

pub struct ResultView {
    pub format: OutputFormat,
    pub output: String,
    pub exit_code: u8,
}

pub struct Report {
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
    pub fn success(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn to_text(&self) -> String {
        if !self.errors.is_empty() {
            let mut text = String::new();
            for error in &self.errors {
                writeln!(text, "Error: {error}").ok();
            }
            for warning in &self.warnings {
                writeln!(text, "Warning: {warning}").ok();
            }
            return text;
        }

        let mut text = String::new();
        writeln!(text, "Suggested Commit Title").ok();
        writeln!(text, "{}", self.title).ok();
        writeln!(text).ok();
        writeln!(text, "Suggested Commit Body").ok();
        writeln!(text, "{}", self.body).ok();
        writeln!(text).ok();
        writeln!(text, "Analyzed DbState files").ok();
        for path in &self.analyzed {
            writeln!(text, "- {path}").ok();
        }
        if !self.ignored.is_empty() {
            writeln!(text).ok();
            writeln!(text, "Ignored non-DbState staged files").ok();
            for path in &self.ignored {
                writeln!(text, "- {path}").ok();
            }
        }
        if !self.warnings.is_empty() {
            writeln!(text).ok();
            writeln!(text, "Warnings").ok();
            for warning in &self.warnings {
                writeln!(text, "- {warning}").ok();
            }
        }
        if self.breaking {
            writeln!(text).ok();
            writeln!(text, "Breaking change warning").ok();
            writeln!(text, "Potentially breaking changes detected.").ok();
        }
        text
    }

    pub fn to_json(&self) -> String {
        let style = if self.style == Style::Conventional {
            "conventional"
        } else {
            "plain"
        };
        let message = if self.title.is_empty() && self.body.is_empty() {
            String::new()
        } else {
            format!("{}\n\n{}", self.title, self.body)
        };
        format!(
            "{{\"command\":\"commit-message\",\"success\":{},\"scope\":\"staged\",\"style\":\"{}\",\"branch\":{},\"ticket\":{},\"title\":\"{}\",\"body\":\"{}\",\"message\":\"{}\",\"breakingChange\":{},\"stagedFiles\":{},\"analyzedFiles\":{},\"ignoredFiles\":{},\"warnings\":{},\"errors\":{}}}",
            self.success(),
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

pub fn usage() -> &'static str {
    "Usage:\n  dbstate commit-message [--style conventional|plain] [--intent <text>] [--format text|json|--json]\n\nGenerates a reviewable title and body from staged DbState-managed files. It never stages, commits, amends, switches branches, pushes, pulls, fetches, or tags."
}

pub fn run(args: &[String], cwd: &Path) -> Result<ResultView, String> {
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
    let output = match format {
        OutputFormat::Text => report.to_text(),
        OutputFormat::Json => report.to_json(),
    };

    Ok(ResultView {
        format,
        output,
        exit_code: if report.success() { 0 } else { 2 },
    })
}

pub fn generate(cwd: &Path, style: Style, intent: Option<&str>) -> Report {
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
    report.branch = git_line(&root, &["branch", "--show-current"]).or_else(|| {
        git_line(&root, &["rev-parse", "--short", "HEAD"]).map(|head| format!("HEAD {head}"))
    });
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
            "No staged files were found. Stage reviewed DbState paths manually, then regenerate."
                .to_string(),
        );
        return report;
    }

    let mut changes = Vec::new();
    for row in rows {
        report.staged.push(safe_path_for_output(&row.path));
        match classify(&row.status, &row.path, row.old_path.as_deref()) {
            Some(mut change) => {
                analyze_staged_diff(&root, &mut change);
                report.analyzed.push(safe_path_for_output(&row.path));
                changes.push(change);
            }
            None => report.ignored.push(safe_path_for_output(&row.path)),
        }
    }

    if changes.is_empty() {
        report.errors.push(
            "Staged files were found, but none are DbState-managed files under database/objects, database/reference-data, or database/releases."
                .to_string(),
        );
        if !report.ignored.is_empty() {
            report.warnings.push(format!(
                "Ignored {} staged non-DbState file(s).",
                report.ignored.len()
            ));
        }
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
            .then(left.path.cmp(&right.path))
    });
    report.breaking = changes
        .iter()
        .any(|change| change.breaking_reason.is_some());
    report.title = build_title(&changes, style, intent, report.breaking);
    report.body = build_body(&changes, report.ticket.as_deref(), intent);
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
        Ok(PathBuf::from(value))
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
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn staged_rows(root: &Path) -> Result<Vec<StagedRow>, String> {
    let output = Command::new("git")
        .args([
            "diff",
            "--cached",
            "--name-status",
            "--find-renames=50%",
            "--no-ext-diff",
            "-z",
            "--",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| format!("Could not inspect staged files: {error}"))?;
    if !output.status.success() {
        return Err("Git could not inspect staged files.".to_string());
    }

    let fields: Vec<String> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(|field| String::from_utf8_lossy(field).to_string())
        .collect();
    let mut rows = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status = fields[index].clone();
        index += 1;
        if status.starts_with('R') || status.starts_with('C') {
            if index + 1 >= fields.len() {
                break;
            }
            let old_path = fields[index].clone();
            let path = fields[index + 1].clone();
            index += 2;
            rows.push(StagedRow {
                status,
                path,
                old_path: Some(old_path),
            });
        } else {
            if index >= fields.len() {
                break;
            }
            let path = fields[index].clone();
            index += 1;
            rows.push(StagedRow {
                status,
                path,
                old_path: None,
            });
        }
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
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
    let details = vec![change_sentence(
        kind,
        &object_type,
        &object_name,
        old_name.as_deref(),
    )];
    let breaking_reason = match kind {
        Kind::Delete => Some(format!("removes {object_type} {object_name}")),
        Kind::Rename => Some(format!(
            "renames {object_type} {} to {object_name}",
            old_name.as_deref().unwrap_or("previous object")
        )),
        Kind::Modify if object_type == "reference data" => Some(format!(
            "updates reference-data values for {object_name}, which may be application-visible"
        )),
        _ => None,
    };
    Some(Change {
        kind,
        path: path.to_string(),
        object_type,
        object_name,
        schema,
        details,
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
        let name = object_name_from_parts(&parts);
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
    if normalized.starts_with("database/releases/objects/") {
        return Some((
            "schema release artifact".to_string(),
            stem(parts.last().copied().unwrap_or("artifact")),
            None,
        ));
    }
    if normalized.starts_with("database/releases/reference-data/") {
        return Some((
            "reference-data review artifact".to_string(),
            stem(parts.last().copied().unwrap_or("artifact")),
            None,
        ));
    }
    if normalized.starts_with("database/releases/") {
        return Some((
            "release artifact".to_string(),
            stem(parts.last().copied().unwrap_or("artifact")),
            None,
        ));
    }
    None
}

fn object_name_from_parts(parts: &[&str]) -> String {
    if parts.get(2) == Some(&"grants") && parts.len() >= 6 {
        let target_kind = parts.get(3).copied().unwrap_or("object");
        return format!(
            "{target_kind}/{}",
            stem(parts.last().copied().unwrap_or("grant"))
        );
    }
    stem(parts.last().copied().unwrap_or("object"))
}

fn stem(name: &str) -> String {
    for extension in [
        ".reference-data.sql",
        ".summary.md",
        ".risk.json",
        ".manifest.json",
        ".sql",
        ".yml",
        ".yaml",
        ".json",
    ] {
        if let Some(value) = name.strip_suffix(extension) {
            return value.to_string();
        }
    }
    name.to_string()
}

fn analyze_staged_diff(root: &Path, change: &mut Change) {
    if !matches!(change.kind, Kind::Modify | Kind::Add | Kind::Delete) {
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

    if change.object_type == "table" && change.kind == Kind::Modify {
        let old_columns = column_names(&removed);
        let new_columns = column_names(&added);
        for column in new_columns.difference(&old_columns) {
            change.details.push(format!(
                "add column {column} to table {}",
                change.object_name
            ));
        }
        let dropped: Vec<String> = old_columns.difference(&new_columns).cloned().collect();
        if !dropped.is_empty() {
            change.breaking_reason = Some(format!(
                "may remove table column(s) {} from {}",
                dropped.join(", "),
                change.object_name
            ));
            for column in dropped {
                change.details.push(format!(
                    "remove column {column} from table {}",
                    change.object_name
                ));
            }
        }
        if nullability_tightened(&removed, &added) && change.breaking_reason.is_none() {
            change.breaking_reason = Some(format!(
                "may change nullable column(s) to NOT NULL on {}",
                change.object_name
            ));
        }
    }

    if change.object_type == "function" && function_header_changed(&removed, &added) {
        change.breaking_reason = Some(format!(
            "changes the signature or definition header of function {}",
            change.object_name
        ));
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
            || upper.starts_with("PRIMARY KEY")
            || upper.starts_with("FOREIGN KEY")
            || upper.starts_with("UNIQUE ")
            || upper.starts_with("CHECK ")
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

fn nullability_tightened(removed: &[&str], added: &[&str]) -> bool {
    let removed_nullable = removed
        .iter()
        .any(|line| !line.to_ascii_uppercase().contains(" NOT NULL"));
    let added_not_null = added
        .iter()
        .any(|line| line.to_ascii_uppercase().contains(" NOT NULL"));
    removed_nullable && added_not_null
}

fn function_header_changed(removed: &[&str], added: &[&str]) -> bool {
    let removed_header = removed.iter().any(|line| is_function_header(line));
    let added_header = added.iter().any(|line| is_function_header(line));
    removed_header && added_header
}

fn is_function_header(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    upper.contains("CREATE FUNCTION") || upper.contains("CREATE OR REPLACE FUNCTION")
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

fn build_title(changes: &[Change], style: Style, intent: Option<&str>, breaking: bool) -> String {
    let summary = intent
        .map(normalize_intent)
        .unwrap_or_else(|| summarize(changes));
    if style == Style::Plain {
        return capitalize(&truncate_summary(&summary));
    }

    let commit_type = commit_type(changes);
    let scope = commit_scope(changes);
    format!(
        "{commit_type}({scope}){}: {}",
        if breaking { "!" } else { "" },
        truncate_summary(&summary)
    )
}

fn commit_type(changes: &[Change]) -> &'static str {
    if changes.iter().all(|change| {
        matches!(
            change.object_type.as_str(),
            "schema release artifact" | "reference-data review artifact" | "release artifact"
        )
    }) {
        return "chore";
    }
    if changes.iter().any(|change| change.object_type == "index") {
        return "perf";
    }
    if changes.iter().any(|change| {
        change.kind == Kind::Add
            && matches!(
                change.object_type.as_str(),
                "table"
                    | "view"
                    | "materialized view"
                    | "function"
                    | "trigger"
                    | "extension"
                    | "enum"
                    | "sequence"
            )
    }) {
        return "feat";
    }
    if changes.iter().any(|change| {
        matches!(
            change.object_type.as_str(),
            "reference data" | "reference-data registry"
        )
    }) {
        return "fix";
    }
    if changes.iter().any(|change| change.kind == Kind::Modify) {
        return "fix";
    }
    "refactor"
}

fn commit_scope(changes: &[Change]) -> String {
    let schemas: BTreeSet<String> = changes
        .iter()
        .filter_map(|change| change.schema.clone())
        .collect();
    if schemas.len() == 1 && changes.iter().all(|change| change.schema.is_some()) {
        schemas.iter().next().expect("one schema").clone()
    } else if changes
        .iter()
        .all(|change| change.object_type.contains("reference-data"))
    {
        "reference-data".to_string()
    } else {
        "database".to_string()
    }
}

fn summarize(changes: &[Change]) -> String {
    let detail = changes
        .iter()
        .flat_map(|change| change.details.iter())
        .find(|detail| detail.starts_with("add column "))
        .cloned();
    if let Some(detail) = detail {
        return detail;
    }
    if changes.len() == 1 {
        return changes[0].details[0].clone();
    }

    let mut counts = BTreeMap::<&str, usize>::new();
    for change in changes {
        *counts.entry(&change.object_type).or_default() += 1;
    }
    let items = counts
        .iter()
        .take(3)
        .map(|(object_type, count)| {
            let label = plural_label(object_type, *count);
            format!("{count} {label}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("update {items}")
}

fn plural_label(object_type: &str, count: usize) -> String {
    if count == 1 {
        return object_type.to_string();
    }
    match object_type {
        "reference data" => "reference-data table files".to_string(),
        "reference-data registry" => "reference-data registries".to_string(),
        "RLS policy" => "RLS policies".to_string(),
        "schema release artifact" => "schema release artifacts".to_string(),
        "reference-data review artifact" => "reference-data review artifacts".to_string(),
        value if value.ends_with('s') => value.to_string(),
        value => format!("{value}s"),
    }
}

fn change_sentence(
    kind: Kind,
    object_type: &str,
    object_name: &str,
    old_name: Option<&str>,
) -> String {
    match kind {
        Kind::Add => format!("add {object_type} {object_name}"),
        Kind::Modify => format!("update {object_type} {object_name}"),
        Kind::Delete => format!("remove {object_type} {object_name}"),
        Kind::Rename => format!(
            "rename {object_type} {} to {object_name}",
            old_name.unwrap_or("previous object")
        ),
    }
}

fn normalize_intent(value: &str) -> String {
    let value = value.trim().trim_end_matches(['.', '!', '?']);
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => format!("{}{}", first.to_lowercase(), characters.collect::<String>()),
        None => "update database state".to_string(),
    }
}

fn truncate_summary(value: &str) -> String {
    const LIMIT: usize = 90;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    let mut truncated = value.chars().take(LIMIT - 1).collect::<String>();
    truncated = truncated.trim_end_matches([' ', '-', ',']).to_string();
    format!("{truncated}...")
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), characters.collect::<String>()),
        None => String::new(),
    }
}

fn build_body(changes: &[Change], ticket: Option<&str>, intent: Option<&str>) -> String {
    let mut lines = Vec::new();
    if let Some(intent) = intent {
        lines.push(capitalize(&normalize_intent(intent)));
        lines.push(String::new());
    }
    lines.push("Technical changes:".to_string());
    let mut emitted = BTreeSet::new();
    for change in changes {
        for detail in &change.details {
            if emitted.insert(detail.clone()) {
                lines.push(format!("- {detail}"));
            }
        }
    }

    let breaking: Vec<&str> = changes
        .iter()
        .filter_map(|change| change.breaking_reason.as_deref())
        .collect();
    if !breaking.is_empty() {
        lines.push(String::new());
        lines.push("BREAKING CHANGE:".to_string());
        lines.push("Potentially breaking changes detected.".to_string());
        lines.extend(breaking.into_iter().map(|reason| format!("- {reason}")));
    }
    if let Some(ticket) = ticket {
        lines.push(String::new());
        lines.push(format!("Refs: {ticket}"));
    }
    lines.join("\n")
}

fn safe_path_for_output(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower.contains("postgres://")
        || lower.contains("postgresql://")
        || lower.contains("password")
        || lower.contains("token")
        || lower.contains("secret")
    {
        "<redacted-path>".to_string()
    } else {
        path.replace('\\', "/")
    }
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
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            other if other < '\u{20}' => {
                write!(escaped, "\\u{:04x}", other as u32).ok();
            }
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
        assert_eq!(
            ticket_from_branch("dbstate/DB-184/customer-email-verification"),
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

    #[test]
    fn classifies_nested_release_paths() {
        let schema = dbstate_path("database/releases/objects/0001_release1.summary.md").unwrap();
        assert_eq!(schema.0, "schema release artifact");
        assert_eq!(schema.1, "0001_release1");
        let data =
            dbstate_path("database/releases/reference-data/0001_reference.reference-data.sql")
                .unwrap();
        assert_eq!(data.0, "reference-data review artifact");
        assert_eq!(data.1, "0001_reference");
    }
}
