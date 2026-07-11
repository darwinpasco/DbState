use std::env;
use std::path::Path;

use crate::postgres::{inspect_postgres_scoped_command, InspectionReport};
use crate::project::{init_project, status_report, ProjectReport};
use crate::reference_data::{data_compare_postgres_command, ReferenceDataCompareReport};
use crate::release::{release_postgres_command, ReleaseReport};
use crate::repository::{
    compare_postgres_command, export_postgres_command, plan_postgres_command,
    sync_postgres_command, CompareReport, ExportReport, PlanReport, SyncReport,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    RepoStatus,
    Init,
    InspectPostgres,
    ExportPostgres,
    SyncPostgres,
    ComparePostgres,
    PlanPostgres,
    ReleasePostgres,
    DataComparePostgres,
}

impl CommandKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RepoStatus => "repo status",
            Self::Init => "init",
            Self::InspectPostgres => "inspect postgres",
            Self::ExportPostgres => "export postgres",
            Self::SyncPostgres => "sync postgres",
            Self::ComparePostgres => "compare postgres",
            Self::PlanPostgres => "plan postgres",
            Self::ReleasePostgres => "release postgres",
            Self::DataComparePostgres => "data-compare postgres",
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandOutput {
    Project(ProjectReport),
    Inspection(InspectionReport),
    Export(ExportReport),
    Sync(SyncReport),
    Compare(CompareReport),
    Plan(PlanReport),
    Release(ReleaseReport),
    DataCompare(ReferenceDataCompareReport),
}

impl CommandOutput {
    pub fn to_text(&self) -> String {
        match self {
            Self::Project(report) => report.to_text(),
            Self::Inspection(report) => report.to_text(),
            Self::Export(report) => report.to_text(),
            Self::Sync(report) => report.to_text(),
            Self::Compare(report) => report.to_text(),
            Self::Plan(report) => report.to_text(),
            Self::Release(report) => report.to_text(),
            Self::DataCompare(report) => report.to_text(),
        }
    }

    pub fn to_json(&self) -> String {
        match self {
            Self::Project(report) => report.to_json(),
            Self::Inspection(report) => report.to_json(),
            Self::Export(report) => report.to_json(),
            Self::Sync(report) => report.to_json(),
            Self::Compare(report) => report.to_json(),
            Self::Plan(report) => report.to_json(),
            Self::Release(report) => report.to_json(),
            Self::DataCompare(report) => report.to_json(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CliResult {
    pub format: OutputFormat,
    pub output: CommandOutput,
    pub exit_code: u8,
}

pub fn run_cli(
    args: &[String],
    current_dir: Result<&Path, &std::io::Error>,
) -> Result<CliResult, String> {
    let cwd = current_dir.map_err(|error| format!("Could not read current directory: {error}"))?;
    let parsed = ParsedArgs::parse(args)?;

    match parsed.command {
        CommandKind::RepoStatus => {
            let report = status_report(cwd, CommandKind::RepoStatus);
            let exit_code = if report.is_git_repository { 0 } else { 2 };
            Ok(CliResult {
                format: parsed.format,
                output: CommandOutput::Project(report),
                exit_code,
            })
        }
        CommandKind::Init => {
            let report = init_project(cwd, parsed.dry_run)?;
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format: parsed.format,
                output: CommandOutput::Project(report),
                exit_code,
            })
        }
        CommandKind::InspectPostgres => {
            let report = inspect_postgres_scoped_command(
                parsed.url,
                env::var("DBSTATE_POSTGRES_URL").ok(),
                parsed.schema,
                parsed.table,
            );
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format: parsed.format,
                output: CommandOutput::Inspection(report),
                exit_code,
            })
        }
        CommandKind::ExportPostgres => {
            let format = parsed.format;
            let report = export_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Export(report),
                exit_code,
            })
        }
        CommandKind::SyncPostgres => {
            let format = parsed.format;
            let report = sync_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Sync(report),
                exit_code,
            })
        }
        CommandKind::ComparePostgres => {
            let format = parsed.format;
            let report = compare_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Compare(report),
                exit_code,
            })
        }
        CommandKind::PlanPostgres => {
            let format = parsed.format;
            let report = plan_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Plan(report),
                exit_code,
            })
        }
        CommandKind::ReleasePostgres => {
            let format = parsed.format;
            let report = release_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::Release(report),
                exit_code,
            })
        }
        CommandKind::DataComparePostgres => {
            let format = parsed.format;
            let report = data_compare_postgres_command(cwd, parsed);
            let exit_code = if report.success { 0 } else { 2 };
            Ok(CliResult {
                format,
                output: CommandOutput::DataCompare(report),
                exit_code,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedArgs {
    pub(crate) command: CommandKind,
    pub(crate) format: OutputFormat,
    pub(crate) dry_run: bool,
    pub(crate) url: Option<String>,
    pub(crate) schema: Option<String>,
    pub(crate) table: Option<String>,
    pub(crate) all: bool,
    pub(crate) includes: Vec<String>,
    pub(crate) excludes: Vec<String>,
    pub(crate) release_name: Option<String>,
}

impl ParsedArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        if args.is_empty() {
            return Err(usage());
        }

        let mut format = OutputFormat::Text;
        let mut dry_run = false;
        let mut url = None;
        let mut schema = None;
        let mut table = None;
        let mut all = false;
        let mut includes = Vec::new();
        let mut excludes = Vec::new();
        let mut release_name = None;
        let mut positional = Vec::new();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--format" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--format requires a value".to_string())?;
                    format = match value.as_str() {
                        "json" => OutputFormat::Json,
                        "text" => OutputFormat::Text,
                        _ => return Err("--format must be either json or text".to_string()),
                    };
                    index += 2;
                }
                "--json" => {
                    format = OutputFormat::Json;
                    index += 1;
                }
                "--dry-run" => {
                    dry_run = true;
                    index += 1;
                }
                "--url" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--url requires a value".to_string())?;
                    url = Some(value.to_string());
                    index += 2;
                }
                "--schema" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--schema requires a value".to_string())?;
                    schema = Some(value.to_string());
                    index += 2;
                }
                "--table" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--table requires a value".to_string())?;
                    table = Some(value.to_string());
                    index += 2;
                }
                "--all" => {
                    all = true;
                    index += 1;
                }
                "--include" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--include requires a value".to_string())?;
                    includes.push(value.to_string());
                    index += 2;
                }
                "--exclude" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--exclude requires a value".to_string())?;
                    excludes.push(value.to_string());
                    index += 2;
                }
                "--name" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| "--name requires a value".to_string())?;
                    release_name = Some(value.to_string());
                    index += 2;
                }
                "--help" | "-h" => return Err(usage()),
                value if value.starts_with('-') => {
                    return Err(format!("Unknown option: {value}"));
                }
                value => {
                    positional.push(value.to_string());
                    index += 1;
                }
            }
        }

        let command = match positional.as_slice() {
            [command] if command == "init" => CommandKind::Init,
            [repo, command] if repo == "repo" && command == "status" => CommandKind::RepoStatus,
            [inspect, database] if inspect == "inspect" && database == "postgres" => {
                CommandKind::InspectPostgres
            }
            [export, database] if export == "export" && database == "postgres" => {
                CommandKind::ExportPostgres
            }
            [sync, database] if sync == "sync" && database == "postgres" => {
                CommandKind::SyncPostgres
            }
            [compare, database] if compare == "compare" && database == "postgres" => {
                CommandKind::ComparePostgres
            }
            [plan, database] if plan == "plan" && database == "postgres" => {
                CommandKind::PlanPostgres
            }
            [release, database] if release == "release" && database == "postgres" => {
                CommandKind::ReleasePostgres
            }
            [data_compare, database]
                if data_compare == "data-compare" && database == "postgres" =>
            {
                CommandKind::DataComparePostgres
            }
            _ => return Err(usage()),
        };

        if dry_run
            && command != CommandKind::Init
            && command != CommandKind::ExportPostgres
            && command != CommandKind::SyncPostgres
            && command != CommandKind::ReleasePostgres
        {
            return Err(
                "--dry-run is only supported for dbstate init, dbstate export postgres, dbstate sync postgres, and dbstate release postgres"
                    .to_string(),
            );
        }

        if url.is_some()
            && command != CommandKind::InspectPostgres
            && command != CommandKind::ExportPostgres
            && command != CommandKind::SyncPostgres
            && command != CommandKind::ComparePostgres
            && command != CommandKind::PlanPostgres
            && command != CommandKind::ReleasePostgres
            && command != CommandKind::DataComparePostgres
        {
            return Err(
                "--url is only supported for dbstate inspect postgres, dbstate export postgres, dbstate sync postgres, dbstate compare postgres, dbstate plan postgres, dbstate release postgres, and dbstate data-compare postgres"
                    .to_string(),
            );
        }

        if (schema.is_some() || table.is_some() || all)
            && command != CommandKind::InspectPostgres
            && command != CommandKind::ExportPostgres
            && command != CommandKind::SyncPostgres
            && command != CommandKind::ComparePostgres
            && command != CommandKind::PlanPostgres
            && command != CommandKind::ReleasePostgres
            && command != CommandKind::DataComparePostgres
        {
            return Err(
                "--schema, --table, and --all are only supported for dbstate inspect postgres, dbstate export postgres, dbstate sync postgres, dbstate compare postgres, dbstate plan postgres, dbstate release postgres, and dbstate data-compare postgres"
                    .to_string(),
            );
        }

        if schema.is_some() && command == CommandKind::DataComparePostgres {
            return Err(
                "--schema is not supported for dbstate data-compare postgres. Use --table <schema.table> or --all."
                    .to_string(),
            );
        }

        if (!includes.is_empty() || !excludes.is_empty())
            && command != CommandKind::PlanPostgres
            && command != CommandKind::ReleasePostgres
        {
            return Err(
                "--include and --exclude are only supported for dbstate plan postgres and dbstate release postgres".to_string(),
            );
        }

        if release_name.is_some() && command != CommandKind::ReleasePostgres {
            return Err("--name is only supported for dbstate release postgres".to_string());
        }

        Ok(Self {
            command,
            format,
            dry_run,
            url,
            schema,
            table,
            all,
            includes,
            excludes,
            release_name,
        })
    }
}

pub fn usage() -> String {
    "Usage:\n  dbstate repo status [--format json|--json]\n  dbstate init [--dry-run] [--format json|--json]\n  dbstate inspect postgres [--url <postgres-url>] [--all | --schema <schema> | --table <schema.table>] [--format json|--json]\n  dbstate export postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--dry-run] [--format json|--json]\n  dbstate sync postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--dry-run] [--format json|--json]\n  dbstate compare postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--format json|--json]\n  dbstate plan postgres (--all | --schema <schema> | --table <schema.table>) [--url <postgres-url>] [--include <object-ref>] [--exclude <object-ref>] [--format json|--json]\n  dbstate release postgres (--all | --schema <schema> | --table <schema.table>) --name <release-name> [--url <postgres-url>] [--include <object-ref>] [--exclude <object-ref>] [--dry-run] [--format json|--json]\n  dbstate data-compare postgres (--all | --table <schema.table>) [--url <postgres-url>] [--format json|--json]\n  dbstate serve [--host <host>] [--port <port>] [--format json|--json]".to_string()
}
