use super::*;
use crate::postgres::{inspect::empty_inspection_report, resolve_postgres_url};
use crate::project::{PathKind, DEFAULT_REGISTRY, EXPECTED_PATHS};
use crate::reference_data::data_compare_postgres_command;
use crate::reference_data::{
    append_reference_table_result, empty_reference_data_compare_report,
    parse_reference_data_registry, parse_reference_data_table_state, reference_row_key,
};
use crate::release::empty_release_report;
use crate::release::release_postgres_command;
use crate::repository::discovery::discover_repository_objects;
use crate::repository::{
    compare_postgres_command, ensure_database_object_path, enum_file_path, export_postgres_command,
    extension_file_path, index_file_path, plan_postgres_command, schema_file_path,
    sequence_file_path, table_file_path, view_file_path,
};
use crate::service::{
    load_connection_profiles_from_path, parse_connection_profile,
    resolve_service_postgres_connection, save_connection_profiles, validate_connection_profile,
    validate_unique_profile_names,
};
use crate::workspace::normalize_local_path_input;
use serde_yaml::Value;
use std::env;
use std::fs;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeSet, path::PathBuf};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn temp_path(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("dbstate-{name}-{}-{now}", std::process::id()))
}

fn create_temp_dir(name: &str) -> PathBuf {
    let path = temp_path(name);
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn init_git_repo(path: &Path) {
    let output = Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .expect("run git init");
    assert!(output.status.success(), "git init failed");
}

fn commit_all(path: &Path, message: &str) {
    let add = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .expect("run git add");
    assert!(add.status.success(), "git add failed");

    let commit = Command::new("git")
        .arg("-c")
        .arg("user.email=dbstate@example.invalid")
        .arg("-c")
        .arg("user.name=DbState Test")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(path)
        .output()
        .expect("run git commit");
    assert!(commit.status.success(), "git commit failed");
}

fn create_complete_structure(root: &Path) {
    for expected in EXPECTED_PATHS {
        let target = root.join(expected.relative);
        match expected.kind {
            PathKind::Directory => fs::create_dir_all(target).expect("create directory"),
            PathKind::File => {
                fs::create_dir_all(target.parent().expect("file has parent"))
                    .expect("create parent");
                fs::write(target, DEFAULT_REGISTRY).expect("create registry");
            }
        }
    }
}

fn placeholder_url(user: &str, credential: &str) -> String {
    format!("{}://{user}:{credential}@example.invalid/db", "postgres")
}

fn sample_inventory() -> PostgresInventory {
    PostgresInventory {
            schemas: vec![SchemaInfo {
                name: "dbstate_slice2".to_string(),
            }],
            tables: vec![TableInfo {
                schema_name: "dbstate_slice2".to_string(),
                table_name: "sample_accounts".to_string(),
                table_type: "BASE TABLE".to_string(),
            }],
            columns: vec![
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "account_id".to_string(),
                    ordinal_position: 1,
                    data_type: "integer".to_string(),
                    is_nullable: false,
                    has_default: false,
                    default_expression: None,
                },
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "account_code".to_string(),
                    ordinal_position: 2,
                    data_type: "text".to_string(),
                    is_nullable: false,
                    has_default: false,
                    default_expression: None,
                },
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "display_name".to_string(),
                    ordinal_position: 3,
                    data_type: "text".to_string(),
                    is_nullable: true,
                    has_default: false,
                    default_expression: None,
                },
                ColumnInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    column_name: "created_at".to_string(),
                    ordinal_position: 4,
                    data_type: "timestamp without time zone".to_string(),
                    is_nullable: false,
                    has_default: true,
                    default_expression: Some("now()".to_string()),
                },
            ],
            extensions: vec![ExtensionInfo {
                extension_name: "pgcrypto".to_string(),
                schema_name: Some("public".to_string()),
                version: Some("1.3".to_string()),
            }],
            enums: vec![EnumInfo {
                schema_name: "dbstate_slice2".to_string(),
                enum_name: "account_status".to_string(),
                labels: vec!["active".to_string(), "closed".to_string()],
            }],
            sequences: vec![SequenceInfo {
                schema_name: "dbstate_slice2".to_string(),
                sequence_name: "account_number_seq".to_string(),
                data_type: Some("bigint".to_string()),
                start_value: Some(1),
                min_value: Some(1),
                max_value: Some(9_223_372_036_854_775_807),
                increment_by: Some(1),
                cycle: false,
                cache_size: Some(1),
            }],
            indexes: vec![IndexInfo {
                schema_name: "dbstate_slice2".to_string(),
                table_name: "sample_accounts".to_string(),
                index_name: "sample_accounts_account_code_idx".to_string(),
                is_unique: true,
                definition:
                    "CREATE UNIQUE INDEX sample_accounts_account_code_idx ON dbstate_slice2.sample_accounts USING btree (account_code)"
                        .to_string(),
            }],
            views: vec![ViewInfo {
                schema_name: "dbstate_slice2".to_string(),
                view_name: "active_accounts".to_string(),
                definition:
                    " SELECT sample_accounts.account_id,\n    sample_accounts.account_code\n   FROM dbstate_slice2.sample_accounts"
                        .to_string(),
            }],
        }
}

fn valid_reference_registry_yaml() -> &'static str {
    "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
    key:
      - code
    ignoreColumns:
      - updated_at
    maskedColumns:
      - secret_note
    allowDeletes: false
"
}

fn valid_reference_table_yaml() -> &'static str {
    "table: dbstate_ref.payment_methods
key:
  - code
rows:
  - code: CASH
    name: Cash
    is_active: true
    sort_order: 10
    updated_at: repo timestamp ignored
    secret_note: repo-secret-cash
  - code: QRPH
    name: QRPh Desired
    is_active: true
    sort_order: 20
    updated_at: repo timestamp ignored
    secret_note: repo-secret-qrph
  - code: WIRE
    name: Wire
    is_active: false
    sort_order: 40
    updated_at: repo timestamp ignored
    secret_note: repo-secret-wire
"
}

fn reference_config_and_state() -> (ReferenceDataTableConfig, ReferenceDataTableState) {
    let registry =
        parse_reference_data_registry(valid_reference_registry_yaml()).expect("registry");
    let config = registry.tables[0].clone();
    let state =
        parse_reference_data_table_state(valid_reference_table_yaml(), &config).expect("state");
    (config, state)
}

fn reference_database_columns() -> BTreeSet<String> {
    [
        "code",
        "name",
        "is_active",
        "sort_order",
        "updated_at",
        "secret_note",
    ]
    .into_iter()
    .map(|value| value.to_string())
    .collect()
}

fn reference_row(values: &[(&str, Option<&str>)]) -> ReferenceDataRow {
    ReferenceDataRow {
        values: values
            .iter()
            .map(|(key, value)| (key.to_string(), value.map(|value| value.to_string())))
            .collect(),
    }
}

#[test]
fn non_git_folder_returns_clear_status() {
    let dir = create_temp_dir("non-git");
    let report = status_report(&dir, CommandKind::RepoStatus);

    assert!(!report.success);
    assert!(!report.is_git_repository);
    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::NotGitRepository
    );
    assert!(report
        .errors
        .contains(&"Current path is not inside a Git repository.".to_string()));
}

#[test]
fn git_repo_without_structure_reports_missing_paths() {
    let dir = create_temp_dir("empty-git");
    init_git_repo(&dir);

    let report = status_report(&dir, CommandKind::RepoStatus);

    assert!(report.success);
    assert!(report.is_git_repository);
    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::GitRepositoryWithoutDbStateStructure
    );
    assert_eq!(report.missing_paths.len(), EXPECTED_PATHS.len());
}

#[test]
fn partial_structure_reports_only_missing_paths() {
    let dir = create_temp_dir("partial-git");
    init_git_repo(&dir);
    fs::create_dir_all(dir.join("database/objects/schemas")).expect("create partial");

    let report = status_report(&dir, CommandKind::RepoStatus);

    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::PartialDbStateStructure
    );
    assert!(report
        .existing_paths
        .contains(&"database/objects/schemas".to_string()));
    assert!(report
        .missing_paths
        .contains(&"database/releases".to_string()));
}

#[test]
fn complete_structure_reports_complete_status() {
    let dir = create_temp_dir("complete-git");
    init_git_repo(&dir);
    create_complete_structure(&dir);

    let report = status_report(&dir, CommandKind::RepoStatus);

    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::CompleteDbStateStructure
    );
    assert!(report.missing_paths.is_empty());
}

#[test]
fn dry_run_init_reports_planned_creates_but_creates_nothing() {
    let dir = create_temp_dir("dry-run");
    init_git_repo(&dir);

    let report = init_project(&dir, true).expect("dry-run init");

    assert!(report.success);
    assert!(!report.planned_creates.is_empty());
    assert!(report.created_paths.is_empty());
    assert!(!dir.join("database").exists());
}

#[test]
fn init_creates_only_missing_folders_and_files() {
    let dir = create_temp_dir("init");
    init_git_repo(&dir);

    let report = init_project(&dir, false).expect("init project");

    assert!(report.success);
    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::CompleteDbStateStructure
    );
    assert!(dir.join("database/objects/tables").is_dir());
    assert!(dir
        .join("database/reference-data/dbstate.reference-data.yml")
        .is_file());
    assert_eq!(
        fs::read_to_string(dir.join("database/reference-data/dbstate.reference-data.yml"))
            .expect("read registry"),
        DEFAULT_REGISTRY
    );
}

#[test]
fn init_does_not_overwrite_existing_registry() {
    let dir = create_temp_dir("no-overwrite");
    init_git_repo(&dir);
    let registry = dir.join("database/reference-data/dbstate.reference-data.yml");
    fs::create_dir_all(registry.parent().expect("registry parent")).expect("create parent");
    fs::write(&registry, "version: 1\ntables:\n  - name: public.keep_me\n")
        .expect("write existing registry");
    commit_all(&dir, "existing registry");

    let report = init_project(&dir, false).expect("init project");

    assert!(report.success);
    assert_eq!(
        fs::read_to_string(registry).expect("read registry"),
        "version: 1\ntables:\n  - name: public.keep_me\n"
    );
}

#[test]
fn init_does_not_create_secret_or_connection_files() {
    let dir = create_temp_dir("no-secrets");
    init_git_repo(&dir);

    init_project(&dir, false).expect("init project");

    let forbidden = [
        "database/connection.yml",
        "database/connections.yml",
        "database/secrets.yml",
        "database/.env",
        "database/reference-data/credentials.yml",
    ];
    for relative in forbidden {
        assert!(!dir.join(relative).exists(), "{relative} should not exist");
    }
}

#[test]
fn dirty_working_tree_warning_is_reported_and_init_is_blocked() {
    let dir = create_temp_dir("dirty");
    init_git_repo(&dir);
    fs::write(dir.join("untracked.txt"), "dirty").expect("write dirty file");

    let status = status_report(&dir, CommandKind::RepoStatus);
    assert!(status.is_dirty);
    assert!(!status.warnings.is_empty());

    let init = init_project(&dir, false).expect("init project");
    assert!(!init.success);
    assert!(init.created_paths.is_empty());
    assert!(!dir.join("database").exists());
}

#[test]
fn project_json_output_includes_expected_fields() {
    let dir = create_temp_dir("json");
    init_git_repo(&dir);

    let report = status_report(&dir, CommandKind::RepoStatus);
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"repositoryPath\"",
        "\"gitRoot\"",
        "\"isGitRepository\"",
        "\"branch\"",
        "\"workingTreeStatus\"",
        "\"isDirty\"",
        "\"dbstateProjectStatus\"",
        "\"missingPaths\"",
        "\"existingPaths\"",
        "\"plannedCreates\"",
        "\"createdPaths\"",
        "\"warnings\"",
        "\"errors\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }
}

#[test]
fn missing_postgres_url_returns_clear_error() {
    let report = inspect_postgres_command(None, None);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Missing PostgreSQL connection URL")));
}

#[test]
fn url_precedence_prefers_cli_url() {
    let cli_url = placeholder_url("cli-user", "cli-credential");
    let env_url = placeholder_url("env-user", "env-credential");
    let resolved =
        resolve_postgres_url(Some(cli_url.clone()), Some(env_url)).expect("resolved url");

    assert_eq!(resolved, cli_url);
}

#[test]
fn redaction_removes_raw_url_and_credential() {
    let credential = ["sensitive", "marker"].join("-");
    let raw = placeholder_url("user", &credential);
    let redacted = redact_message(&format!("could not connect to {raw}"), &raw);

    assert!(!redacted.contains(&raw));
    assert!(!redacted.contains(&credential));
    assert!(redacted.contains("<redacted>"));
}

#[test]
fn inspection_json_output_includes_expected_fields_and_no_secrets() {
    let mut report = empty_inspection_report(CommandKind::InspectPostgres);
    let raw = format!("{} failed", placeholder_url("user", "sensitive-marker"));
    report.errors.push(redact_postgres_url(&raw));
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"databaseType\"",
        "\"inspectionScope\"",
        "\"schemas\"",
        "\"tables\"",
        "\"columns\"",
        "\"extensions\"",
        "\"enums\"",
        "\"sequences\"",
        "\"indexes\"",
        "\"views\"",
        "\"counts\"",
        "\"warnings\"",
        "\"errors\"",
        "\"deferredObjectTypes\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }

    assert!(!json.contains("sensitive-marker"));
    assert!(!json.contains("postgres://"));
}

#[test]
fn inspect_command_does_not_create_repository_files() {
    let dir = create_temp_dir("inspect-no-files");
    init_git_repo(&dir);

    let before_database_exists = dir.join("database").exists();
    let report = inspect_postgres_command(None, None);

    assert!(!report.success);
    assert_eq!(before_database_exists, dir.join("database").exists());
}

#[test]
fn internal_schema_filtering_excludes_postgresql_schemas() {
    assert!(!is_user_schema("pg_catalog"));
    assert!(!is_user_schema("information_schema"));
    assert!(!is_user_schema("pg_toast"));
    assert!(!is_user_schema("pg_toast_temp_1"));
    assert!(!is_user_schema("pg_temp_1"));
    assert!(is_user_schema("public"));
    assert!(is_user_schema("app_core"));
}

#[test]
fn object_inventory_model_represents_schemas_tables_and_columns() {
    let inventory = PostgresInventory {
        schemas: vec![SchemaInfo {
            name: "app".to_string(),
        }],
        tables: vec![TableInfo {
            schema_name: "app".to_string(),
            table_name: "orders".to_string(),
            table_type: "BASE TABLE".to_string(),
        }],
        columns: vec![ColumnInfo {
            schema_name: "app".to_string(),
            table_name: "orders".to_string(),
            column_name: "id".to_string(),
            ordinal_position: 1,
            data_type: "integer".to_string(),
            is_nullable: false,
            has_default: true,
            default_expression: Some("nextval('orders_id_seq'::regclass)".to_string()),
        }],
        extensions: vec![ExtensionInfo {
            extension_name: "pgcrypto".to_string(),
            schema_name: Some("public".to_string()),
            version: Some("1.3".to_string()),
        }],
        enums: vec![EnumInfo {
            schema_name: "app".to_string(),
            enum_name: "order_status".to_string(),
            labels: vec!["new".to_string(), "paid".to_string()],
        }],
        sequences: vec![SequenceInfo {
            schema_name: "app".to_string(),
            sequence_name: "orders_id_seq".to_string(),
            data_type: Some("integer".to_string()),
            start_value: Some(1),
            min_value: Some(1),
            max_value: Some(2_147_483_647),
            increment_by: Some(1),
            cycle: false,
            cache_size: Some(1),
        }],
        indexes: vec![IndexInfo {
            schema_name: "app".to_string(),
            table_name: "orders".to_string(),
            index_name: "orders_created_at_idx".to_string(),
            is_unique: false,
            definition: "CREATE INDEX orders_created_at_idx ON app.orders USING btree (id)"
                .to_string(),
        }],
        views: vec![ViewInfo {
            schema_name: "app".to_string(),
            view_name: "open_orders".to_string(),
            definition: " SELECT orders.id FROM app.orders".to_string(),
        }],
    };

    assert_eq!(inventory.schemas[0].name, "app");
    assert_eq!(inventory.tables[0].table_name, "orders");
    assert_eq!(inventory.columns[0].column_name, "id");
    assert_eq!(inventory.extensions[0].extension_name, "pgcrypto");
    assert_eq!(inventory.enums[0].labels, vec!["new", "paid"]);
    assert_eq!(inventory.sequences[0].sequence_name, "orders_id_seq");
    assert_eq!(inventory.indexes[0].index_name, "orders_created_at_idx");
    assert_eq!(inventory.views[0].view_name, "open_orders");
}

#[test]
fn deferred_object_types_are_explicit() {
    let report = empty_inspection_report(CommandKind::InspectPostgres);

    assert!(!report
        .deferred_object_types
        .contains(&"extensions".to_string()));
    assert!(!report.deferred_object_types.contains(&"enums".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"sequences".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"indexes".to_string()));
    assert!(!report.deferred_object_types.contains(&"views".to_string()));
    assert!(report
        .deferred_object_types
        .contains(&"functions".to_string()));
    assert!(report.deferred_object_types.contains(&"grants".to_string()));
}

#[test]
fn missing_export_selection_returns_clear_error() {
    let error = ExportSelection::from_options(false, None, None).expect_err("selection error");
    assert!(error.contains("Selection is required"));
}

#[test]
fn export_paths_stay_under_database_objects() {
    assert_eq!(
        schema_file_path("core").expect("schema path"),
        "database/objects/schemas/core.sql"
    );
    assert_eq!(
        table_file_path("core", "payment_attempts").expect("table path"),
        "database/objects/tables/core.payment_attempts.sql"
    );
    assert_eq!(
        extension_file_path("pgcrypto").expect("extension path"),
        "database/objects/extensions/pgcrypto.sql"
    );
    assert_eq!(
        enum_file_path("core", "payment_status").expect("enum path"),
        "database/objects/enums/core.payment_status.sql"
    );
    assert_eq!(
        sequence_file_path("core", "payment_id_seq").expect("sequence path"),
        "database/objects/sequences/core.payment_id_seq.sql"
    );
    assert_eq!(
        index_file_path("core", "payments", "payments_code_idx").expect("index path"),
        "database/objects/indexes/core.payments.payments_code_idx.sql"
    );
    assert_eq!(
        view_file_path("core", "active_payments").expect("view path"),
        "database/objects/views/core.active_payments.sql"
    );
    assert!(schema_file_path("../evil").is_err());
    assert!(table_file_path("core", "bad/name").is_err());
    assert!(ensure_database_object_path("database/releases/bad.sql").is_err());
}

#[test]
fn identifier_quoting_handles_required_cases() {
    assert_eq!(quote_postgres_identifier("normal"), "\"normal\"");
    assert_eq!(quote_postgres_identifier("MixedCase"), "\"MixedCase\"");
    assert_eq!(quote_postgres_identifier("select"), "\"select\"");
    assert_eq!(quote_postgres_identifier("has\"quote"), "\"has\"\"quote\"");
}

#[test]
fn generated_schema_sql_matches_golden_expectation() {
    let expected = "-- DbState PostgreSQL desired-state object\n-- Object type: schema\n-- Object name: dbstate_slice2\n\nCREATE SCHEMA \"dbstate_slice2\";\n";
    assert_eq!(render_schema_sql("dbstate_slice2"), expected);
}

#[test]
fn generated_table_sql_matches_golden_expectation() {
    let expected = "-- DbState PostgreSQL desired-state object\n-- Object type: table\n-- Object name: dbstate_slice2.sample_accounts\n\nCREATE TABLE \"dbstate_slice2\".\"sample_accounts\" (\n    \"account_id\" integer NOT NULL,\n    \"account_code\" text NOT NULL,\n    \"display_name\" text,\n    \"created_at\" timestamp without time zone DEFAULT now() NOT NULL\n);\n";
    assert_eq!(
        render_table_sql(
            "dbstate_slice2",
            "sample_accounts",
            &sample_inventory().columns
        ),
        expected
    );
}

#[test]
fn generated_slice16_object_sql_is_deterministic() {
    let inventory = sample_inventory();

    assert!(render_extension_sql(&inventory.extensions[0])
        .contains("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\";"));
    assert!(render_enum_sql(&inventory.enums[0])
        .contains("CREATE TYPE \"dbstate_slice2\".\"account_status\" AS ENUM"));
    assert!(render_enum_sql(&inventory.enums[0]).contains("'active',"));
    assert!(render_sequence_sql(&inventory.sequences[0])
        .contains("CREATE SEQUENCE \"dbstate_slice2\".\"account_number_seq\""));
    assert!(render_index_sql(&inventory.indexes[0])
        .contains("CREATE UNIQUE INDEX sample_accounts_account_code_idx"));
    assert!(render_view_sql(&inventory.views[0])
        .contains("CREATE VIEW \"dbstate_slice2\".\"active_accounts\" AS"));
}

#[test]
fn export_dry_run_creates_no_files() {
    let dir = create_temp_dir("export-dry-run");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report =
        export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);

    assert!(report.success);
    assert!(!report.planned_files.is_empty());
    assert!(report.created_files.is_empty());
    assert!(!dir
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());
}

#[test]
fn export_requires_git_repository_and_dbstate_structure() {
    let non_git = create_temp_dir("export-non-git");
    let report =
        export_postgres_with_inventory(&non_git, &sample_inventory(), &ExportSelection::All, false);
    assert!(!report.success);
    assert!(report.errors[0].contains("not inside a Git repository"));

    let no_structure = create_temp_dir("export-no-structure");
    init_git_repo(&no_structure);
    let report = export_postgres_with_inventory(
        &no_structure,
        &sample_inventory(),
        &ExportSelection::All,
        false,
    );
    assert!(!report.success);
    assert!(report.errors[0].contains("Run dbstate init first"));
}

#[test]
fn export_write_is_blocked_when_working_tree_is_dirty() {
    let dir = create_temp_dir("export-dirty");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report =
        export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report.errors[0].contains("working tree has changes"));
    assert!(report.created_files.is_empty());
}

#[test]
fn export_does_not_overwrite_existing_files_by_default() {
    let dir = create_temp_dir("export-no-overwrite");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let existing = dir.join("database/objects/schemas/dbstate_slice2.sql");
    fs::write(&existing, "-- keep me\n").expect("write existing schema file");
    commit_all(&dir, "complete structure");

    let report = export_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Schema("dbstate_slice2".to_string()),
        true,
    );

    assert!(report.success);
    assert!(report
        .skipped_files
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert_eq!(
        fs::read_to_string(existing).expect("read existing"),
        "-- keep me\n"
    );
}

#[test]
fn table_export_warns_when_schema_file_is_missing() {
    let dir = create_temp_dir("export-table-warning");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report = export_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        true,
    );

    assert!(report.success);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("without its schema object file")));
}

#[test]
fn export_json_includes_expected_fields_and_no_secrets() {
    let dir = create_temp_dir("export-json");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report =
        export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"databaseType\"",
        "\"exportScope\"",
        "\"dryRun\"",
        "\"selectedSchemas\"",
        "\"selectedTables\"",
        "\"plannedFiles\"",
        "\"createdFiles\"",
        "\"skippedFiles\"",
        "\"warnings\"",
        "\"errors\"",
        "\"deferredObjectTypes\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("sensitive-marker"));
}

#[test]
fn actual_export_creates_schema_and_table_files() {
    let dir = create_temp_dir("export-write");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report =
        export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(report.success);
    assert!(dir
        .join("database/objects/schemas/dbstate_slice2.sql")
        .is_file());
    assert!(dir
        .join("database/objects/tables/dbstate_slice2.sample_accounts.sql")
        .is_file());
}

#[test]
fn sync_dry_run_creates_or_updates_no_files() {
    let dir = create_temp_dir("sync-dry-run");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);

    assert!(report.success);
    assert!(report
        .planned_creates
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
    assert!(!dir
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());
}

#[test]
fn sync_requires_git_repository_and_dbstate_structure() {
    let non_git = create_temp_dir("sync-non-git");
    let report =
        sync_postgres_with_inventory(&non_git, &sample_inventory(), &ExportSelection::All, false);
    assert!(!report.success);
    assert!(report.errors[0].contains("not inside a Git repository"));

    let no_structure = create_temp_dir("sync-no-structure");
    init_git_repo(&no_structure);
    let report = sync_postgres_with_inventory(
        &no_structure,
        &sample_inventory(),
        &ExportSelection::All,
        false,
    );
    assert!(!report.success);
    assert!(report.errors[0].contains("Run dbstate init first"));
}

#[test]
fn sync_write_is_blocked_when_working_tree_is_dirty() {
    let dir = create_temp_dir("sync-dirty");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report.errors[0].contains("working tree has changes"));
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
}

#[test]
fn sync_creates_added_object_files() {
    let dir = create_temp_dir("sync-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(report.success);
    assert!(report
        .created_files
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(report
        .created_files
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
}

#[test]
fn sync_updates_changed_object_files() {
    let dir = create_temp_dir("sync-update");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let table_path = dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    fs::write(&table_path, "-- stale table definition\n").expect("write stale table file");
    commit_all(&dir, "stale table");

    let report = sync_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        false,
    );

    assert!(report.success);
    assert!(report
        .updated_files
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    assert_eq!(
        fs::read_to_string(table_path).expect("read updated table"),
        render_table_sql(
            "dbstate_slice2",
            "sample_accounts",
            &sample_inventory().columns
        )
    );
}

#[test]
fn sync_leaves_unchanged_files_untouched() {
    let dir = create_temp_dir("sync-unchanged");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let schema_path = dir.join("database/objects/schemas/dbstate_slice2.sql");
    fs::write(&schema_path, render_schema_sql("dbstate_slice2")).expect("write schema file");
    commit_all(&dir, "schema file");

    let report = sync_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Schema("dbstate_slice2".to_string()),
        true,
    );

    assert!(report.success);
    assert!(report
        .unchanged_files
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
}

#[test]
fn sync_never_writes_under_releases() {
    let dir = create_temp_dir("sync-no-releases");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(report.success);
    assert!(report
        .created_files
        .iter()
        .all(|path| path.starts_with("database/objects/")));
    assert!(fs::read_dir(dir.join("database/releases"))
        .expect("read releases")
        .next()
        .is_none());
}

#[test]
fn sync_json_includes_expected_fields_and_no_secrets() {
    let dir = create_temp_dir("sync-json");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true);
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"databaseType\"",
        "\"syncScope\"",
        "\"dryRun\"",
        "\"selectedSchemas\"",
        "\"selectedTables\"",
        "\"addedFiles\"",
        "\"changedFiles\"",
        "\"unchangedFiles\"",
        "\"skippedFiles\"",
        "\"plannedCreates\"",
        "\"plannedUpdates\"",
        "\"createdFiles\"",
        "\"updatedFiles\"",
        "\"warnings\"",
        "\"errors\"",
        "\"deferredObjectTypes\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("sensitive-marker"));
}

#[test]
fn compare_missing_selection_returns_clear_error() {
    let dir = create_temp_dir("compare-missing-selection");
    let parsed =
        ParsedArgs::parse(&["compare".to_string(), "postgres".to_string()]).expect("parse");

    let report = compare_postgres_command(&dir, parsed);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Selection is required")));
}

#[test]
fn compare_requires_git_repository_and_dbstate_structure() {
    let non_git = create_temp_dir("compare-non-git");
    let report =
        compare_postgres_with_inventory(&non_git, &sample_inventory(), &ExportSelection::All);
    assert!(!report.success);
    assert!(report.errors[0].contains("not inside a Git repository"));

    let no_structure = create_temp_dir("compare-no-structure");
    init_git_repo(&no_structure);
    let report =
        compare_postgres_with_inventory(&no_structure, &sample_inventory(), &ExportSelection::All);
    assert!(!report.success);
    assert!(report.errors[0].contains("Run dbstate init first"));
}

#[test]
fn compare_is_read_only_and_can_run_with_dirty_working_tree() {
    let dir = create_temp_dir("compare-dirty-readonly");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success);
    assert!(report.is_dirty);
    assert!(report.in_sync.is_empty());
    assert!(report.repo_different.is_empty());
    assert!(report.repo_only.is_empty());
    assert!(!report.database_only.is_empty());
    assert!(!dir
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());
}

#[test]
fn repository_schema_and_table_files_are_discovered() {
    let dir = create_temp_dir("compare-discover");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        render_table_sql(
            "dbstate_slice2",
            "sample_accounts",
            &sample_inventory().columns,
        ),
    )
    .expect("write table");

    let import = discover_repository_objects(&dir).expect("discover objects");

    assert!(import.objects.contains_key("schema:dbstate_slice2"));
    assert!(import
        .objects
        .contains_key("table:dbstate_slice2.sample_accounts"));
    assert!(import.skipped.is_empty());
}

#[test]
fn repository_invalid_file_names_are_reported_as_skipped() {
    let dir = create_temp_dir("compare-invalid-names");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(dir.join("database/objects/tables/bad.txt"), "-- bad\n")
        .expect("write bad table file");
    fs::write(dir.join("database/objects/tables/a.b.c.sql"), "-- bad\n")
        .expect("write bad table file");

    let import = discover_repository_objects(&dir).expect("discover objects");

    assert!(import
        .skipped
        .contains(&"database/objects/tables/bad.txt".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/tables/a.b.c.sql".to_string()));
}

#[test]
fn compare_text_normalization_is_deterministic() {
    assert_eq!(
        normalize_desired_state_text("line one  \r\nline two\r\n\r\n"),
        "line one\nline two\n"
    );
    assert_eq!(normalize_desired_state_text(""), "");
}

#[test]
fn compare_classifies_in_sync_objects() {
    let dir = create_temp_dir("compare-in-sync");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        render_table_sql(
            "dbstate_slice2",
            "sample_accounts",
            &sample_inventory().columns,
        ),
    )
    .expect("write table");
    commit_all(&dir, "desired state files");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success);
    assert!(report
        .in_sync
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(report
        .in_sync
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
}

#[test]
fn compare_classifies_repo_different_objects() {
    let dir = create_temp_dir("compare-different");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- stale table\n",
    )
    .expect("write stale table");
    commit_all(&dir, "stale desired state");

    let report = compare_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
    );

    assert!(report.success);
    assert!(report
        .repo_different
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
}

#[test]
fn compare_classifies_repo_only_objects() {
    let dir = create_temp_dir("compare-repo-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.local_only.sql"),
        "-- local only\n",
    )
    .expect("write local only table");
    commit_all(&dir, "local only desired state");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success);
    assert!(report
        .repo_only
        .contains(&"database/objects/tables/dbstate_slice2.local_only.sql".to_string()));
}

#[test]
fn compare_classifies_database_only_objects() {
    let dir = create_temp_dir("compare-database-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success);
    assert!(report
        .database_only
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(report
        .database_only
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
}

#[test]
fn compare_json_includes_expected_fields_and_no_secrets() {
    let dir = create_temp_dir("compare-json");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let mut report =
        compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);
    report.warnings.push(redact_postgres_url(&placeholder_url(
        "user",
        "sensitive-marker",
    )));
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"databaseType\"",
        "\"compareScope\"",
        "\"selectedSchemas\"",
        "\"selectedTables\"",
        "\"inSync\"",
        "\"repoDifferent\"",
        "\"repoOnly\"",
        "\"databaseOnly\"",
        "\"skipped\"",
        "\"warnings\"",
        "\"errors\"",
        "\"deferredObjectTypes\"",
        "\"workingTreeStatus\"",
        "\"isDirty\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("sensitive-marker"));
}

#[test]
fn plan_missing_scope_selection_returns_clear_error() {
    let dir = create_temp_dir("plan-missing-selection");
    let parsed = ParsedArgs::parse(&["plan".to_string(), "postgres".to_string()]).expect("parse");

    let report = plan_postgres_command(&dir, parsed);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Selection is required")));
}

#[test]
fn plan_invalid_object_ref_returns_clear_error() {
    let error = PlanSelection::from_options(vec!["bad-ref".to_string()], Vec::new())
        .expect_err("invalid object ref");

    assert!(error.contains("Invalid object reference"));
}

#[test]
fn plan_requires_git_repository_and_dbstate_structure() {
    let non_git = create_temp_dir("plan-non-git");
    let report = plan_postgres_with_inventory(
        &non_git,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );
    assert!(!report.success);
    assert!(report.errors[0].contains("not inside a Git repository"));

    let no_structure = create_temp_dir("plan-no-structure");
    init_git_repo(&no_structure);
    let report = plan_postgres_with_inventory(
        &no_structure,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );
    assert!(!report.success);
    assert!(report.errors[0].contains("Run dbstate init first"));
}

#[test]
fn plan_is_read_only_and_can_run_with_dirty_working_tree() {
    let dir = create_temp_dir("plan-dirty-readonly");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );

    assert!(report.success);
    assert!(report.is_dirty);
    assert!(!dir
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());
    assert!(report
        .plan_items
        .iter()
        .any(|item| item.plan_intent == "reviewDatabaseOnly"));
}

#[test]
fn plan_builds_item_from_repo_different_object() {
    let dir = create_temp_dir("plan-repo-different");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- stale table\n",
    )
    .expect("write stale table");
    commit_all(&dir, "stale table");

    let report = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );

    assert!(report.success);
    assert!(report.plan_items.iter().any(|item| {
        item.object_ref == "table:dbstate_slice2.sample_accounts"
            && item.compare_classification == "repoDifferent"
            && item.plan_intent == "updateDatabaseLater"
    }));
}

#[test]
fn plan_builds_item_from_repo_only_object() {
    let dir = create_temp_dir("plan-repo-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    commit_all(&dir, "repo-only schema");

    let report = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );

    assert!(report.success);
    assert!(report.plan_items.iter().any(|item| {
        item.object_ref == "schema:local_only"
            && item.compare_classification == "repoOnly"
            && item.plan_intent == "createInDatabaseLater"
    }));
}

#[test]
fn plan_builds_review_item_from_database_only_object() {
    let dir = create_temp_dir("plan-database-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );

    assert!(report.success);
    assert!(report.plan_items.iter().any(|item| {
        item.object_ref == "schema:dbstate_slice2"
            && item.compare_classification == "databaseOnly"
            && item.plan_intent == "reviewDatabaseOnly"
    }));
}

#[test]
fn plan_include_limits_selected_plan_items() {
    let dir = create_temp_dir("plan-include");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        "-- stale schema\n",
    )
    .expect("write stale schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- stale table\n",
    )
    .expect("write stale table");
    commit_all(&dir, "stale files");

    let selection = PlanSelection::from_options(
        vec!["table:dbstate_slice2.sample_accounts".to_string()],
        Vec::new(),
    )
    .expect("plan selection");
    let report =
        plan_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, &selection);

    assert!(report.success);
    assert_eq!(report.plan_items.len(), 1);
    assert_eq!(
        report.plan_items[0].object_ref,
        "table:dbstate_slice2.sample_accounts"
    );
}

#[test]
fn plan_exclude_excludes_selected_objects() {
    let dir = create_temp_dir("plan-exclude");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        "-- stale schema\n",
    )
    .expect("write stale schema");
    commit_all(&dir, "stale schema");

    let selection =
        PlanSelection::from_options(Vec::new(), vec!["schema:dbstate_slice2".to_string()])
            .expect("plan selection");
    let report =
        plan_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, &selection);

    assert!(report.success);
    assert!(!report
        .plan_items
        .iter()
        .any(|item| item.object_ref == "schema:dbstate_slice2"));
    assert!(report
        .excluded_objects
        .contains(&"schema:dbstate_slice2".to_string()));
}

#[test]
fn plan_blocks_table_when_required_schema_file_is_missing() {
    let dir = create_temp_dir("plan-missing-schema");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- stale table\n",
    )
    .expect("write stale table");
    commit_all(&dir, "table without schema");

    let report = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
    );

    assert!(report.success);
    assert!(report.blocked_items.iter().any(|item| {
        item.object_ref == "table:dbstate_slice2.sample_accounts" && item.plan_intent == "blocked"
    }));
    assert!(report.dependency_warnings.iter().any(|warning| {
        warning.warning_type == "missingDependency" && warning.severity == "blocked"
    }));
}

#[test]
fn plan_warns_when_schema_is_excluded_for_selected_table() {
    let dir = create_temp_dir("plan-excluded-schema");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        "-- stale schema\n",
    )
    .expect("write stale schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- stale table\n",
    )
    .expect("write stale table");
    commit_all(&dir, "stale files");

    let selection = PlanSelection::from_options(
        vec!["table:dbstate_slice2.sample_accounts".to_string()],
        vec!["schema:dbstate_slice2".to_string()],
    )
    .expect("plan selection");
    let report =
        plan_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, &selection);

    assert!(report.success);
    assert!(report
        .blocked_items
        .iter()
        .any(|item| { item.object_ref == "table:dbstate_slice2.sample_accounts" }));
    assert!(report.dependency_warnings.iter().any(|warning| {
        warning.warning_type == "dependentObjectImpacted"
            && warning.object_ref == "table:dbstate_slice2.sample_accounts"
    }));
}

#[test]
fn plan_json_includes_expected_fields_and_no_secrets() {
    let dir = create_temp_dir("plan-json");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let mut report = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );
    report.warnings.push(redact_postgres_url(&placeholder_url(
        "user",
        "sensitive-marker",
    )));
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"databaseType\"",
        "\"planScope\"",
        "\"selectedSchemas\"",
        "\"selectedTables\"",
        "\"includedObjects\"",
        "\"excludedObjects\"",
        "\"planItems\"",
        "\"blockedItems\"",
        "\"dependencyWarnings\"",
        "\"compareSummary\"",
        "\"warnings\"",
        "\"errors\"",
        "\"deferredObjectTypes\"",
        "\"workingTreeStatus\"",
        "\"isDirty\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("sensitive-marker"));
}

#[test]
fn release_missing_name_returns_clear_error() {
    let dir = create_temp_dir("release-missing-name");
    let parsed = ParsedArgs::parse(&[
        "release".to_string(),
        "postgres".to_string(),
        "--all".to_string(),
    ])
    .expect("parse release");

    let report = release_postgres_command(&dir, parsed);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Release name is required")));
}

#[test]
fn release_invalid_name_returns_clear_error() {
    let dir = create_temp_dir("release-invalid-name");
    let parsed = ParsedArgs::parse(&[
        "release".to_string(),
        "postgres".to_string(),
        "--all".to_string(),
        "--name".to_string(),
        "../bad".to_string(),
    ])
    .expect("parse release");

    let report = release_postgres_command(&dir, parsed);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Release name must use only")));
}

#[test]
fn release_missing_scope_selection_returns_clear_error() {
    let dir = create_temp_dir("release-missing-scope");
    let parsed = ParsedArgs::parse(&[
        "release".to_string(),
        "postgres".to_string(),
        "--name".to_string(),
        "slice7".to_string(),
    ])
    .expect("parse release");

    let report = release_postgres_command(&dir, parsed);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Selection is required")));
}

#[test]
fn release_requires_git_repository_and_dbstate_structure() {
    let non_git = create_temp_dir("release-non-git");
    let report = release_postgres_with_inventory(
        &non_git,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
        "slice7",
        false,
    );
    assert!(!report.success);
    assert!(report.errors[0].contains("not inside a Git repository"));

    let no_structure = create_temp_dir("release-no-structure");
    init_git_repo(&no_structure);
    let report = release_postgres_with_inventory(
        &no_structure,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
        "slice7",
        false,
    );
    assert!(!report.success);
    assert!(report.errors[0].contains("Run dbstate init first"));
}

#[test]
fn release_write_is_blocked_when_working_tree_is_dirty() {
    let dir = create_temp_dir("release-dirty");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
        "slice7",
        false,
    );

    assert!(!report.success);
    assert!(report.errors[0].contains("working tree has changes"));
    assert!(report.created_artifacts.is_empty());
}

#[test]
fn release_dry_run_writes_no_files_and_plans_artifacts() {
    let dir = create_temp_dir("release-dry-run");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    commit_all(&dir, "repo-only schema");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["schema:local_only".to_string()], Vec::new())
            .expect("plan selection"),
        "slice7",
        true,
    );

    assert!(report.success);
    assert!(report
        .planned_artifacts
        .contains(&"database/releases/0001_slice7.sql".to_string()));
    assert!(report.created_artifacts.is_empty());
    assert!(!dir.join("database/releases/0001_slice7.sql").exists());
}

#[test]
fn release_generates_sql_summary_and_risk_json_under_releases() {
    let dir = create_temp_dir("release-write");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    fs::write(
        dir.join("database/objects/tables/local_only.accounts.sql"),
        render_table_sql("local_only", "accounts", &sample_inventory().columns),
    )
    .expect("write repo-only table");
    commit_all(&dir, "repo-only objects");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "schema:local_only".to_string(),
                "table:local_only.accounts".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "Slice7_Test",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.created_artifacts,
        vec![
            "database/releases/0001_slice7_test.sql".to_string(),
            "database/releases/0001_slice7_test.summary.md".to_string(),
            "database/releases/0001_slice7_test.risk.json".to_string(),
            "database/releases/0001_slice7_test.manifest.json".to_string(),
        ]
    );
    let sql = fs::read_to_string(dir.join("database/releases/0001_slice7_test.sql")).expect("sql");
    let summary = fs::read_to_string(dir.join("database/releases/0001_slice7_test.summary.md"))
        .expect("summary");
    let risk =
        fs::read_to_string(dir.join("database/releases/0001_slice7_test.risk.json")).expect("risk");
    let manifest = fs::read_to_string(dir.join("database/releases/0001_slice7_test.manifest.json"))
        .expect("manifest");

    assert!(sql.contains("-- DbState Release Artifact"));
    assert!(sql.contains("-- Safety: Review-only. DbState does not execute this SQL."));
    assert!(sql.contains("BEGIN REVIEW SECTION: Summary"));
    assert!(sql.contains("BEGIN REVIEW SECTION: Creates"));
    assert!(sql.contains("BEGIN REVIEW SECTION: Review Required"));
    assert!(sql.contains("BEGIN REVIEW SECTION: Blocked Items"));
    assert!(sql.contains("BEGIN REVIEW SECTION: Deferred Object Types"));
    assert!(sql.contains("CREATE SCHEMA IF NOT EXISTS \"local_only\";"));
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"local_only\".\"accounts\""));
    for forbidden in [
        "DROP TABLE",
        "DROP SCHEMA",
        "ALTER TABLE DROP",
        "TRUNCATE",
        "DELETE FROM",
        "INSERT INTO",
    ] {
        assert!(!sql.contains(forbidden), "forbidden SQL found: {forbidden}");
    }
    assert!(summary.contains("## Reviewer Checklist"));
    assert!(summary.contains("Review all REVIEW REQUIRED comments"));
    assert!(summary.contains("Confirm no destructive SQL is present"));
    assert!(summary.contains("DbState did not execute SQL"));
    assert!(summary.contains("## Object Counts By Status"));
    assert!(summary.contains("## Risk Summary"));
    assert!(risk.contains("\"command\":\"release postgres\""));
    assert!(risk.contains("\"releaseSequence\":\"0001\""));
    assert!(risk.contains("\"generatedArtifacts\""));
    assert!(risk.contains("\"repositoryContext\""));
    assert!(risk.contains("\"counts\""));
    assert!(risk.contains("\"objectTypeCounts\""));
    assert!(risk.contains("\"riskLevel\":\"low\""));
    assert!(risk.contains("\"riskReasons\""));
    assert!(risk.contains("\"destructiveSqlGenerated\":false"));
    assert!(risk.contains("\"directApplyAvailable\":false"));
    assert!(risk.contains("\"generatedSqlExecutionSupported\":false"));
    assert!(risk.contains("\"databaseMutationPerformed\":false"));
    assert!(risk.contains("\"gitMutationPerformed\":false"));
    assert!(risk.contains("\"credentialPersistencePerformed\":false"));
    assert!(manifest.contains("\"releaseName\":\"Slice7_Test\""));
    assert!(manifest.contains("\"artifactType\":\"sql\""));
    assert!(manifest.contains("\"artifactType\":\"manifest\""));
    assert!(!sql.contains("postgres://"));
    assert!(!summary.contains("postgres://"));
    assert!(!risk.contains("postgres://"));
    assert!(!manifest.contains("postgres://"));
}

#[test]
fn release_does_not_overwrite_existing_artifacts() {
    let dir = create_temp_dir("release-sequence");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/releases/0001_slice7.sql"),
        "-- existing\n",
    )
    .expect("write existing sql");
    fs::write(
        dir.join("database/releases/0001_slice7.summary.md"),
        "existing\n",
    )
    .expect("write existing summary");
    fs::write(dir.join("database/releases/0001_slice7.risk.json"), "{}\n")
        .expect("write existing risk");
    fs::write(
        dir.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    commit_all(&dir, "existing artifacts and repo-only schema");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["schema:local_only".to_string()], Vec::new())
            .expect("plan selection"),
        "slice7",
        true,
    );

    assert!(report.success);
    assert!(report
        .planned_artifacts
        .contains(&"database/releases/0002_slice7.sql".to_string()));
}

#[test]
fn release_blocks_when_selected_plan_items_are_blocked() {
    let dir = create_temp_dir("release-blocked");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- local drift\n",
    )
    .expect("write table without schema file");
    commit_all(&dir, "table without schema");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
        "slice7",
        false,
    );

    assert!(!report.success);
    assert_eq!(report.risk_level, "blocked");
    assert!(report.created_artifacts.is_empty());
    assert!(report
        .blocked_items
        .iter()
        .any(|item| item.object_ref == "table:dbstate_slice2.sample_accounts"));
}

#[test]
fn changed_table_release_uses_review_only_comment_not_alter() {
    let dir = create_temp_dir("release-changed-table");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- local drift\n",
    )
    .expect("write changed table");
    commit_all(&dir, "changed table");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
        "slice7",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    let sql = fs::read_to_string(dir.join("database/releases/0001_slice7.sql")).expect("read sql");
    assert!(sql.contains("REVIEW REQUIRED: object differs"));
    assert!(!sql.contains("\nALTER TABLE "));
}

#[test]
fn release_json_includes_expected_fields_and_no_secrets() {
    let mut report = empty_release_report(true);
    report.release_name = "slice7".to_string();
    report.release_scope = "all".to_string();
    report
        .planned_artifacts
        .push("database/releases/0001_slice7.sql".to_string());
    report.warnings.push(redact_postgres_url(&placeholder_url(
        "user",
        "sensitive-marker",
    )));
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"databaseType\"",
        "\"releaseName\"",
        "\"releaseScope\"",
        "\"dryRun\"",
        "\"selectedSchemas\"",
        "\"selectedTables\"",
        "\"includedObjects\"",
        "\"excludedObjects\"",
        "\"planItems\"",
        "\"blockedItems\"",
        "\"dependencyWarnings\"",
        "\"plannedArtifacts\"",
        "\"createdArtifacts\"",
        "\"riskLevel\"",
        "\"warnings\"",
        "\"errors\"",
        "\"deferredObjectTypes\"",
        "\"workingTreeStatus\"",
        "\"isDirty\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("sensitive-marker"));
}

#[test]
fn data_compare_missing_scope_selection_returns_clear_error() {
    let dir = create_temp_dir("data-compare-missing-scope");
    let parsed =
        ParsedArgs::parse(&["data-compare".to_string(), "postgres".to_string()]).expect("parse");

    let report = data_compare_postgres_command(&dir, parsed);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Selection is required")));
}

#[test]
fn data_compare_requires_git_repository_and_dbstate_structure() {
    let non_git = create_temp_dir("data-compare-non-git");
    let report = data_compare_postgres_with_connection(
        &non_git,
        &placeholder_url("user", "secret"),
        &ReferenceDataSelection::All,
    )
    .expect("report");
    assert!(!report.success);
    assert!(report.errors[0].contains("not inside a Git repository"));

    let no_structure = create_temp_dir("data-compare-no-structure");
    init_git_repo(&no_structure);
    let report = data_compare_postgres_with_connection(
        &no_structure,
        &placeholder_url("user", "secret"),
        &ReferenceDataSelection::All,
    )
    .expect("report");
    assert!(!report.success);
    assert!(report.errors[0].contains("Run dbstate init first"));
}

#[test]
fn data_compare_empty_registry_is_valid_and_read_only_when_dirty() {
    let dir = create_temp_dir("data-compare-empty-registry");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report = data_compare_postgres_with_connection(
        &dir,
        &placeholder_url("user", "secret"),
        &ReferenceDataSelection::All,
    )
    .expect("report");

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.selected_tables.len(), 0);
    assert!(report.is_dirty);
    assert!(!dir.join("database/releases/0001_data.sql").exists());
}

#[test]
fn data_compare_invalid_postgres_url_returns_clear_error() {
    let dir = create_temp_dir("data-compare-invalid-url");
    init_git_repo(&dir);
    create_complete_structure(&dir);

    let report = data_compare_postgres_with_connection(
        &dir,
        "not-a-postgres-url",
        &ReferenceDataSelection::All,
    )
    .expect("report");

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Invalid PostgreSQL connection URL")));
    assert!(!report
        .errors
        .iter()
        .any(|error| error.contains("not-a-postgres-url")));
}

#[test]
fn reference_registry_parser_accepts_valid_configured_table() {
    let registry =
        parse_reference_data_registry(valid_reference_registry_yaml()).expect("registry");

    assert_eq!(registry.version, 1);
    assert_eq!(registry.tables.len(), 1);
    assert_eq!(registry.tables[0].name, "dbstate_ref.payment_methods");
    assert_eq!(
        registry.tables[0].file,
        "tables/dbstate_ref.payment_methods.yml"
    );
    assert_eq!(registry.tables[0].key_columns, vec!["code".to_string()]);
    assert_eq!(
        registry.tables[0].ignore_columns,
        vec!["updated_at".to_string()]
    );
    assert_eq!(
        registry.tables[0].masked_columns,
        vec!["secret_note".to_string()]
    );
}

#[test]
fn reference_registry_parser_rejects_missing_key() {
    let yaml = "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
";
    let error = parse_reference_data_registry(yaml).expect_err("missing key");

    assert!(error.contains("missing required field 'key'"));
}

#[test]
fn reference_registry_parser_rejects_masked_or_ignored_key() {
    let masked = "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
    key: [code]
    maskedColumns: [code]
";
    let ignored = "version: 1
tables:
  - name: dbstate_ref.payment_methods
    file: tables/dbstate_ref.payment_methods.yml
    key: [code]
    ignoreColumns: [code]
";

    assert!(parse_reference_data_registry(masked)
        .expect_err("masked key")
        .contains("cannot be masked"));
    assert!(parse_reference_data_registry(ignored)
        .expect_err("ignored key")
        .contains("cannot be ignored"));
}

#[test]
fn reference_table_file_parser_accepts_valid_rows() {
    let (config, state) = reference_config_and_state();

    assert_eq!(state.table_name, config.name);
    assert_eq!(state.rows.len(), 3);
    assert_eq!(
        reference_row_key(&state.rows[0], &config.key_columns).expect("row key"),
        "code=CASH"
    );
}

#[test]
fn reference_table_file_parser_rejects_duplicate_row_keys() {
    let (config, _) = reference_config_and_state();
    let yaml = "table: dbstate_ref.payment_methods
key: [code]
rows:
  - code: CASH
    name: Cash
  - code: CASH
    name: Cash Duplicate
";
    let error = parse_reference_data_table_state(yaml, &config).expect_err("duplicate key");

    assert!(error.contains("duplicate row key"));
}

#[test]
fn reference_table_file_parser_rejects_missing_key_column() {
    let (config, _) = reference_config_and_state();
    let yaml = "table: dbstate_ref.payment_methods
key: [code]
rows:
  - name: Missing Code
";
    let error = parse_reference_data_table_state(yaml, &config).expect_err("missing key");

    assert!(error.contains("missing key column"));
}

#[test]
fn data_compare_selected_unconfigured_table_returns_clear_error() {
    let dir = create_temp_dir("data-compare-unconfigured");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report = data_compare_postgres_with_connection(
        &dir,
        &placeholder_url("user", "secret"),
        &ReferenceDataSelection::Table("dbstate_ref.missing".to_string()),
    )
    .expect("report");

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("not configured")));
}

#[test]
fn reference_data_compare_classifies_rows_and_ignores_columns() {
    let (config, state) = reference_config_and_state();
    let database_rows = vec![
        reference_row(&[
            ("code", Some("CASH")),
            ("name", Some("Cash")),
            ("is_active", Some("true")),
            ("sort_order", Some("10")),
            ("updated_at", Some("different ignored value")),
        ]),
        reference_row(&[
            ("code", Some("QRPH")),
            ("name", Some("QRPh Live")),
            ("is_active", Some("true")),
            ("sort_order", Some("20")),
            ("updated_at", Some("different ignored value")),
        ]),
        reference_row(&[
            ("code", Some("CARD")),
            ("name", Some("Card")),
            ("is_active", Some("true")),
            ("sort_order", Some("30")),
        ]),
    ];
    let result = compare_reference_data_table(
        &config,
        &state,
        &database_rows,
        &reference_database_columns(),
    );

    assert_eq!(result.row_counts.in_sync, 1);
    assert_eq!(result.row_counts.repo_different, 1);
    assert_eq!(result.row_counts.repo_only, 1);
    assert_eq!(result.row_counts.database_only, 1);
    let different = result
        .row_results
        .iter()
        .find(|row| row.row_key == "code=QRPH")
        .expect("different row");
    assert_eq!(different.classification, "repoDifferent");
    assert_eq!(different.changed_columns, vec!["name".to_string()]);
    assert!(!different
        .changed_columns
        .contains(&"updated_at".to_string()));
}

#[test]
fn reference_data_compare_masks_columns_without_exposing_values() {
    let (config, state) = reference_config_and_state();
    let database_rows = vec![reference_row(&[
        ("code", Some("CASH")),
        ("name", Some("Cash")),
        ("is_active", Some("true")),
        ("sort_order", Some("10")),
        ("secret_note", Some("database-secret-value")),
    ])];
    let result = compare_reference_data_table(
        &config,
        &state,
        &database_rows,
        &reference_database_columns(),
    );
    let mut report = empty_reference_data_compare_report();
    append_reference_table_result(&mut report, result);
    report.success = true;
    let json = report.to_json();
    let text = report.to_text();

    assert!(json.contains("secret_note"));
    assert!(!json.contains("repo-secret-cash"));
    assert!(!json.contains("database-secret-value"));
    assert!(!text.contains("repo-secret-cash"));
    assert!(!text.contains("database-secret-value"));
}

#[test]
fn data_compare_json_includes_expected_fields_and_no_secrets() {
    let mut report = empty_reference_data_compare_report();
    report.compare_scope = "all".to_string();
    report.warnings.push(redact_postgres_url(&placeholder_url(
        "user",
        "sensitive-marker",
    )));
    let json = report.to_json();

    for field in [
        "\"command\"",
        "\"success\"",
        "\"repositoryPath\"",
        "\"gitRoot\"",
        "\"isGitRepository\"",
        "\"branch\"",
        "\"workingTreeStatus\"",
        "\"isDirty\"",
        "\"databaseType\"",
        "\"compareScope\"",
        "\"dataCompareScope\"",
        "\"selectedTables\"",
        "\"tableResults\"",
        "\"counts\"",
        "\"inSync\"",
        "\"repoDifferent\"",
        "\"repoOnly\"",
        "\"databaseOnly\"",
        "\"skipped\"",
        "\"warnings\"",
        "\"errors\"",
    ] {
        assert!(json.contains(field), "missing JSON field {field}");
    }
    assert!(!json.contains("postgres://"));
    assert!(!json.contains("sensitive-marker"));
}

fn assert_common_json_contract(json: &str) {
    for field in ["\"command\"", "\"success\"", "\"warnings\"", "\"errors\""] {
        assert!(json.contains(field), "missing common JSON field {field}");
    }
}

fn assert_repository_json_contract(json: &str) {
    for field in [
        "\"repositoryPath\"",
        "\"gitRoot\"",
        "\"isGitRepository\"",
        "\"branch\"",
        "\"workingTreeStatus\"",
        "\"isDirty\"",
    ] {
        assert!(
            json.contains(field),
            "missing repository JSON field {field}"
        );
    }
}

fn assert_postgres_json_contract(json: &str) {
    assert!(json.contains("\"databaseType\":\"postgresql\""));
}

#[test]
fn slice9_common_json_contract_fields_are_present() {
    let dir = create_temp_dir("slice9-json-contract");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let project_json = status_report(&dir, CommandKind::RepoStatus).to_json();
    assert_common_json_contract(&project_json);
    assert_repository_json_contract(&project_json);

    let inspection_json = empty_inspection_report(CommandKind::InspectPostgres).to_json();
    assert_common_json_contract(&inspection_json);
    assert_postgres_json_contract(&inspection_json);

    let export_json =
        export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true)
            .to_json();
    assert_common_json_contract(&export_json);
    assert_repository_json_contract(&export_json);
    assert_postgres_json_contract(&export_json);

    let sync_json =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, true)
            .to_json();
    assert_common_json_contract(&sync_json);
    assert_repository_json_contract(&sync_json);
    assert_postgres_json_contract(&sync_json);

    let compare_json =
        compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All).to_json();
    assert_common_json_contract(&compare_json);
    assert_repository_json_contract(&compare_json);
    assert_postgres_json_contract(&compare_json);

    let plan_json = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
    )
    .to_json();
    assert_common_json_contract(&plan_json);
    assert_repository_json_contract(&plan_json);
    assert_postgres_json_contract(&plan_json);

    let release_json = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::include_all(),
        "slice9_contract",
        true,
    )
    .to_json();
    assert_common_json_contract(&release_json);
    assert_repository_json_contract(&release_json);
    assert_postgres_json_contract(&release_json);

    let data_compare_json = data_compare_postgres_with_connection(
        &dir,
        &placeholder_url("user", "contract-secret"),
        &ReferenceDataSelection::All,
    )
    .expect("data compare report")
    .to_json();
    assert_common_json_contract(&data_compare_json);
    assert_repository_json_contract(&data_compare_json);
    assert_postgres_json_contract(&data_compare_json);
    assert!(data_compare_json.contains("\"dataCompareScope\""));
    assert!(!data_compare_json.contains("contract-secret"));
    assert!(!data_compare_json.contains("postgres://"));
}

#[test]
fn slice9_usage_includes_all_current_commands_and_common_options() {
    let usage = usage();

    for expected in [
        "dbstate repo status",
        "dbstate init",
        "dbstate inspect postgres",
        "dbstate export postgres",
        "dbstate sync postgres",
        "dbstate compare postgres",
        "dbstate plan postgres",
        "dbstate release postgres",
        "dbstate data-compare postgres",
        "--format json",
        "--json",
        "--url <postgres-url>",
        "--dry-run",
        "--all",
        "--schema",
        "--table",
        "--include",
        "--exclude",
        "--name",
        "dbstate serve",
        "--host <host>",
        "--port <port>",
    ] {
        assert!(usage.contains(expected), "usage missing {expected}");
    }
}

#[test]
fn slice11_service_routes_include_only_approved_endpoints() {
    let routes = service_route_definitions();

    for expected in [
        ("GET", "/"),
        ("GET", "/ui"),
        ("GET", "/ui/"),
        ("GET", "/ui/app.css"),
        ("GET", "/ui/app.js"),
        ("GET", "/health"),
        ("GET", "/api/v1/health"),
        ("GET", "/api/v1/workspace/roots"),
        ("POST", "/api/v1/workspace/list-directories"),
        ("POST", "/api/v1/workspace/validate"),
        ("GET", "/api/v1/connections/profiles"),
        ("POST", "/api/v1/connections/profiles"),
        ("PUT", "/api/v1/connections/profiles/{name}"),
        ("DELETE", "/api/v1/connections/profiles/{name}"),
        ("POST", "/api/v1/connections/test"),
        ("POST", "/api/v1/repo/status"),
        ("POST", "/api/v1/init/plan"),
        ("POST", "/api/v1/init/write"),
        ("POST", "/api/v1/postgres/inspect"),
        ("POST", "/api/v1/postgres/compare"),
        ("POST", "/api/v1/postgres/plan"),
        ("POST", "/api/v1/postgres/data-compare"),
        ("POST", "/api/v1/postgres/object-ddl"),
        ("POST", "/api/v1/postgres/repository-sync/preview"),
        ("POST", "/api/v1/postgres/repository-sync/write"),
        ("POST", "/api/v1/postgres/release/preview"),
        ("POST", "/api/v1/postgres/release/write"),
    ] {
        assert!(routes.contains(&expected), "missing route {expected:?}");
    }

    for (_, path) in routes {
        assert!(!path.contains("export"));
        assert!(!path.contains("/api/v1/postgres/sync"));
        assert!(!path.contains("apply"));
    }
}

#[test]
fn slice12_ui_routes_serve_static_assets() {
    let dir = create_temp_dir("slice12-ui-routes");

    let root = service_response("GET", "/", "", &dir);
    assert_eq!(root.status_code, 200);
    assert!(root.content_type.contains("text/html"));
    assert!(root.body.contains("DbState PostgreSQL v0.1"));

    let ui = service_response("GET", "/ui", "", &dir);
    assert_eq!(ui.status_code, 200);
    assert!(ui.content_type.contains("text/html"));

    let ui_slash = service_response("GET", "/ui/", "", &dir);
    assert_eq!(ui_slash.status_code, 200);
    assert!(ui_slash.content_type.contains("text/html"));

    let css = service_response("GET", "/ui/app.css", "", &dir);
    assert_eq!(css.status_code, 200);
    assert!(css.content_type.contains("text/css"));
    assert!(css.body.contains(".workflow-panel"));

    let js = service_response("GET", "/ui/app.js", "", &dir);
    assert_eq!(js.status_code, 200);
    assert!(js.content_type.contains("application/javascript"));
    assert!(js.body.contains("/api/v1/health"));

    let versioned_js = service_response("GET", "/ui/app.js?v=slice22-beta-ui", "", &dir);
    assert_eq!(versioned_js.status_code, 200);
    assert!(versioned_js.body.contains("directionForWorkflowMode"));
}

#[test]
fn slice12_ui_html_contains_safety_messages_and_no_external_assets() {
    let html = ui_html();

    assert!(html.contains("Local only"));
    assert!(html.contains("No SQL execution"));
    assert!(html.contains("direct database apply"));
    assert!(html.contains("Controlled local repository/release file writes"));
    assert!(html.contains("Schema compare workflow shell"));
    assert!(html.contains("Init Plan"));
    assert!(html.contains("Initialize DbState Project"));
    assert!(html.contains("INITIALIZE DBSTATE PROJECT"));
    assert!(html.contains("Workspace"));
    assert!(html.contains("Source &amp; Target"));
    assert!(html.contains("Compare Options"));
    assert!(html.contains("Results"));
    assert!(html.contains("Object Diff"));
    assert!(html.contains("Warnings"));
    assert!(html.contains("Release Plan"));
    assert!(html.contains("Reports / Raw JSON"));
    assert!(html.contains("About / Safety"));
    assert!(html.contains("workspace-path"));
    assert!(html.contains("workspace-browse"));
    assert!(html.contains("directory-picker"));
    assert!(html.contains("directory-picker-path"));
    assert!(html.contains("directory-list"));
    assert!(html.contains("Session-only"));
    assert!(html.contains("does not clone or fetch repositories"));
    assert!(html.contains("results-grid"));
    assert!(html.contains("selected-json"));
    assert!(html.contains("warnings-list"));
    assert!(html.contains("object-type-filter"));
    assert!(html.contains("status-filter"));
    assert!(html.contains("status-legend"));
    assert!(html.contains("results-source"));
    assert!(html.contains("results-target"));
    assert!(!html.contains("<th>Source</th>"));
    assert!(!html.contains("<th>Target</th>"));
    assert!(html.contains("release-name"));
    assert!(html.contains("Reviewer Checklist"));
    assert!(html.contains("dbstate release postgres --all --name"));
    assert!(html.contains("Raw JSON"));
    assert!(html.contains("/ui/app.css?v=slice22-beta-ui"));
    assert!(html.contains("/ui/app.js?v=slice22-beta-ui"));
    assert!(!html.contains("http://"));
    assert!(!html.contains("https://"));
    assert!(!html.contains("cdn"));
    assert!(!html.contains("unpkg"));
    assert!(!html.contains("jsdelivr"));
}

#[test]
fn slice12_ui_javascript_calls_only_approved_endpoints() {
    let js = ui_js();
    let approved = [
        "/api/v1/health",
        "/api/v1/connections/profiles",
        "/api/v1/connections/test",
        "/api/v1/repo/status",
        "/api/v1/init/plan",
        "/api/v1/init/write",
        "/api/v1/postgres/inspect",
        "/api/v1/postgres/compare",
        "/api/v1/postgres/plan",
        "/api/v1/postgres/data-compare",
        "/api/v1/postgres/object-ddl",
        "/api/v1/postgres/repository-sync/preview",
        "/api/v1/postgres/repository-sync/write",
        "/api/v1/postgres/release/preview",
        "/api/v1/postgres/release/write",
        "/api/v1/workspace/roots",
        "/api/v1/workspace/list-directories",
        "/api/v1/workspace/validate",
    ];

    for endpoint in approved {
        assert!(
            js.contains(endpoint),
            "missing approved endpoint {endpoint}"
        );
    }

    for forbidden in [
        "/api/v1/postgres/export",
        "/api/v1/release",
        "/api/v1/postgres/apply",
        "localStorage",
        "sessionStorage",
        "showDirectoryPicker",
        "console.log",
        "execute generated SQL",
        "execute SQL",
        "sync to database",
        "directApply",
        "mutateDatabase",
        "clone",
        "git fetch",
        "git pull",
        "git push",
        "git add",
        "git commit",
    ] {
        assert!(
            !js.contains(forbidden),
            "UI JavaScript contains forbidden pattern {forbidden}"
        );
    }
}

#[test]
fn ui_contains_stable_playwright_demo_selectors() {
    let html = ui_html();
    let js = ui_js();
    let expected_html_selectors = [
        "data-testid=\"tab-workspace\"",
        "data-testid=\"workspace-browse\"",
        "data-testid=\"workspace-health\"",
        "data-testid=\"workspace-check\"",
        "data-testid=\"workspace-repo-status\"",
        "data-testid=\"workspace-init-plan\"",
        "data-testid=\"workspace-initialize-project\"",
        "data-testid=\"tab-source-target\"",
        "data-testid=\"workflow-mode\"",
        "data-testid=\"connection-mode\"",
        "data-testid=\"source-connection-input\"",
        "data-testid=\"target-connection-input\"",
        "data-testid=\"source-target-run\"",
        "data-testid=\"tab-compare-options\"",
        "data-testid=\"preview-repository-sync\"",
        "data-testid=\"write-repository-changes\"",
        "data-testid=\"tab-results\"",
        "data-testid=\"results-object-type-filter\"",
        "data-testid=\"results-status-filter\"",
        "data-testid=\"results-table\"",
        "data-testid=\"results-visible-row-count\"",
        "data-testid=\"results-included-count\"",
        "data-testid=\"results-status-legend\"",
        "data-testid=\"tab-object-diff\"",
        "data-testid=\"object-diff-left\"",
        "data-testid=\"object-diff-right\"",
        "data-testid=\"object-diff-repository\"",
        "data-testid=\"object-diff-database\"",
        "data-testid=\"selected-json-item\"",
        "data-testid=\"tab-warnings\"",
        "data-testid=\"warnings-panel\"",
        "data-testid=\"warnings-list\"",
        "data-testid=\"tab-release-plan\"",
        "data-testid=\"release-plan-dry-run\"",
        "data-testid=\"release-context\"",
        "data-testid=\"risk-summary\"",
        "data-testid=\"object-summary\"",
        "data-testid=\"release-candidates\"",
        "data-testid=\"generated-artifacts\"",
        "data-testid=\"generate-release-artifact\"",
        "data-testid=\"tab-reports-raw-json\"",
        "data-testid=\"reports-panel\"",
        "data-testid=\"raw-json-panel\"",
        "data-testid=\"raw-json-copy\"",
    ];

    for selector in expected_html_selectors {
        assert!(
            html.contains(selector),
            "missing stable demo selector {selector}"
        );
    }

    for expected in [
            "tr.setAttribute(\"data-testid\", \"results-row-actor\")",
            "include.setAttribute(\"data-testid\", \"results-include-checkbox\")",
            "td.setAttribute(\"data-testid\", \"results-row-first-selectable\")",
            "function setObjectDiffDdlTestIds(direction)",
            "sourceDetail.setAttribute(\"data-testid\", direction.sourceDdlSide === \"database\" ? \"object-diff-database\" : \"object-diff-repository\")",
            "targetDetail.setAttribute(\"data-testid\", direction.targetDdlSide === \"repository\" ? \"object-diff-repository\" : \"object-diff-database\")",
        ] {
            assert!(js.contains(expected), "missing JS selector path {expected}");
        }

    for not_added in [
        "data-testid=\"selected-json-copy\"",
        "data-testid=\"warnings-empty-state\"",
        "data-testid=\"raw-json-download\"",
    ] {
        assert!(
            !html.contains(not_added),
            "selector should not exist without a corresponding UI element: {not_added}"
        );
    }
}

#[test]
fn slice22_object_diff_markers_are_visual_only_and_beta_colored() {
    let css = ui_css();
    let js = ui_js();

    assert!(css.contains(".diff-marker"));
    assert!(css.contains("user-select: none"));
    assert!(css.contains(".diff-line-text"));
    assert!(css.contains(".diff-line-same {\n  color: #ffffff;"));
    assert!(css.contains(".diff-line-source-only {\n  color: #7ee2a8;"));
    assert!(css.contains(".diff-line-target-only {\n  color: #ff8f85;"));
    assert!(!css.contains("text-decoration: line-through"));

    assert!(js.contains("appendVisualDiffLine(sourceLine"));
    assert!(js.contains("appendVisualDiffLine(targetLine"));
    assert!(js.contains("markerElement.className = \"diff-marker\""));
    assert!(js.contains("textElement.className = \"diff-line-text\""));
    assert!(js.contains("markerElement.setAttribute(\"aria-hidden\", \"true\")"));
    assert!(!js.contains("\"+ \" + row.source"));
    assert!(!js.contains("\"- \" + row.target"));
    assert!(!js.contains("line-through"));
}

#[test]
fn slice22_current_private_beta_text_avoids_older_slice_labels() {
    let combined = format!("{}\n{}\n{}", ui_html(), ui_css(), ui_js());

    for forbidden in [
        "Slice 16A",
        "Slice 16.A",
        "Slice 17",
        "automatic ALTER is not generated in Slice",
        "Full dependency ordering is not implemented in Slice",
    ] {
        assert!(
            !combined.contains(forbidden),
            "current UI assets expose old implementation label {forbidden}"
        );
    }
}

#[test]
fn slice13a_ui_html_contains_schema_compare_workflow_structure() {
    let html = ui_html();

    for expected in [
        "Workspace",
        "Source &amp; Target",
        "Compare Options",
        "Results",
        "Object Diff",
        "Warnings",
        "Release Plan",
        "Reports / Raw JSON",
        "About / Safety",
        "results-grid",
        "Object type",
        "Planned operation",
        "Source",
        "Target",
        "Source type",
        "Target type",
        "Diff detail not available yet",
        "reviewable files",
    ] {
        assert!(
            html.contains(expected),
            "missing UI workflow text {expected}"
        );
    }

    for forbidden in [
        "React",
        "Vue",
        "Svelte",
        "Angular",
        "Vite",
        "node_modules",
        "unpkg",
        "jsdelivr",
        "Deploy to database",
        "Execute SQL",
        "Sync to Database",
    ] {
        assert!(
            !html.contains(forbidden),
            "UI HTML contains forbidden pattern {forbidden}"
        );
    }
}

#[test]
fn slice13b_results_grid_usability_contract_is_present() {
    let html = ui_html();
    let css = ui_css();
    let js = ui_js();

    assert!(html.contains("<label for=\"object-type-filter\">Object type"));
    assert!(html.contains("<option value=\"all\">All</option>"));
    assert!(html.contains("<option value=\"schema\">Schema</option>"));
    assert!(html.contains("<option value=\"table\">Table</option>"));
    assert!(!html.contains("<option value=\"column\">Column</option>"));
    assert!(!html.contains("<option value=\"referenceData\">Reference data</option>"));
    assert!(html.contains("<select id=\"compare-schema\">"));
    assert!(html.contains("<select id=\"compare-table\">"));
    assert!(html.contains("<select id=\"data-table\" disabled>"));
    assert!(html.contains("Run Inspect first to populate schema and table lists."));
    assert!(html.contains("reference-data compare are out-of-scope"));
    assert!(!html.contains("placeholder=\"dbstate_slice2\""));
    assert!(!html.contains("placeholder=\"schema.table\""));
    assert!(!html.contains("table:dbstate_slice2.sample_accounts"));
    assert!(!html.contains("schema:public"));
    assert!(html.contains("status-legend"));
    assert!(html.contains("inSync"));
    assert!(html.contains("repoDifferent"));
    assert!(html.contains("databaseOnly"));
    assert!(html.contains("inspected"));
    assert!(!html.contains("<th>Source</th>"));
    assert!(!html.contains("<th>Target</th>"));

    for class_name in [
        ".status-repodifferent",
        ".status-insync",
        ".status-repoonly",
        ".status-databaseonly",
        ".status-skipped",
        ".status-error",
        ".status-inspected",
    ] {
        assert!(
            css.contains(class_name),
            "missing status class {class_name}"
        );
    }

    for expected in [
        "identityFromPath",
        "database/objects/schemas/",
        "database/objects/tables/",
        "database/objects/extensions/",
        "database/objects/enums/",
        "database/objects/sequences/",
        "database/objects/indexes/",
        "database/objects/views/",
        "objectType: \"schema\"",
        "objectType: \"table\"",
        "objectType: \"extension\"",
        "objectType: \"enum\"",
        "objectType: \"sequence\"",
        "objectType: \"index\"",
        "objectType: \"view\"",
        "referenceDataRow",
        "rowMatchesFilter",
        "rowMatchesStatusFilter",
        "compareResultRows",
        "status-filter",
        "object-type-filter",
        "data.schemas",
        "data.tables",
        "data.columns",
        "data.extensions",
        "data.enums",
        "data.sequences",
        "data.indexes",
        "data.views",
        "updateCompareOptionLists",
        "updateTableOptions",
        "updateReferenceDataOptions",
        "updateObjectTypeFilterOptions",
        "columnsForSelectedTable",
        "projectStructureGuidance",
        "This workspace is a Git repository but not yet an initialized DbState project",
        "return { scope: \"table\", table: table };",
        "return { scope: \"schema\", schema: schema };",
        "return { scope: \"all\" };",
        "statusBadge(row.status)",
    ] {
        assert!(js.contains(expected), "missing JS mapping text {expected}");
    }

    assert!(!js.contains("objectRef: \"column:"));
    assert!(js.contains("label === \"Inspect\""));
    assert!(js.contains("label === \"Reference-data compare\""));
    assert!(js.contains("appendOption(select, \"referenceData\", \"Reference data\")"));
    assert!(!js.contains("service:response"));
    assert!(!js.contains("objectType: \"service\""));
}

#[test]
fn slice15_service_routes_and_write_confirmation_are_present() {
    let routes = service_route_definitions();
    assert!(routes.contains(&("POST", "/api/v1/postgres/object-ddl")));
    assert!(routes.contains(&("POST", "/api/v1/postgres/repository-sync/preview")));
    assert!(routes.contains(&("POST", "/api/v1/postgres/repository-sync/write")));
    assert!(routes.contains(&("POST", "/api/v1/postgres/release/preview")));
    assert!(routes.contains(&("POST", "/api/v1/postgres/release/write")));
    assert!(routes.contains(&("GET", "/api/v1/workspace/roots")));
    assert!(routes.contains(&("POST", "/api/v1/workspace/list-directories")));
    assert!(routes.contains(&("POST", "/api/v1/workspace/validate")));

    let dir = create_temp_dir("slice15-confirmation");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let missing_confirmation = service_response(
        "POST",
        "/api/v1/postgres/repository-sync/write",
        r#"{ "scope": "all" }"#,
        &dir,
    );

    assert_eq!(missing_confirmation.status_code, 400);
    assert_common_json_contract(&missing_confirmation.body);
    assert!(missing_confirmation.body.contains("WRITE REPOSITORY FILES"));
    assert!(!missing_confirmation.body.contains("postgres://"));
}

#[test]
fn slice15_object_ddl_reads_only_database_objects_under_workspace() {
    let dir = create_temp_dir("slice15-object-ddl");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let schema_path = dir
        .join("database")
        .join("objects")
        .join("schemas")
        .join("core.sql");
    fs::write(&schema_path, "CREATE SCHEMA \"core\";\n").expect("write schema ddl");
    commit_all(&dir, "complete structure");

    let body = r#"{ "scope": "all", "objectType": "schema", "schema": "core", "objectName": "core", "relativePath": "database/objects/schemas/core.sql" }"#;
    let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("\"repositoryDdl\":\"CREATE SCHEMA"));
    assert!(response.body.contains("Database DDL is unavailable"));
    assert!(!response.body.contains("postgres://"));

    for unsafe_path in [
        "database/releases/0001.sql",
        "database/objects/../secrets.sql",
        "/database/objects/schemas/core.sql",
        "database/objects/schemas/core.txt",
    ] {
        let body = format!(
            r#"{{ "objectType": "schema", "schema": "core", "objectName": "core", "relativePath": "{}" }}"#,
            unsafe_path
        );
        let rejected = service_response("POST", "/api/v1/postgres/object-ddl", &body, &dir);
        assert_eq!(rejected.status_code, 400, "{}", rejected.body);
    }
}

#[test]
fn slice16a_object_ddl_returns_object_only_full_context_and_related_objects() {
    let dir = create_temp_dir("slice16a-object-ddl-context");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let table_path = dir
        .join("database")
        .join("objects")
        .join("tables")
        .join("core.accounts.sql");
    fs::write(
        &table_path,
        "CREATE TABLE \"core\".\"accounts\" (\n    \"account_id\" integer NOT NULL\n);\n",
    )
    .expect("write table ddl");
    let index_path = dir
        .join("database")
        .join("objects")
        .join("indexes")
        .join("core.accounts.accounts_code_idx.sql");
    fs::write(
        &index_path,
        "CREATE INDEX \"accounts_code_idx\" ON \"core\".\"accounts\" (\"account_code\");\n",
    )
    .expect("write index ddl");
    commit_all(&dir, "complete structure with table and index");

    let body = r#"{ "scope": "all", "objectType": "table", "schema": "core", "objectName": "accounts", "relativePath": "database/objects/tables/core.accounts.sql" }"#;
    let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("\"objectOnly\""));
    assert!(response.body.contains("\"fullContext\""));
    assert!(response.body.contains("\"relatedObjects\""));
    assert!(response.body.contains("CREATE TABLE"));
    assert!(response.body.contains("accounts_code_idx"));
    assert!(response
        .body
        .contains("database/objects/indexes/core.accounts.accounts_code_idx.sql"));
    assert!(response.body.contains("\"group\":\"Indexes\""));
    assert!(response.body.contains("\"group\":\"Constraints\""));
    assert!(response.body.contains("\"group\":\"Comments\""));
    assert!(response
        .body
        .contains("Object Only DDL is the normalized durable object"));
    assert!(!response.body.contains("postgres://"));
}

#[test]
fn slice15_repository_sync_preview_uses_dry_run_and_does_not_write_without_connection() {
    let dir = create_temp_dir("slice15-preview");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let objects = dir.join("database").join("objects");
    let before = fs::read_dir(&objects).expect("read objects").count();

    let response = service_response(
        "POST",
        "/api/v1/postgres/repository-sync/preview",
        r#"{ "scope": "all" }"#,
        &dir,
    );

    assert_eq!(response.status_code, 400);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("Missing PostgreSQL connection URL"));
    assert_eq!(
        fs::read_dir(&objects).expect("read objects").count(),
        before
    );
}

#[test]
fn slice15_repository_sync_write_rejects_dirty_tree_before_files_are_written() {
    let dir = create_temp_dir("slice15-dirty-write");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("dirty tree");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("working tree has changes")));
}

#[test]
fn slice15_workspace_directory_picker_endpoints_are_safe_and_directory_only() {
    let dir = create_temp_dir("slice15-directory-picker");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::create_dir_all(dir.join("child-a")).expect("create child-a");
    fs::create_dir_all(dir.join("child-b")).expect("create child-b");
    fs::write(dir.join("not-listed.txt"), "not a directory").expect("write file");

    let roots = service_response("GET", "/api/v1/workspace/roots", "", &dir);
    assert_eq!(roots.status_code, 200, "{}", roots.body);
    assert_common_json_contract(&roots.body);
    assert!(roots.body.contains("workspace roots"));
    assert!(roots.body.contains("Service working directory"));

    let list_body = format!(r#"{{ "path": "{}" }}"#, escape_json(&display_path(&dir)));
    let list = service_response(
        "POST",
        "/api/v1/workspace/list-directories",
        &list_body,
        &dir,
    );
    assert_eq!(list.status_code, 200, "{}", list.body);
    assert_common_json_contract(&list.body);
    assert!(list.body.contains("child-a"));
    assert!(list.body.contains("child-b"));
    assert!(!list.body.contains("not-listed.txt"));

    for body in [
        r#"{ "path": "https://example.com/repo.git" }"#.to_string(),
        r#"{ "path": "bad\u0000path" }"#.to_string(),
        format!(
            r#"{{ "path": "{}" }}"#,
            escape_json(&display_path(&dir.join("missing")))
        ),
    ] {
        let rejected = service_response("POST", "/api/v1/workspace/list-directories", &body, &dir);
        assert_eq!(rejected.status_code, 400, "{}", rejected.body);
        assert_common_json_contract(&rejected.body);
    }

    let validate_body = format!(
        r#"{{ "repositoryPath": "{}" }}"#,
        escape_json(&display_path(&dir))
    );
    let validate = service_response("POST", "/api/v1/workspace/validate", &validate_body, &dir);
    assert_eq!(validate.status_code, 200, "{}", validate.body);
    assert_common_json_contract(&validate.body);
    assert!(validate.body.contains("repositoryPath"));
    assert!(validate.body.contains("dbstateProjectStatus"));
}

#[test]
fn slice15_ui_database_to_repository_workflow_contract_is_present() {
    let html = ui_html();
    let js = ui_js();
    let css = ui_css();

    for expected in [
        "Database to Repository",
        "Preview Repository Sync",
        "WRITE REPOSITORY FILES",
        "WRITE REPOSITORY FILES",
        "Copy JSON",
        "Source type",
        "Target type",
        "Source DDL",
        "Target DDL",
        "Full Context DDL",
        "Object Only DDL",
        "Related Objects",
        "Raw Details",
        "data-diff-mode=\"fullContext\"",
        "source-related-objects",
        "target-related-objects",
        "DDL unavailable",
        "repository-sync-controls",
        "source-content",
        "target-content",
        "repository-context",
        "postgres-connection-context",
        "catalog-context",
        "repository-branch",
        "repository-tree",
        "repository-project",
        "repository-dirty",
        "workspace-browse",
        "directory-picker",
        "Select Workspace Folder",
        "Select this folder",
    ] {
        assert!(html.contains(expected), "missing UI text {expected}");
    }
    assert!(html.find("Workflow Mode").unwrap() < html.find("id=\"source-panel\"").unwrap());
    assert!(html.find("Workflow Mode").unwrap() < html.find("id=\"target-panel\"").unwrap());
    assert!(!html.contains("Repository Side"));
    assert!(!html.contains("Database Side"));

    for expected in [
            "/api/v1/postgres/object-ddl",
            "/api/v1/postgres/repository-sync/preview",
            "/api/v1/postgres/repository-sync/write",
            "/api/v1/workspace/roots",
            "/api/v1/workspace/list-directories",
            "/api/v1/workspace/validate",
            "repositorySyncBody",
            "confirmRepositoryWrite",
            "confirmationText",
            "data-standard-operation-action",
            "standardOperationAllowed",
            "Compare is not available in Database to Repository mode. Use Preview Repository Sync.",
            "copyRedactedJson",
            "jsonViewer.textContent",
            "Database to Repository Preview",
            "Database to Repository Write",
            "directionForResultRow",
            "directionForWorkflowMode",
            "rowMatchesWorkflowMode",
            "rowMatchesCurrentWorkflow",
            "isDatabaseToRepositoryRow",
            "workflowLayout",
            "placeSourceTargetContext",
            "sourceContext: \"connection\"",
            "targetContext: \"repository\"",
            "sourceContext: \"repository\"",
            "targetContext: \"connection\"",
            "targetContext: \"catalog\"",
            "Repository configured reference data",
            "Read-only catalog view",
            "DbState captures supported PostgreSQL database state into the selected repository after preview and explicit confirmation.",
            "DbState compares repository desired state to PostgreSQL through read-only service operations.",
            "DbState reads PostgreSQL catalog state through read-only inspection.",
            "DbState compares configured repository reference data to PostgreSQL through read-only service operations.",
            "normalizeDdlForComparison",
            "updateDdlComparisonStatus",
            "ddl-similar",
            "ddl-different",
            "ddl-unavailable",
            "Similar",
            "Different",
            "DDL not available yet for this object.",
            "objectDdlRequest",
            "loadSelectedObjectDdl",
            "objectDdlSection",
            "renderLoadedObjectDiff",
            "relatedObjectsForSide",
            "renderRelatedObjectList",
            "state.objectDiffMode = \"fullContext\"",
            "const label = mode === \"objectOnly\" ? \"Object Only DDL\" : \"Full Context DDL\";",
            "openDirectoryPicker",
            "loadDirectoryRoots",
            "listDirectories",
            "selectDirectoryAsWorkspace",
            "approvedEndpoints.workspaceValidate",
            "friendlyPath",
            "row.producingWorkflowMode = workflowModeForOperation(label);",
            "row.workflowMode = row.producingWorkflowMode;",
            "const direction = directionForResultRow(row);",
            "objectDiffDisplayPayload(row, direction)",
            "clearOperationResults",
            "Workflow mode changed. Run the selected operation again.",
            "operation: row.resultOperation || row.operation || \"review\"",
            "objectDiffDirectionRegressionFixture",
            "staleCompareResultInDatabaseToRepositoryFixture",
            "window.dbstateUiTestHooks",
            "sourceType: \"Repository\"",
            "targetType: \"Database\"",
            "sourceType: displayPayload.sourceType",
            "targetType: displayPayload.targetType",
            "renderedText",
            "\"sourceType \" + displayPayload.sourceType",
            "\"targetType \" + displayPayload.targetType",
            "\"Source type: \" + direction.sourceType",
            "\"Target type: \" + direction.targetType",
            "\"Source DDL \" + sourceDdl",
            "\"Target DDL \" + targetDdl",
        ] {
            assert!(js.contains(expected), "missing JS text {expected}");
        }
    assert!(js.contains(
            "const visibleRows = state.rows.filter(function (row) {\n      return rowMatchesFilter(row) && rowMatchesCurrentWorkflow(row);"
        ));
    assert!(js.contains(
        "Selected result belongs to a different workflow. Run the current workflow again."
    ));
    let direction_index = js.rfind("if (isDatabaseToRepositoryMode(mode))").unwrap();
    let direction_block = &js[direction_index..direction_index + 360];
    assert!(direction_block.contains("sourceType: \"Database\""));
    assert!(direction_block.contains("targetType: \"Repository\""));
    assert!(direction_block.contains("sourceDdlSide: \"database\""));
    assert!(direction_block.contains("targetDdlSide: \"repository\""));
    assert!(js.contains(
        "const ddlBySide = { repository: repositoryDdl, database: databaseDdl, \"\": \"\" };"
    ));
    assert!(js.contains("const sourceDdl = ddlBySide[direction.sourceDdlSide] || \"\";"));
    assert!(js.contains("const targetDdl = ddlBySide[direction.targetDdlSide] || \"\";"));
    assert!(js.contains("byId(\"selected-json\").textContent = redactedJson(objectDiffDisplayPayload(row, direction));"));
    assert!(!js.contains("row.sourceType"));
    assert!(!js.contains("row.targetType"));
    let fixture_index = js
        .find("function objectDiffDirectionRegressionFixture()")
        .unwrap();
    let fixture_end = js[fixture_index..]
        .find("if (typeof window !== \"undefined\")")
        .map(|offset| fixture_index + offset)
        .unwrap();
    let fixture_block = &js[fixture_index..fixture_end];
    assert!(fixture_block.contains("producingWorkflowMode: \"databaseToRepository\""));
    assert!(fixture_block.contains("resultOperation: \"Database to Repository Preview\""));
    assert!(fixture_block.contains("sourceType: \"Repository\""));
    assert!(fixture_block.contains("targetType: \"Database\""));
    assert!(fixture_block.contains("sourceType: displayPayload.sourceType"));
    assert!(fixture_block.contains("targetType: displayPayload.targetType"));
    assert!(fixture_block.contains("const sourceDdl = ddlBySide[direction.sourceDdlSide]"));
    assert!(fixture_block.contains("const targetDdl = ddlBySide[direction.targetDdlSide]"));
    assert!(fixture_block.contains("sourceDdl: sourceDdl"));
    assert!(fixture_block.contains("targetDdl: targetDdl"));
    assert!(fixture_block.contains("\"sourceType \" + displayPayload.sourceType"));
    assert!(fixture_block.contains("\"targetType \" + displayPayload.targetType"));
    assert!(fixture_block.contains("\"Source type: \" + direction.sourceType"));
    assert!(fixture_block.contains("\"Target type: \" + direction.targetType"));
    assert!(fixture_block.contains("\"Source DDL \" + sourceDdl"));
    assert!(fixture_block.contains("\"Target DDL \" + targetDdl"));
    assert!(!fixture_block.contains("\"sourceType \" + staleRow.sourceType"));
    assert!(!fixture_block.contains("\"targetType \" + staleRow.targetType"));
    let stale_fixture_index = js
        .find("function staleCompareResultInDatabaseToRepositoryFixture()")
        .unwrap();
    let stale_fixture_end = js[stale_fixture_index..]
        .find("if (typeof window !== \"undefined\")")
        .map(|offset| stale_fixture_index + offset)
        .unwrap();
    let stale_fixture_block = &js[stale_fixture_index..stale_fixture_end];
    assert!(stale_fixture_block.contains("producingWorkflowMode: \"compare\""));
    assert!(stale_fixture_block.contains("operation: \"Compare\""));
    assert!(stale_fixture_block.contains("sourceType: \"Repository\""));
    assert!(stale_fixture_block.contains("targetType: \"Database\""));
    assert!(stale_fixture_block.contains("const selectedMode = \"databaseToRepository\""));
    assert!(stale_fixture_block
        .contains("const canRender = rowMatchesWorkflowMode(staleCompareRow, selectedMode);"));
    assert!(stale_fixture_block.contains("canRender: canRender"));
    assert!(stale_fixture_block.contains("visibleRows: canRender ? 1 : 0"));
    assert!(stale_fixture_block.contains(
        "Selected result belongs to a different workflow. Run the current workflow again."
    ));
    for expected in [
        "addedFiles",
        "changedFiles",
        "unchangedFiles",
        "plannedCreates",
        "plannedUpdates",
        "createdFiles",
        "updatedFiles",
    ] {
        assert!(js.contains(expected), "missing sync mapping {expected}");
    }
    assert!(css.contains(".status-plannedcreate"));
    assert!(css.contains(".status-updated"));

    for forbidden in [
        "Deploy to database",
        "Execute SQL",
        "Execute generated SQL",
        "Sync to Database",
        "Apply to Target",
        "Push to Target Database",
        "localStorage",
        "sessionStorage",
        "showDirectoryPicker",
    ] {
        assert!(
            !html.contains(forbidden) && !js.contains(forbidden),
            "UI contains forbidden pattern {forbidden}"
        );
    }
}

#[test]
fn slice15_workspace_path_display_normalizes_windows_verbatim_prefixes() {
    let verbatim = Path::new(r"\\?\D:\DbState\ExitPassDb");
    assert_eq!(display_path(verbatim), "D:/DbState/ExitPassDb");
    assert_eq!(
        normalize_local_path_input("///?/D:/DbState/ExitPassDb"),
        "D:/DbState/ExitPassDb"
    );

    let js = ui_js();
    assert!(js.contains("text.replace(/^\\/{2,3}\\?\\//, \"\")"));
    assert!(js.contains("byId(\"workspace-path\").value = gitRoot"));
    assert!(!js.contains("localStorage"));
    assert!(!js.contains("sessionStorage"));
    assert!(!js.contains("showDirectoryPicker"));
}

#[test]
fn slice14_missing_profile_file_returns_empty_list_and_saves_outside_repo() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let repo = create_temp_dir("slice14-repo");
    let config = create_temp_dir("slice14-config");
    env::set_var("DBSTATE_CONFIG_DIR", &config);

    let store = load_connection_profiles().expect("load profiles");
    assert!(store.profiles.is_empty());
    assert!(!config.join(PROFILE_FILE_NAME).exists());

    let store = ConnectionProfileStore {
        profiles: vec![valid_connection_profile("exitpass-local")],
    };
    save_connection_profiles(&store).expect("save profiles");

    let profile_file = config.join(PROFILE_FILE_NAME);
    assert!(profile_file.exists());
    assert!(!profile_file.starts_with(&repo));
    let content = fs::read_to_string(profile_file).expect("read profile file");
    assert!(!content.contains("password"));
    assert!(!content.contains("token"));
    assert!(!content.contains("postgres://"));
    env::remove_var("DBSTATE_CONFIG_DIR");
}

#[test]
fn slice14_profile_validation_rejects_duplicate_invalid_and_secret_values() {
    let duplicate = ConnectionProfileStore {
        profiles: vec![
            valid_connection_profile("local"),
            valid_connection_profile("LOCAL"),
        ],
    };
    assert!(validate_unique_profile_names(&duplicate.profiles).is_err());

    let mut invalid_port = valid_connection_profile("bad-port");
    invalid_port.port = 0;
    assert!(validate_connection_profile(&invalid_port).is_err());

    let mut url_host = valid_connection_profile("url-host");
    url_host.host = "postgres://localhost".to_string();
    assert!(validate_connection_profile(&url_host).is_err());

    let secret_json = r#"{ "name": "bad", "host": "localhost", "port": 5432, "database": "db", "username": "postgres", "sslMode": "disable", "password": "secret" }"#;
    let value: Value = serde_yaml::from_str(secret_json).expect("parse json");
    let error = parse_connection_profile(&value).expect_err("secret field rejected");
    assert!(!error.contains("password"));
    assert!(!error.contains("secret\""));
    assert!(error.contains("<redacted-field>"));
}

#[test]
fn slice14_invalid_profile_json_returns_clear_error() {
    let path = temp_path("slice14-invalid-json").with_extension("json");
    fs::write(&path, "{ invalid json").expect("write invalid json");

    let error = load_connection_profiles_from_path(&path).expect_err("invalid json");

    assert!(error.contains("invalid JSON"));
}

#[test]
fn slice14_connection_resolution_precedence_and_profile_session_password() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let config = create_temp_dir("slice14-resolution-config");
    env::set_var("DBSTATE_CONFIG_DIR", &config);
    env::set_var(
        "DBSTATE_POSTGRES_URL",
        "postgres://env_user:env_secret@example.invalid/env_db",
    );
    save_connection_profiles(&ConnectionProfileStore {
        profiles: vec![valid_connection_profile("local")],
    })
    .expect("save profiles");

    let request_url: Value = serde_yaml::from_str(
            r#"{ "postgresUrl": "postgres://request_user:request_secret@example.invalid/request_db", "connection": { "profileName": "local", "password": "profile_secret" } }"#,
        )
        .expect("parse request");
    let resolved = resolve_service_postgres_connection(&request_url)
        .expect("resolve")
        .expect("connection");
    assert_eq!(resolved.source, "requestUrl");
    assert!(resolved.url.contains("request_user"));

    let profile_request: Value = serde_yaml::from_str(
        r#"{ "connection": { "profileName": "local", "password": "profile secret" } }"#,
    )
    .expect("parse request");
    let resolved = resolve_service_postgres_connection(&profile_request)
        .expect("resolve")
        .expect("connection");
    assert_eq!(resolved.source, "profile");
    assert!(resolved.url.starts_with("postgres://postgres:"));
    assert!(resolved.url.contains("profile%20secret"));
    assert!(!resolved.url.contains("profile secret"));

    let env_request: Value = serde_yaml::from_str("{}").expect("parse request");
    let resolved = resolve_service_postgres_connection(&env_request)
        .expect("resolve")
        .expect("connection");
    assert_eq!(resolved.source, "environment");
    assert!(resolved.url.contains("env_user"));

    env::remove_var("DBSTATE_CONFIG_DIR");
    env::remove_var("DBSTATE_POSTGRES_URL");
}

#[test]
fn slice14_service_profile_endpoints_return_json_and_never_store_secrets() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let config = create_temp_dir("slice14-service-config");
    let cwd = create_temp_dir("slice14-service-cwd");
    env::set_var("DBSTATE_CONFIG_DIR", &config);

    let empty = service_response("GET", "/api/v1/connections/profiles", "", &cwd);
    assert_eq!(empty.status_code, 200);
    assert!(empty.body.contains("\"profiles\":[]"));

    let create = service_response(
        "POST",
        "/api/v1/connections/profiles",
        r#"{ "name": "exitpass-local", "host": "localhost", "port": 5433, "database": "exitpass_v12_dev", "username": "postgres", "sslMode": "disable", "description": "local dev" }"#,
        &cwd,
    );
    assert_eq!(create.status_code, 200, "{}", create.body);
    assert!(create.body.contains("exitpass-local"));
    assert!(!create.body.contains("password"));
    assert!(!create.body.contains("postgres://"));

    let update = service_response(
        "PUT",
        "/api/v1/connections/profiles/exitpass-local",
        r#"{ "name": "exitpass-local", "host": "localhost", "port": 5434, "database": "exitpass_v12_dev", "username": "postgres", "sslMode": "prefer" }"#,
        &cwd,
    );
    assert_eq!(update.status_code, 200, "{}", update.body);
    assert!(update.body.contains("\"port\":5434"));

    let stored = fs::read_to_string(config.join(PROFILE_FILE_NAME)).expect("read profiles");
    assert!(!stored.contains("password"));
    assert!(!stored.contains("postgres://"));

    let delete = service_response(
        "DELETE",
        "/api/v1/connections/profiles/exitpass-local",
        "",
        &cwd,
    );
    assert_eq!(delete.status_code, 200, "{}", delete.body);
    assert!(delete.body.contains("\"profiles\":[]"));

    env::remove_var("DBSTATE_CONFIG_DIR");
}

#[test]
fn slice14_connection_test_redacts_missing_and_bad_connection_values() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let cwd = create_temp_dir("slice14-test-connection");
    env::remove_var("DBSTATE_POSTGRES_URL");

    let missing = service_response("POST", "/api/v1/connections/test", "{}", &cwd);
    assert_eq!(missing.status_code, 400);
    assert!(missing.body.contains("Missing PostgreSQL connection"));

    let bad = service_response(
        "POST",
        "/api/v1/connections/test",
        r#"{ "postgresUrl": "not-a-postgres-url-with-secret" }"#,
        &cwd,
    );
    assert_eq!(bad.status_code, 503);
    assert!(bad.body.contains("Invalid PostgreSQL connection URL"));
    assert!(!bad.body.contains("not-a-postgres-url-with-secret"));
    assert!(!bad.body.contains("password"));
}

#[test]
fn slice14_ui_connection_profile_contract_is_present() {
    let html = ui_html();
    let js = ui_js();

    for expected in [
        "connection-mode",
        "Use session URL",
        "Use saved profile",
        "Use service environment variable",
        "profile-select",
        "profile-password",
        "profile-host",
        "profile-database",
        "profile-username",
        "profile-sslmode",
        "Save Profile",
        "Delete Profile",
        "Test Connection",
        "Passwords, tokens, and full URLs are never saved.",
    ] {
        assert!(
            html.contains(expected),
            "missing UI profile text {expected}"
        );
    }
    for forbidden in ["Save Password", "Remember Password", "save password"] {
        assert!(!html.contains(forbidden));
    }
    for expected in [
        "/api/v1/connections/profiles",
        "/api/v1/connections/test",
        "attachConnection",
        "profileName",
        "connection.password",
        "selectedConnectionMode",
        "connectionString",
        "connectionUrl",
        "postgresUrl",
    ] {
        assert!(js.contains(expected), "missing UI JS text {expected}");
    }
    assert!(!js.contains("localStorage"));
    assert!(!js.contains("sessionStorage"));
    assert!(!js.contains("deploy to database"));
    assert!(!js.contains("execute SQL"));
    assert!(!js.contains("sync to database"));
}

fn valid_connection_profile(name: &str) -> ConnectionProfile {
    ConnectionProfile {
        name: name.to_string(),
        host: "localhost".to_string(),
        port: 5432,
        database: "exitpass_v12_dev".to_string(),
        username: "postgres".to_string(),
        ssl_mode: "disable".to_string(),
        description: Some("Local development database".to_string()),
        default_schema: None,
    }
}

#[test]
fn slice13_valid_repository_path_is_accepted_by_service() {
    let fallback = create_temp_dir("slice13-fallback");
    let selected = create_temp_dir("slice13-selected");
    init_git_repo(&selected);
    create_complete_structure(&selected);
    commit_all(&selected, "complete structure");
    let body = format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&selected));

    let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);

    assert_eq!(response.status_code, 200);
    assert_common_json_contract(&response.body);
    assert_repository_json_contract(&response.body);
    assert!(response.body.contains(&escape_json(&display_path(
        &selected.canonicalize().unwrap()
    ))));
    assert!(response
        .body
        .contains("\"dbstateProjectStatus\":\"completeDbStateStructure\""));
}

#[test]
fn slice13_missing_or_empty_repository_path_falls_back_to_service_cwd() {
    let fallback = create_temp_dir("slice13-service-cwd");
    init_git_repo(&fallback);
    create_complete_structure(&fallback);
    commit_all(&fallback, "complete structure");

    let omitted = service_response("POST", "/api/v1/repo/status", "{}", &fallback);
    let empty = service_response(
        "POST",
        "/api/v1/repo/status",
        r#"{ "repositoryPath": "   " }"#,
        &fallback,
    );

    assert_eq!(omitted.status_code, 200);
    assert_eq!(empty.status_code, 200);
    assert!(omitted
        .body
        .contains(&escape_json(&display_path(&fallback))));
    assert!(empty.body.contains(&escape_json(&display_path(&fallback))));
}

#[test]
fn slice13_invalid_repository_paths_are_rejected() {
    let fallback = create_temp_dir("slice13-invalid-paths");
    init_git_repo(&fallback);
    let file_path = fallback.join("not-a-directory.txt");
    fs::write(&file_path, "not a directory").expect("write test file");
    let nonexistent = fallback.join("missing");

    for (body, expected) in [
        (
            format!(
                r#"{{ "repositoryPath": "{}" }}"#,
                display_path(&nonexistent)
            ),
            "does not exist",
        ),
        (
            format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&file_path)),
            "must point to a directory",
        ),
        (
            r#"{ "repositoryPath": "https://example.com/repo.git" }"#.to_string(),
            "local filesystem path",
        ),
        (
            r#"{ "repositoryPath": "git@example.com:repo.git" }"#.to_string(),
            "local filesystem path",
        ),
    ] {
        let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);
        assert_eq!(response.status_code, 400);
        assert!(
            response.body.contains(expected),
            "response was {}",
            response.body
        );
    }
}

#[test]
fn slice13_non_git_directory_is_rejected_as_workspace() {
    let fallback = create_temp_dir("slice13-fallback-git");
    init_git_repo(&fallback);
    let non_git = create_temp_dir("slice13-non-git");
    let body = format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&non_git));

    let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);

    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("Git working tree"));
}

#[test]
fn slice13_git_without_dbstate_structure_returns_useful_status() {
    let fallback = create_temp_dir("slice13-fallback-status");
    init_git_repo(&fallback);
    let selected = create_temp_dir("slice13-git-no-structure");
    init_git_repo(&selected);
    let body = format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&selected));

    let response = service_response("POST", "/api/v1/repo/status", &body, &fallback);

    assert_eq!(response.status_code, 200);
    assert!(response
        .body
        .contains("\"dbstateProjectStatus\":\"gitRepositoryWithoutDbStateStructure\""));
    assert!(response.body.contains("\"missingPaths\""));
}

#[test]
fn slice13_project_operations_use_selected_repository_path() {
    let fallback = create_temp_dir("slice13-fallback-operation");
    init_git_repo(&fallback);
    create_complete_structure(&fallback);
    commit_all(&fallback, "fallback structure");

    let selected = create_temp_dir("slice13-selected-operation");
    init_git_repo(&selected);
    create_complete_structure(&selected);
    commit_all(&selected, "selected structure");
    let body = format!(
        r#"{{ "repositoryPath": "{}", "dryRun": true }}"#,
        display_path(&selected)
    );

    let response = service_response("POST", "/api/v1/init/plan", &body, &fallback);

    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(&escape_json(&display_path(
        &selected.canonicalize().unwrap()
    ))));
    assert!(!response
        .body
        .contains(&escape_json(&display_path(&fallback))));
}

#[test]
fn init_write_service_endpoint_requires_confirmation_and_git_repository() {
    let dir = create_temp_dir("init-write-confirmation");
    init_git_repo(&dir);

    let missing = service_response("POST", "/api/v1/init/write", "{}", &dir);
    assert_eq!(missing.status_code, 400);
    assert!(missing.body.contains("confirmationText"));
    assert!(!dir.join("database").exists());

    let wrong = service_response(
        "POST",
        "/api/v1/init/write",
        r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE" }"#,
        &dir,
    );
    assert_eq!(wrong.status_code, 400);
    assert!(!dir.join("database").exists());

    let non_git = create_temp_dir("init-write-non-git");
    let body =
        r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;
    let rejected = service_response("POST", "/api/v1/init/write", body, &non_git);
    assert_eq!(rejected.status_code, 400);
    assert!(rejected.body.contains("\"success\":false"));
    assert!(!non_git.join("database").exists());
}

#[test]
fn init_write_service_endpoint_creates_only_project_structure_without_staging() {
    let dir = create_temp_dir("init-write-success");
    init_git_repo(&dir);
    let body =
        r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;

    let response = service_response("POST", "/api/v1/init/write", body, &dir);

    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("\"success\":true"));
    assert!(response
        .body
        .contains("\"dbstateProjectStatus\":\"completeDbStateStructure\""));
    assert!(response.body.contains("\"plannedCreates\""));
    assert!(response.body.contains("\"createdPaths\""));
    for expected in EXPECTED_PATHS {
        assert!(
            dir.join(expected.relative).exists(),
            "missing {}",
            expected.relative
        );
    }
    assert!(!dir.join("database/releases/0001_unexpected.sql").exists());
    let staged = Command::new("git")
        .arg("diff")
        .arg("--cached")
        .arg("--name-only")
        .current_dir(&dir)
        .output()
        .expect("run git diff cached");
    assert!(staged.status.success(), "git diff cached failed");
    assert!(
        String::from_utf8_lossy(&staged.stdout).trim().is_empty(),
        "init write staged files"
    );
}

#[test]
fn init_write_service_endpoint_does_not_overwrite_existing_registry() {
    let dir = create_temp_dir("init-write-no-overwrite");
    init_git_repo(&dir);
    let registry = dir.join("database/reference-data/dbstate.reference-data.yml");
    fs::create_dir_all(registry.parent().expect("registry parent"))
        .expect("create registry parent");
    fs::write(&registry, "version: 1\ntables:\n  - keep_me\n").expect("write custom registry");
    commit_all(&dir, "existing registry");
    let body =
        r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;

    let response = service_response("POST", "/api/v1/init/write", body, &dir);

    assert_eq!(response.status_code, 200);
    assert_eq!(
        fs::read_to_string(&registry).expect("read registry"),
        "version: 1\ntables:\n  - keep_me\n"
    );
}

#[test]
fn slice13_ui_javascript_includes_repository_path_without_persistence() {
    let js = ui_js();

    assert!(js.contains("repositoryPath"));
    assert!(js.contains("workspace-path"));
    assert!(js.contains("attachWorkspacePath"));
    assert!(!js.contains("localStorage"));
    assert!(!js.contains("sessionStorage"));
    assert!(!js.contains("clone"));
    assert!(!js.contains("git fetch"));
    assert!(!js.contains("git pull"));
    assert!(!js.contains("git push"));
    assert!(!js.contains("git add"));
    assert!(!js.contains("git commit"));
}

#[test]
fn slice12_service_api_routes_remain_json() {
    let dir = create_temp_dir("slice12-api-json");
    let health = service_response("GET", "/api/v1/health", "", &dir);
    assert_eq!(health.status_code, 200);
    assert!(health.content_type.contains("application/json"));
    assert_common_json_contract(&health.body);

    let missing = service_response("GET", "/api/v1/missing", "", &dir);
    assert_eq!(missing.status_code, 404);
    assert!(missing.content_type.contains("application/json"));
    assert_common_json_contract(&missing.body);
}

#[test]
fn slice11_service_health_and_repo_status_return_json_contract() {
    let dir = create_temp_dir("slice11-service-status");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let health = service_response("GET", "/health", "", &dir);
    assert_eq!(health.status_code, 200);
    assert_common_json_contract(&health.body);
    assert!(health.body.contains("\"service\":\"dbstate\""));

    let status = service_response("POST", "/api/v1/repo/status", "{}", &dir);
    assert_eq!(status.status_code, 200);
    assert_common_json_contract(&status.body);
    assert_repository_json_contract(&status.body);
    assert!(status.body.contains("\"command\":\"repo status\""));
}

#[test]
fn slice11_service_init_plan_is_dry_run_only() {
    let dir = create_temp_dir("slice11-init-plan");
    init_git_repo(&dir);

    let plan = service_response("POST", "/api/v1/init/plan", r#"{ "dryRun": true }"#, &dir);
    assert_eq!(plan.status_code, 200);
    assert!(plan.body.contains("\"command\":\"init\""));
    assert!(plan.body.contains("\"plannedCreates\""));
    assert!(!dir.join("database").exists());

    let write = service_response("POST", "/api/v1/init/plan", r#"{ "dryRun": false }"#, &dir);
    assert_eq!(write.status_code, 400);
    assert!(write.body.contains("init planning only"));
}

#[test]
fn slice11_service_rejects_invalid_scope_and_write_requests() {
    let dir = create_temp_dir("slice11-invalid-requests");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let invalid_scope = service_response(
        "POST",
        "/api/v1/postgres/compare",
        r#"{ "scope": "everything" }"#,
        &dir,
    );
    assert_eq!(invalid_scope.status_code, 400);
    assert!(invalid_scope.body.contains("Invalid scope"));

    let write_request = service_response(
        "POST",
        "/api/v1/postgres/plan",
        r#"{ "scope": "all", "apply": true }"#,
        &dir,
    );
    assert_eq!(write_request.status_code, 400);
    assert!(write_request.body.contains("read-only or plan-only"));

    let remote_workspace = service_response(
        "POST",
        "/api/v1/repo/status",
        r#"{ "repositoryPath": "https://example.com/repo.git" }"#,
        &dir,
    );
    assert_eq!(remote_workspace.status_code, 400);
    assert!(remote_workspace.body.contains("local filesystem path"));
}

#[test]
fn slice11_service_redacts_postgres_url_and_credentials() {
    let dir = create_temp_dir("slice11-redaction");
    let raw_url = placeholder_url("service-user", "service-secret-marker");
    let body = format!(r#"{{ "postgresUrl": "{raw_url}", "scope": "all" }}"#);

    let response = service_response("POST", "/api/v1/postgres/inspect", &body, &dir);

    assert_eq!(response.status_code, 503);
    assert_common_json_contract(&response.body);
    assert!(!response.body.contains(&raw_url));
    assert!(!response.body.contains("service-secret-marker"));
    assert!(!response.body.contains("postgres://"));
}

#[test]
fn slice11_service_rejects_unknown_route_and_invalid_json() {
    let dir = create_temp_dir("slice11-routing");

    let unknown = service_response("POST", "/api/v1/postgres/apply", "{}", &dir);
    assert_eq!(unknown.status_code, 404);
    assert!(!unknown.body.contains("direct apply"));

    let invalid_json = service_response("POST", "/api/v1/repo/status", "{ invalid json", &dir);
    assert_eq!(invalid_json.status_code, 400);
    assert!(invalid_json.body.contains("Invalid JSON request body"));
}

#[test]
fn slice9_malformed_postgres_urls_are_rejected_without_leaking_values() {
    let invalid_url = "not-a-postgres-url-with-sensitive-marker";
    let export_args = ParsedArgs::parse(&[
        "export".to_string(),
        "postgres".to_string(),
        "--all".to_string(),
        "--url".to_string(),
        invalid_url.to_string(),
    ])
    .expect("parse export");
    let export_report = export_postgres_command(Path::new("."), export_args);
    assert!(!export_report.success);
    assert!(export_report
        .errors
        .iter()
        .any(|error| error.contains("Invalid PostgreSQL connection URL")));
    assert!(!export_report.to_json().contains(invalid_url));

    let inspect_report = inspect_postgres_command(Some(invalid_url.to_string()), None);
    assert!(!inspect_report.success);
    assert!(inspect_report
        .errors
        .iter()
        .any(|error| error.contains("Invalid PostgreSQL connection URL")));
    assert!(!inspect_report.to_json().contains(invalid_url));
}

#[test]
fn slice9_exit_code_contract_for_key_paths_is_stable() {
    let dir = create_temp_dir("slice9-exit-codes");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let status_args = vec![
        "repo".to_string(),
        "status".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let status = run_cli(&status_args, Ok(dir.as_path())).expect("status cli");
    assert_eq!(status.exit_code, 0);

    let invalid_args = vec!["inspect".to_string(), "mysql".to_string()];
    assert!(ParsedArgs::parse(&invalid_args).is_err());

    let compare_report =
        compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);
    assert!(compare_report.success);
    assert!(!compare_report.database_only.is_empty());

    let (config, state) = reference_config_and_state();
    let database_rows = vec![reference_row(&[
        ("code", Some("CASH")),
        ("name", Some("Cash Live Difference")),
    ])];
    let data_result = compare_reference_data_table(
        &config,
        &state,
        &database_rows,
        &reference_database_columns(),
    );
    assert_eq!(data_result.row_counts.repo_different, 1);

    let blocked_plan = plan_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
    );
    assert!(blocked_plan.success);
    assert!(!blocked_plan.blocked_items.is_empty());

    let blocked_release = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
        "slice9_blocked",
        true,
    );
    assert!(!blocked_release.success);
    assert!(!blocked_release.blocked_items.is_empty());
}

#[test]
fn cli_rejects_apply_and_database_mutation_commands() {
    let invalid_commands = [
        vec!["apply".to_string()],
        vec!["execute".to_string()],
        vec!["compare".to_string()],
        vec!["inspect".to_string(), "mysql".to_string()],
    ];

    for args in invalid_commands {
        assert!(ParsedArgs::parse(&args).is_err());
    }

    assert!(ParsedArgs::parse(&["inspect".to_string(), "postgres".to_string()]).is_ok());
    assert!(ParsedArgs::parse(&[
        "export".to_string(),
        "postgres".to_string(),
        "--all".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "sync".to_string(),
        "postgres".to_string(),
        "--all".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "compare".to_string(),
        "postgres".to_string(),
        "--all".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "plan".to_string(),
        "postgres".to_string(),
        "--all".to_string(),
        "--include".to_string(),
        "table:dbstate_slice2.sample_accounts".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "release".to_string(),
        "postgres".to_string(),
        "--all".to_string(),
        "--name".to_string(),
        "slice7".to_string(),
        "--dry-run".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "data-compare".to_string(),
        "postgres".to_string(),
        "--all".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "data-compare".to_string(),
        "postgres".to_string(),
        "--table".to_string(),
        "dbstate_ref.payment_methods".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "compare".to_string(),
        "postgres".to_string(),
        "--all".to_string(),
        "--include".to_string(),
        "schema:dbstate_slice2".to_string()
    ])
    .is_err());
}
