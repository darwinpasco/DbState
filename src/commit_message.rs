use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy)]
pub(crate) enum OutputFormat { Text, Json }

#[derive(Clone, Copy, PartialEq)]
enum Style { Conventional, Plain }

#[derive(Clone, Copy, PartialEq)]
enum Kind { Add, Modify, Delete, Rename }

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
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--style" => {
                style = match args.get(i + 1).map(String::as_str) {
                    Some("conventional") => Style::Conventional,
                    Some("plain") => Style::Plain,
                    _ => return Err("--style must be conventional or plain".into()),
                };
                i += 2;
            }
            "--format" => {
                format = match args.get(i + 1).map(String::as_str) {
                    Some("text") => OutputFormat::Text,
                    Some("json") => OutputFormat::Json,
                    _ => return Err("--format must be text or json".into()),
                };
                i += 2;
            }
            "--json" => { format = OutputFormat::Json; i += 1; }
            "--intent" => {
                let value = args.get(i + 1).ok_or("--intent requires a value")?.trim();
                if value.is_empty() { return Err("--intent cannot be empty".into()); }
                intent = Some(value.to_string());
                i += 2;
            }
            "--help" | "-h" => return Err(usage().into()),
            value => return Err(format!("Unknown commit-message option: {value}")),
        }
    }

    let report = generate(cwd, style, intent.as_deref());
    let success = report.errors.is_empty();
    let output = match format {
        OutputFormat::Text => report.text(),
        OutputFormat::Json => report.json(),
    };
    Ok(ResultView { format, output, exit_code: if success { 0 } else { 2 } })
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
    fn text(&self) -> String {
        if !self.errors.is_empty() {
            return self.errors.iter().map(|e| format!("Error: {e}\n")).collect();
        }
        format!("{}\n\n{}\n", self.title, self.body)
    }

    fn json(&self) -> String {
        let style = if self.style == Style::Conventional { "conventional" } else { "plain" };
        let message = format!("{}\n\n{}", self.title, self.body);
        format!(
            "{{\"command\":\"commit-message\",\"success\":{},\"scope\":\"staged\",\"style\":\"{}\",\"branch\":{},\"ticket\":{},\"title\":\"{}\",\"body\":\"{}\",\"message\":\"{}\",\"breakingChange\":{},\"stagedFiles\":{},\"analyzedFiles\":{},\"ignoredFiles\":{},\"warnings\":{},\"errors\":{}}}",
            self.errors.is_empty(), style, opt_json(self.branch.as_deref()), opt_json(self.ticket.as_deref()),
            esc(&self.title), esc(&self.body), esc(&message), self.breaking,
            arr(&self.staged), arr(&self.analyzed), arr(&self.ignored), arr(&self.warnings), arr(&self.errors)
        )
    }
}

fn generate(cwd: &Path, style: Style, intent: Option<&str>) -> Report {
    let mut report = Report {
        style, branch: None, ticket: None, title: String::new(), body: String::new(),
        staged: vec![], analyzed: vec![], ignored: vec![], breaking: false,
        warnings: vec![], errors: vec![],
    };
    let root = match git_root(cwd) { Ok(v) => v, Err(e) => { report.errors.push(e); return report; } };
    report.branch = git_line(&root, &["branch", "--show-current"]);
    report.ticket = report.branch.as_deref().and_then(ticket_from_branch);
    let rows = match staged_rows(&root) { Ok(v) => v, Err(e) => { report.errors.push(e); return report; } };
    if rows.is_empty() {
        report.errors.push("No staged files were found. Stage the intended DbState changes, then regenerate the commit message.".into());
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
        report.errors.push("Staged files were found, but none are DbState-managed files under database/objects, database/reference-data, or database/releases.".into());
        return report;
    }
    if !report.ignored.is_empty() {
        report.warnings.push(format!("Ignored {} staged non-DbState file(s).", report.ignored.len()));
    }
    changes.sort_by(|a, b| a.object_type.cmp(&b.object_type).then(a.object_name.cmp(&b.object_name)));
    report.breaking = changes.iter().any(|c| c.breaking_reason.is_some());
    report.title = title(&changes, style, intent, report.breaking);
    report.body = body(&changes, report.ticket.as_deref());
    report
}

fn git_root(cwd: &Path) -> Result<PathBuf, String> {
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).current_dir(cwd).output()
        .map_err(|e| format!("Could not run Git: {e}"))?;
    if !out.status.success() { return Err("Current path is not inside a Git repository.".into()); }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() { Err("Git did not return a repository root.".into()) } else { Ok(value.into()) }
}

fn git_line(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).current_dir(root).output().ok()?;
    if !out.status.success() { return None; }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn staged_rows(root: &Path) -> Result<Vec<(String, String, Option<String>)>, String> {
    let out = Command::new("git").args(["diff", "--cached", "--name-status", "--find-renames=50%", "--no-ext-diff", "--"])
        .current_dir(root).output().map_err(|e| format!("Could not inspect staged files: {e}"))?;
    if !out.status.success() { return Err("Git could not inspect staged files.".into()); }
    let mut rows = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 { continue; }
        if parts[0].starts_with('R') || parts[0].starts_with('C') {
            if parts.len() >= 3 { rows.push((parts[0].into(), parts[2].into(), Some(parts[1].into()))); }
        } else { rows.push((parts[0].into(), parts[1].into(), None)); }
    }
    Ok(rows)
}

fn classify(status: &str, path: &str, old_path: Option<&str>) -> Option<Change> {
    let (object_type, object_name, schema) = dbstate_path(path)?;
    let kind = match status.chars().next()? { 'A' => Kind::Add, 'M' | 'T' => Kind::Modify, 'D' => Kind::Delete, 'R' | 'C' => Kind::Rename, _ => return None };
    let old_name = old_path.and_then(dbstate_path).map(|(_, n, _)| n);
    let breaking_reason = match kind {
        Kind::Delete => Some(format!("removes {object_type} {object_name}")),
        Kind::Rename => Some(format!("renames {object_type} {} to {object_name}", old_name.as_deref().unwrap_or("previous object"))),
        _ => None,
    };
    Some(Change { kind, path: path.into(), object_type, object_name, old_name, schema, breaking_reason })
}

fn dbstate_path(path: &str) -> Option<(String, String, Option<String>)> {
    let p = path.replace('\\', "/");
    let parts: Vec<&str> = p.split('/').collect();
    if parts.len() >= 4 && parts[0] == "database" && parts[1] == "objects" {
        let object_type = match parts[2] {
            "schemas" => "schema", "extensions" => "extension", "enums" => "enum", "sequences" => "sequence",
            "tables" => "table", "indexes" => "index", "views" => "view", "materialized-views" => "materialized view",
            "functions" => "function", "triggers" => "trigger", "constraints" => "constraint", "grants" => "grant",
            "rls-policies" => "RLS policy", other => other.trim_end_matches('s'),
        }.to_string();
        let name = stem(parts.last().copied().unwrap_or("object"));
        let schema = name.split_once('.').map(|(s, _)| s.to_string());
        return Some((object_type, name, schema));
    }
    if p == "database/reference-data/dbstate.reference-data.yml" { return Some(("reference-data registry".into(), "dbstate.reference-data".into(), None)); }
    if p.starts_with("database/reference-data/tables/") {
        let name = stem(parts.last().copied().unwrap_or("reference-data"));
        let schema = name.split_once('.').map(|(s, _)| s.to_string());
        return Some(("reference data".into(), name, schema));
    }
    if p.starts_with("database/releases/") { return Some(("release artifact".into(), parts.last().unwrap_or(&"artifact").to_string(), None)); }
    None
}

fn stem(name: &str) -> String {
    for ext in [".sql", ".yml", ".yaml", ".json"] { if let Some(v) = name.strip_suffix(ext) { return v.into(); } }
    name.into()
}

fn detect_breaking(root: &Path, change: &mut Change) {
    if change.breaking_reason.is_some() || change.kind != Kind::Modify || (change.object_type != "table" && change.object_type != "function") { return; }
    let out = Command::new("git").args(["diff", "--cached", "--unified=0", "--no-ext-diff", "--", &change.path]).current_dir(root).output();
    let Ok(out) = out else { return; }; if !out.status.success() { return; }
    let diff = String::from_utf8_lossy(&out.stdout);
    let removed: Vec<&str> = diff.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).map(|l| l[1..].trim()).collect();
    let added: Vec<&str> = diff.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).map(|l| l[1..].trim()).collect();
    if change.object_type == "function" && removed.iter().any(|l| l.to_ascii_uppercase().contains("CREATE FUNCTION") || l.to_ascii_uppercase().contains("CREATE OR REPLACE FUNCTION")) {
        change.breaking_reason = Some(format!("changes the signature or definition header of function {}", change.object_name)); return;
    }
    if change.object_type == "table" {
        let old = column_names(&removed); let new = column_names(&added);
        let dropped: Vec<String> = old.difference(&new).cloned().collect();
        if !dropped.is_empty() { change.breaking_reason = Some(format!("may remove table column(s) {} from {}", dropped.join(", "), change.object_name)); }
    }
}

fn column_names(lines: &[&str]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in lines {
        let value = line.trim().trim_end_matches(','); let upper = value.to_ascii_uppercase();
        if value.is_empty() || upper.starts_with("CREATE TABLE") || upper.starts_with("CONSTRAINT ") || value.starts_with(')') { continue; }
        let token = value.split_whitespace().next().unwrap_or("").trim_matches('"');
        if !token.is_empty() && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { names.insert(token.into()); }
    }
    names
}

fn ticket_from_branch(branch: &str) -> Option<String> {
    for token in branch.split(|c: char| !c.is_ascii_alphanumeric() && c != '-') {
        if let Some((prefix, digits)) = token.split_once('-') {
            if prefix.len() >= 2 && prefix.chars().all(|c| c.is_ascii_alphabetic()) && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                return Some(format!("{}-{digits}", prefix.to_ascii_uppercase()));
            }
        }
    }
    None
}

fn title(changes: &[Change], style: Style, intent: Option<&str>, breaking: bool) -> String {
    let summary = intent.map(normalize_intent).unwrap_or_else(|| summary(changes));
    if style == Style::Plain { return cap(&summary); }
    let commit_type = if changes.iter().any(|c| c.kind == Kind::Add && c.object_type != "release artifact") { "feat" } else if changes.iter().all(|c| c.object_type == "release artifact") { "chore" } else { "refactor" };
    let schemas: BTreeSet<String> = changes.iter().filter_map(|c| c.schema.clone()).collect();
    let scope = if schemas.len() == 1 && changes.iter().all(|c| c.schema.is_some()) { schemas.iter().next().unwrap().clone() } else { "database".into() };
    format!("{commit_type}({scope}){}: {summary}", if breaking { "!" } else { "" })
}

fn summary(changes: &[Change]) -> String {
    if changes.len() == 1 {
        let c = &changes[0]; let action = match c.kind { Kind::Add => "add", Kind::Modify => "update", Kind::Delete => "remove", Kind::Rename => "rename" };
        return if c.kind == Kind::Rename { format!("rename {} {} to {}", c.object_type, c.old_name.as_deref().unwrap_or("previous object"), c.object_name) } else { format!("{action} {} {}", c.object_type, c.object_name) };
    }
    let mut counts = BTreeMap::<&str, usize>::new(); for c in changes { *counts.entry(&c.object_type).or_default() += 1; }
    let items = counts.iter().take(3).map(|(t, n)| format!("{n} {}", if *n == 1 { (*t).to_string() } else { format!("{t}s") })).collect::<Vec<_>>().join(", ");
    format!("update {items}")
}

fn normalize_intent(value: &str) -> String {
    let value = value.trim().trim_end_matches(|c: char| matches!(c, '.' | '!' | '?'));
    let mut chars = value.chars(); match chars.next() { Some(c) => format!("{}{}", c.to_lowercase(), chars.collect::<String>()), None => "update database state".into() }
}
fn cap(value: &str) -> String { let mut c = value.chars(); match c.next() { Some(v) => format!("{}{}", v.to_uppercase(), c.collect::<String>()), None => String::new() } }

fn body(changes: &[Change], ticket: Option<&str>) -> String {
    let mut lines = Vec::new();
    for c in changes {
        let action = match c.kind { Kind::Add => "add", Kind::Modify => "update", Kind::Delete => "remove", Kind::Rename => "rename" };
        lines.push(if c.kind == Kind::Rename { format!("- rename {} {} to {}", c.object_type, c.old_name.as_deref().unwrap_or("previous object"), c.object_name) } else { format!("- {action} {} {}", c.object_type, c.object_name) });
    }
    let breaking: Vec<&str> = changes.iter().filter_map(|c| c.breaking_reason.as_deref()).collect();
    if !breaking.is_empty() { lines.push(String::new()); lines.push("BREAKING CHANGE:".into()); lines.extend(breaking.into_iter().map(|v| format!("- {v}"))); }
    if let Some(ticket) = ticket { lines.push(String::new()); lines.push(format!("Refs: {ticket}")); }
    lines.join("\n")
}

fn esc(value: &str) -> String { value.chars().flat_map(|c| match c { '"' => "\\\"".chars().collect::<Vec<_>>(), '\\' => "\\\\".chars().collect(), '\n' => "\\n".chars().collect(), '\r' => "\\r".chars().collect(), '\t' => "\\t".chars().collect(), c => vec![c] }).collect() }
fn arr(values: &[String]) -> String { format!("[{}]", values.iter().map(|v| format!("\"{}\"", esc(v))).collect::<Vec<_>>().join(",")) }
fn opt_json(value: Option<&str>) -> String { value.map(|v| format!("\"{}\"", esc(v))).unwrap_or_else(|| "null".into()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn infers_ticket() { assert_eq!(ticket_from_branch("feature/DB-184-customer"), Some("DB-184".into())); }
    #[test]
    fn classifies_table() { let value = dbstate_path("database/objects/tables/public.customers.sql").unwrap(); assert_eq!(value.0, "table"); assert_eq!(value.1, "public.customers"); }
}
