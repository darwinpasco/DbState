use super::*;
use crate::ci::{
    ci_apply_function_retry_for_test, ci_apply_order_for_test, ci_reference_data_summary_for_test,
    ci_release_summary_for_test, ci_report_with_compare_back_details_for_test,
    ci_skips_bootstrap_public_schema_for_test, format_ci_sql_execution_message,
    validate_disposable_database, CiSqlExecutionError,
};
use crate::postgres::{
    inspect::empty_inspection_report, render_rls_policy_sql, resolve_postgres_url, RlsPolicyInfo,
};
use crate::project::{PathKind, DEFAULT_REGISTRY, EXPECTED_PATHS};
use crate::reference_data::{
    append_reference_table_result, empty_reference_data_compare_report,
    parse_reference_data_registry, parse_reference_data_table_state, reference_row_key,
    ReferenceDataCandidateTable, ReferenceDataForeignKeyReference,
};
use crate::reference_data::{
    data_compare_postgres_command, reference_data_review_script_preview_from_compare,
    reference_data_review_script_write_from_compare,
};
use crate::release::empty_release_report;
use crate::release::release_postgres_command;
use crate::repository::discovery::discover_repository_objects;
use crate::repository::objects::{
    aggregate_file_path, constraint_file_path, domain_file_path, function_file_path,
    function_identity_slug, grant_file_path, materialized_view_file_path, rls_policy_file_path,
    trigger_file_path, DesiredStateObject, RepositoryObjectType,
};
use crate::repository::{
    compare_postgres_command, ensure_database_object_path, enum_file_path, export_postgres_command,
    extension_file_path, index_file_path, plan_postgres_command, schema_file_path,
    sequence_file_path, sync_postgres_with_inventory_for_paths, table_file_path, view_file_path,
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
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

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
    set_git_head_branch(path, "feature/dbstate-test");
}

fn set_git_head_branch(path: &Path, branch: &str) {
    fs::write(
        path.join(".git/HEAD"),
        format!("ref: refs/heads/{branch}\n"),
    )
    .expect("set test git branch");
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

fn stage_path(path: &Path, relative: &str) {
    let add = Command::new("git")
        .arg("add")
        .arg("--")
        .arg(relative)
        .current_dir(path)
        .output()
        .expect("run git add path");
    assert!(
        add.status.success(),
        "git add {relative} failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );
}

fn run_commit_message_text(path: &Path, args: &[&str]) -> (u8, String) {
    let args = args
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    let result = crate::commit_message::run(&args, path).expect("run commit-message");
    (result.exit_code, result.output)
}

fn run_cli_output(path: &Path, args: &[&str]) -> (u8, String) {
    let args = args
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    let result = run_cli(&args, Ok(path)).expect("run cli");
    let output = match result.format {
        OutputFormat::Text => result.output.to_text(),
        OutputFormat::Json => result.output.to_json(),
    };
    (result.exit_code, output)
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
                partition_key: None,
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
            domains: vec![DomainInfo {
                schema_name: "dbstate_slice2".to_string(),
                domain_name: "account_year".to_string(),
                base_type: "integer".to_string(),
                is_not_null: false,
                default_expression: None,
                check_constraints: vec![DomainCheckInfo {
                    constraint_name: "account_year_check".to_string(),
                    definition: "CHECK ((VALUE >= 1901) AND (VALUE <= 2155))".to_string(),
                }],
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
            materialized_views: vec![MaterializedViewInfo {
                schema_name: "dbstate_slice2".to_string(),
                materialized_view_name: "account_summary".to_string(),
                definition:
                    " SELECT sample_accounts.account_id,\n    sample_accounts.account_code\n   FROM dbstate_slice2.sample_accounts"
                        .to_string(),
                is_populated: Some(false),
                tablespace: None,
            }],
            constraints: vec![
                ConstraintInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    constraint_name: "sample_accounts_pkey".to_string(),
                    constraint_type: "primaryKey".to_string(),
                    definition: "PRIMARY KEY (account_id)".to_string(),
                    columns: vec!["account_id".to_string()],
                    referenced_schema: None,
                    referenced_table: None,
                    referenced_columns: Vec::new(),
                },
                ConstraintInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    constraint_name: "sample_accounts_code_key".to_string(),
                    constraint_type: "uniqueConstraint".to_string(),
                    definition: "UNIQUE (account_code)".to_string(),
                    columns: vec!["account_code".to_string()],
                    referenced_schema: None,
                    referenced_table: None,
                    referenced_columns: Vec::new(),
                },
                ConstraintInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    constraint_name: "sample_accounts_parent_fkey".to_string(),
                    constraint_type: "foreignKey".to_string(),
                    definition:
                        "FOREIGN KEY (account_id) REFERENCES dbstate_slice2.sample_accounts(account_id) ON UPDATE CASCADE ON DELETE RESTRICT"
                            .to_string(),
                    columns: vec!["account_id".to_string()],
                    referenced_schema: Some("dbstate_slice2".to_string()),
                    referenced_table: Some("sample_accounts".to_string()),
                    referenced_columns: vec!["account_id".to_string()],
                },
                ConstraintInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    constraint_name: "sample_accounts_code_check".to_string(),
                    constraint_type: "checkConstraint".to_string(),
                    definition: "CHECK ((account_code <> ''::text))".to_string(),
                    columns: vec!["account_code".to_string()],
                    referenced_schema: None,
                    referenced_table: None,
                    referenced_columns: Vec::new(),
                },
            ],
            functions: vec![
                FunctionInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    function_name: "account_label".to_string(),
                    identity_arguments: "account_id integer".to_string(),
                    result_type: Some("text".to_string()),
                    language: Some("sql".to_string()),
                    volatility: Some("stable".to_string()),
                    security_definer: false,
                    is_strict: false,
                    definition:
                        "CREATE FUNCTION dbstate_slice2.account_label(account_id integer)\n RETURNS text\n LANGUAGE sql\n STABLE\nAS $function$\n    SELECT 'account-' || account_id::text;\n$function$"
                            .to_string(),
                },
                FunctionInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    function_name: "account_label".to_string(),
                    identity_arguments: "account_code text".to_string(),
                    result_type: Some("text".to_string()),
                    language: Some("sql".to_string()),
                    volatility: Some("stable".to_string()),
                    security_definer: false,
                    is_strict: true,
                    definition:
                        "CREATE FUNCTION dbstate_slice2.account_label(account_code text)\n RETURNS text\n LANGUAGE sql\n STABLE\n STRICT\nAS $function$\n    SELECT upper(account_code);\n$function$"
                            .to_string(),
                },
            ],
            aggregates: vec![AggregateInfo {
                schema_name: "dbstate_slice2".to_string(),
                aggregate_name: "account_labels".to_string(),
                identity_arguments: "text".to_string(),
                transition_function_schema: "dbstate_slice2".to_string(),
                transition_function_name: "account_label_concat".to_string(),
                state_type: "text".to_string(),
                final_function_schema: None,
                final_function_name: None,
                initial_condition: None,
                sort_operator: None,
            }],
            triggers: vec![TriggerInfo {
                schema_name: "dbstate_slice2".to_string(),
                relation_name: "sample_accounts".to_string(),
                trigger_name: "sample_accounts_audit_trigger".to_string(),
                trigger_function_schema: Some("dbstate_slice2".to_string()),
                trigger_function_name: Some("account_label".to_string()),
                timing: Some("AFTER".to_string()),
                events: vec!["INSERT".to_string()],
                orientation: Some("ROW".to_string()),
                definition:
                    "CREATE TRIGGER sample_accounts_audit_trigger AFTER INSERT ON dbstate_slice2.sample_accounts FOR EACH ROW EXECUTE FUNCTION dbstate_slice2.account_label(account_id)"
                        .to_string(),
            }],
            grants: vec![
                GrantInfo {
                    target_kind: "schema".to_string(),
                    schema_name: "dbstate_slice2".to_string(),
                    object_name: None,
                    identity_arguments: None,
                    grantee: "app_reader".to_string(),
                    grantor: Some("dbstate_owner".to_string()),
                    privileges: vec!["USAGE".to_string()],
                    grantable_privileges: Vec::new(),
                    with_grant_option: false,
                },
                GrantInfo {
                    target_kind: "table".to_string(),
                    schema_name: "dbstate_slice2".to_string(),
                    object_name: Some("sample_accounts".to_string()),
                    identity_arguments: None,
                    grantee: "PUBLIC".to_string(),
                    grantor: Some("dbstate_owner".to_string()),
                    privileges: vec!["SELECT".to_string()],
                    grantable_privileges: Vec::new(),
                    with_grant_option: false,
                },
                GrantInfo {
                    target_kind: "sequence".to_string(),
                    schema_name: "dbstate_slice2".to_string(),
                    object_name: Some("account_number_seq".to_string()),
                    identity_arguments: None,
                    grantee: "app_writer".to_string(),
                    grantor: Some("dbstate_owner".to_string()),
                    privileges: vec!["USAGE".to_string(), "SELECT".to_string()],
                    grantable_privileges: vec!["USAGE".to_string()],
                    with_grant_option: true,
                },
                GrantInfo {
                    target_kind: "function".to_string(),
                    schema_name: "dbstate_slice2".to_string(),
                    object_name: Some("account_label".to_string()),
                    identity_arguments: Some("account_id integer".to_string()),
                    grantee: "app_reader".to_string(),
                    grantor: Some("dbstate_owner".to_string()),
                    privileges: vec!["EXECUTE".to_string()],
                    grantable_privileges: Vec::new(),
                    with_grant_option: false,
                },
            ],
            rls_policies: vec![
                RlsPolicyInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    policy_name: "sample_accounts_public_read".to_string(),
                    command: "SELECT".to_string(),
                    policy_kind: "PERMISSIVE".to_string(),
                    roles: vec!["PUBLIC".to_string()],
                    using_expression: Some("true".to_string()),
                    with_check_expression: None,
                    table_rls_enabled: Some(true),
                    table_rls_forced: Some(false),
                },
                RlsPolicyInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    policy_name: "sample_accounts_insert_check".to_string(),
                    command: "INSERT".to_string(),
                    policy_kind: "PERMISSIVE".to_string(),
                    roles: vec!["app_writer".to_string()],
                    using_expression: None,
                    with_check_expression: Some("account_code <> ''::text".to_string()),
                    table_rls_enabled: Some(true),
                    table_rls_forced: Some(false),
                },
                RlsPolicyInfo {
                    schema_name: "dbstate_slice2".to_string(),
                    table_name: "sample_accounts".to_string(),
                    policy_name: "sample_accounts_restrictive_update".to_string(),
                    command: "UPDATE".to_string(),
                    policy_kind: "RESTRICTIVE".to_string(),
                    roles: vec!["PUBLIC".to_string(), "app_writer".to_string()],
                    using_expression: Some("account_id > 0".to_string()),
                    with_check_expression: Some("account_id > 0".to_string()),
                    table_rls_enabled: Some(true),
                    table_rls_forced: Some(false),
                },
            ],
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

fn partitioned_payment_inventory() -> PostgresInventory {
    PostgresInventory {
        schemas: vec![SchemaInfo {
            name: "public".to_string(),
        }],
        tables: vec![TableInfo {
            schema_name: "public".to_string(),
            table_name: "payment".to_string(),
            table_type: "PARTITIONED TABLE".to_string(),
            partition_key: Some("RANGE (payment_date)".to_string()),
        }],
        columns: vec![
            ColumnInfo {
                schema_name: "public".to_string(),
                table_name: "payment".to_string(),
                column_name: "payment_id".to_string(),
                ordinal_position: 1,
                data_type: "integer".to_string(),
                is_nullable: false,
                has_default: false,
                default_expression: None,
            },
            ColumnInfo {
                schema_name: "public".to_string(),
                table_name: "payment".to_string(),
                column_name: "payment_date".to_string(),
                ordinal_position: 2,
                data_type: "timestamp without time zone".to_string(),
                is_nullable: false,
                has_default: false,
                default_expression: None,
            },
        ],
        extensions: Vec::new(),
        enums: Vec::new(),
        domains: Vec::new(),
        sequences: Vec::new(),
        indexes: Vec::new(),
        views: Vec::new(),
        materialized_views: Vec::new(),
        constraints: vec![ConstraintInfo {
            schema_name: "public".to_string(),
            table_name: "payment".to_string(),
            constraint_name: "payment_pkey".to_string(),
            constraint_type: "primaryKey".to_string(),
            definition: "PRIMARY KEY (payment_id, payment_date)".to_string(),
            columns: vec!["payment_id".to_string(), "payment_date".to_string()],
            referenced_schema: None,
            referenced_table: None,
            referenced_columns: Vec::new(),
        }],
        functions: Vec::new(),
        aggregates: Vec::new(),
        triggers: Vec::new(),
        grants: Vec::new(),
        rls_policies: Vec::new(),
    }
}

fn reference_data_review_compare_fixture() -> ReferenceDataCompareReport {
    let (config, mut state) = reference_config_and_state();
    for row in &mut state.rows {
        if row.values.get("code").and_then(Option::as_deref) == Some("WIRE") {
            row.values
                .insert("name".to_string(), Some("Wire's Transfer".to_string()));
        }
    }
    let database_rows = vec![
        reference_row(&[
            ("code", Some("CASH")),
            ("name", Some("Cash")),
            ("is_active", Some("true")),
            ("sort_order", Some("10")),
            ("updated_at", Some("different ignored value")),
            ("secret_note", Some("database-secret-value")),
        ]),
        reference_row(&[
            ("code", Some("QRPH")),
            ("name", Some("QRPh Live")),
            ("is_active", Some("true")),
            ("sort_order", Some("20")),
            ("updated_at", Some("different ignored value")),
            ("secret_note", Some("database-secret-qrph")),
        ]),
        reference_row(&[
            ("code", Some("CARD")),
            ("name", Some("Card")),
            ("is_active", Some("true")),
            ("sort_order", Some("30")),
            ("secret_note", Some("database-secret-card")),
        ]),
    ];
    let mut table = compare_reference_data_table(
        &config,
        &state,
        &database_rows,
        &reference_database_columns(),
    );
    table
        .foreign_key_references
        .push(ReferenceDataForeignKeyReference {
            constraint_name: "city_country_id_fkey".to_string(),
            referencing_schema: "public".to_string(),
            referencing_table: "city".to_string(),
            referencing_columns: vec!["country_id".to_string()],
            referenced_schema: "dbstate_ref".to_string(),
            referenced_table: "payment_methods".to_string(),
            referenced_columns: vec!["code".to_string()],
        });
    let mut report = empty_reference_data_compare_report();
    report.success = true;
    report.selected_tables = vec![table.table_name.clone()];
    append_reference_table_result(&mut report, table);
    report
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
    assert!(report
        .planned_creates
        .contains(&"database/releases/objects".to_string()));
    assert!(report
        .planned_creates
        .contains(&"database/objects/domains".to_string()));
    assert!(report
        .planned_creates
        .contains(&"database/objects/aggregates".to_string()));
    assert!(report
        .planned_creates
        .contains(&"database/releases/reference-data".to_string()));
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
    assert!(dir.join("database/objects/aggregates").is_dir());
    assert!(dir.join("database/objects/domains").is_dir());
    assert!(dir.join("database/releases/objects").is_dir());
    assert!(dir.join("database/releases/reference-data").is_dir());
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
fn project_status_reports_missing_release_artifact_subfolders() {
    let dir = create_temp_dir("missing-release-subfolders");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::remove_dir_all(dir.join("database/releases/objects")).expect("remove objects release dir");
    fs::remove_dir_all(dir.join("database/releases/reference-data"))
        .expect("remove reference-data release dir");

    let report = status_report(&dir, CommandKind::RepoStatus);

    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::PartialDbStateStructure
    );
    assert!(report
        .missing_paths
        .contains(&"database/releases/objects".to_string()));
    assert!(report
        .missing_paths
        .contains(&"database/releases/reference-data".to_string()));
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
fn dirty_working_tree_warning_is_reported_and_init_allows_unrelated_paths() {
    let dir = create_temp_dir("dirty");
    init_git_repo(&dir);
    fs::write(dir.join("untracked.txt"), "dirty").expect("write dirty file");

    let status = status_report(&dir, CommandKind::RepoStatus);
    assert!(status.is_dirty);
    assert!(!status.warnings.is_empty());

    let init = init_project(&dir, false).expect("init project");
    assert!(init.success, "{:?}", init.errors);
    assert!(!init.created_paths.is_empty());
    assert!(dir.join("database").exists());
    assert!(init
        .warnings
        .iter()
        .any(|warning| warning.contains("unrelated changes")));
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
        "\"materializedViews\"",
        "\"constraints\"",
        "\"functions\"",
        "\"triggers\"",
        "\"grants\"",
        "\"rlsPolicies\"",
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
            partition_key: None,
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
        domains: vec![DomainInfo {
            schema_name: "app".to_string(),
            domain_name: "order_year".to_string(),
            base_type: "integer".to_string(),
            is_not_null: false,
            default_expression: None,
            check_constraints: vec![DomainCheckInfo {
                constraint_name: "order_year_check".to_string(),
                definition: "CHECK ((VALUE >= 2000))".to_string(),
            }],
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
        materialized_views: vec![MaterializedViewInfo {
            schema_name: "app".to_string(),
            materialized_view_name: "order_summary".to_string(),
            definition: " SELECT orders.id FROM app.orders".to_string(),
            is_populated: Some(false),
            tablespace: None,
        }],
        constraints: vec![
            ConstraintInfo {
                schema_name: "app".to_string(),
                table_name: "orders".to_string(),
                constraint_name: "orders_pkey".to_string(),
                constraint_type: "primaryKey".to_string(),
                definition: "PRIMARY KEY (id)".to_string(),
                columns: vec!["id".to_string()],
                referenced_schema: None,
                referenced_table: None,
                referenced_columns: Vec::new(),
            },
            ConstraintInfo {
                schema_name: "app".to_string(),
                table_name: "orders".to_string(),
                constraint_name: "orders_number_key".to_string(),
                constraint_type: "uniqueConstraint".to_string(),
                definition: "UNIQUE (id)".to_string(),
                columns: vec!["id".to_string()],
                referenced_schema: None,
                referenced_table: None,
                referenced_columns: Vec::new(),
            },
            ConstraintInfo {
                schema_name: "app".to_string(),
                table_name: "orders".to_string(),
                constraint_name: "orders_parent_fkey".to_string(),
                constraint_type: "foreignKey".to_string(),
                definition: "FOREIGN KEY (id) REFERENCES app.orders(id)".to_string(),
                columns: vec!["id".to_string()],
                referenced_schema: Some("app".to_string()),
                referenced_table: Some("orders".to_string()),
                referenced_columns: vec!["id".to_string()],
            },
            ConstraintInfo {
                schema_name: "app".to_string(),
                table_name: "orders".to_string(),
                constraint_name: "orders_id_check".to_string(),
                constraint_type: "checkConstraint".to_string(),
                definition: "CHECK ((id > 0))".to_string(),
                columns: vec!["id".to_string()],
                referenced_schema: None,
                referenced_table: None,
                referenced_columns: Vec::new(),
            },
        ],
        functions: vec![
            FunctionInfo {
                schema_name: "app".to_string(),
                function_name: "order_label".to_string(),
                identity_arguments: "order_id integer".to_string(),
                result_type: Some("text".to_string()),
                language: Some("sql".to_string()),
                volatility: Some("stable".to_string()),
                security_definer: false,
                is_strict: false,
                definition:
                    "CREATE FUNCTION app.order_label(order_id integer)\n RETURNS text\n LANGUAGE sql\n STABLE\nAS $function$\n    SELECT order_id::text;\n$function$"
                        .to_string(),
            },
            FunctionInfo {
                schema_name: "app".to_string(),
                function_name: "order_label".to_string(),
                identity_arguments: "order_code text".to_string(),
                result_type: Some("text".to_string()),
                language: Some("sql".to_string()),
                volatility: Some("stable".to_string()),
                security_definer: false,
                is_strict: true,
                definition:
                    "CREATE FUNCTION app.order_label(order_code text)\n RETURNS text\n LANGUAGE sql\n STABLE\n STRICT\nAS $function$\n    SELECT upper(order_code);\n$function$"
                        .to_string(),
            },
        ],
        aggregates: vec![AggregateInfo {
            schema_name: "app".to_string(),
            aggregate_name: "order_labels".to_string(),
            identity_arguments: "text".to_string(),
            transition_function_schema: "app".to_string(),
            transition_function_name: "order_label_concat".to_string(),
            state_type: "text".to_string(),
            final_function_schema: None,
            final_function_name: None,
            initial_condition: None,
            sort_operator: None,
        }],
        triggers: vec![TriggerInfo {
            schema_name: "app".to_string(),
            relation_name: "orders".to_string(),
            trigger_name: "orders_audit_trigger".to_string(),
            trigger_function_schema: Some("app".to_string()),
            trigger_function_name: Some("order_label".to_string()),
            timing: Some("AFTER".to_string()),
            events: vec!["INSERT".to_string(), "UPDATE".to_string()],
            orientation: Some("ROW".to_string()),
            definition:
                "CREATE TRIGGER orders_audit_trigger AFTER INSERT OR UPDATE ON app.orders FOR EACH ROW EXECUTE FUNCTION app.order_label(order_id)"
                    .to_string(),
        }],
        grants: vec![GrantInfo {
            target_kind: "table".to_string(),
            schema_name: "app".to_string(),
            object_name: Some("orders".to_string()),
            identity_arguments: None,
            grantee: "app_reader".to_string(),
            grantor: Some("app_owner".to_string()),
            privileges: vec!["SELECT".to_string()],
            grantable_privileges: Vec::new(),
            with_grant_option: false,
        }],
        rls_policies: vec![RlsPolicyInfo {
            schema_name: "app".to_string(),
            table_name: "orders".to_string(),
            policy_name: "orders_tenant_isolation".to_string(),
            command: "SELECT".to_string(),
            policy_kind: "PERMISSIVE".to_string(),
            roles: vec!["PUBLIC".to_string(), "app_reader".to_string()],
            using_expression: Some("tenant_id = current_setting('app.tenant_id')::uuid".to_string()),
            with_check_expression: None,
            table_rls_enabled: Some(true),
            table_rls_forced: Some(false),
        }],
    };

    assert_eq!(inventory.schemas[0].name, "app");
    assert_eq!(inventory.tables[0].table_name, "orders");
    assert_eq!(inventory.columns[0].column_name, "id");
    assert_eq!(inventory.extensions[0].extension_name, "pgcrypto");
    assert_eq!(inventory.enums[0].labels, vec!["new", "paid"]);
    assert_eq!(inventory.domains[0].domain_name, "order_year");
    assert_eq!(inventory.domains[0].base_type, "integer");
    assert_eq!(inventory.sequences[0].sequence_name, "orders_id_seq");
    assert_eq!(inventory.indexes[0].index_name, "orders_created_at_idx");
    assert_eq!(inventory.views[0].view_name, "open_orders");
    assert_eq!(inventory.materialized_views.len(), 1);
    assert_eq!(
        inventory.materialized_views[0].materialized_view_name,
        "order_summary"
    );
    assert_eq!(inventory.constraints.len(), 4);
    assert_eq!(inventory.constraints[0].constraint_type, "primaryKey");
    assert_eq!(inventory.constraints[1].constraint_type, "uniqueConstraint");
    assert_eq!(inventory.constraints[2].constraint_type, "foreignKey");
    assert_eq!(inventory.constraints[3].constraint_type, "checkConstraint");
    assert_eq!(inventory.functions.len(), 2);
    assert_eq!(inventory.functions[0].function_name, "order_label");
    assert_ne!(
        inventory.functions[0].identity_arguments,
        inventory.functions[1].identity_arguments
    );
    assert_eq!(inventory.triggers.len(), 1);
    assert_eq!(inventory.triggers[0].relation_name, "orders");
    assert_eq!(inventory.triggers[0].trigger_name, "orders_audit_trigger");
    assert_eq!(inventory.grants[0].grantee, "app_reader");
    assert_eq!(
        inventory.rls_policies[0].policy_name,
        "orders_tenant_isolation"
    );
    assert_eq!(inventory.rls_policies[0].roles[0], "PUBLIC");
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
        .contains(&"domains".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"sequences".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"indexes".to_string()));
    assert!(!report.deferred_object_types.contains(&"views".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"materializedViews".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"primaryKeys".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"uniqueConstraints".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"foreignKeys".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"checkConstraints".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"functions".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"triggers".to_string()));
    assert!(!report.deferred_object_types.contains(&"grants".to_string()));
    assert!(!report
        .deferred_object_types
        .contains(&"rlsPolicies".to_string()));
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
        domain_file_path("core", "payment_year").expect("domain path"),
        "database/objects/domains/core.payment_year.sql"
    );
    assert_eq!(
        aggregate_file_path("core", "group_concat", "text").expect("aggregate path"),
        "database/objects/aggregates/core.group_concat.text.sql"
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
    assert_eq!(
        materialized_view_file_path("core", "active_payments").expect("materialized view path"),
        "database/objects/materialized-views/core.active_payments.sql"
    );
    assert_eq!(
        constraint_file_path("primaryKey", "core", "payments", "payments_pkey")
            .expect("primary key path"),
        "database/objects/constraints/primary-keys/core.payments.payments_pkey.sql"
    );
    assert_eq!(
        constraint_file_path("uniqueConstraint", "core", "payments", "payments_code_key")
            .expect("unique constraint path"),
        "database/objects/constraints/unique-constraints/core.payments.payments_code_key.sql"
    );
    assert_eq!(
        constraint_file_path("foreignKey", "core", "payments", "payments_customer_fkey")
            .expect("foreign key path"),
        "database/objects/constraints/foreign-keys/core.payments.payments_customer_fkey.sql"
    );
    assert_eq!(
        constraint_file_path(
            "checkConstraint",
            "core",
            "payments",
            "payments_amount_check"
        )
        .expect("check constraint path"),
        "database/objects/constraints/check-constraints/core.payments.payments_amount_check.sql"
    );
    assert_eq!(
        function_file_path("core", "calculate_total", "integer, numeric").expect("function path"),
        "database/objects/functions/core.calculate_total.integer_numeric.sql"
    );
    assert_eq!(
        function_file_path("core", "now_utc", "").expect("function no-args path"),
        "database/objects/functions/core.now_utc.no_args.sql"
    );
    assert_eq!(
        function_identity_slug("integer, numeric").expect("function slug"),
        "integer_numeric"
    );
    assert_eq!(
        trigger_file_path("core", "payments", "payments_audit_trigger").expect("trigger path"),
        "database/objects/triggers/core.payments.payments_audit_trigger.sql"
    );
    assert_eq!(
        grant_file_path("schema", "core", None, None, "PUBLIC").expect("schema grant path"),
        "database/objects/grants/schemas/core.public.sql"
    );
    assert_eq!(
        grant_file_path("table", "core", Some("payments"), None, "app_reader")
            .expect("table grant path"),
        "database/objects/grants/tables/core.payments.app_reader.sql"
    );
    assert_eq!(
        grant_file_path(
            "function",
            "core",
            Some("calculate_total"),
            Some("integer_numeric"),
            "app_reader"
        )
        .expect("function grant path"),
        "database/objects/grants/functions/core.calculate_total.integer_numeric.app_reader.sql"
    );
    assert_eq!(
        rls_policy_file_path("core", "payments", "payments_tenant_policy")
            .expect("RLS policy path"),
        "database/objects/rls-policies/core.payments.payments_tenant_policy.sql"
    );
    assert!(schema_file_path("../evil").is_err());
    assert!(table_file_path("core", "bad/name").is_err());
    assert!(domain_file_path("core", "bad/name").is_err());
    assert!(aggregate_file_path("core", "bad/name", "integer").is_err());
    assert!(function_file_path("core", "bad/name", "integer").is_err());
    assert!(trigger_file_path("core", "payments", "bad/name").is_err());
    assert!(materialized_view_file_path("core", "bad/name").is_err());
    assert!(grant_file_path("table", "core", Some("payments"), None, "bad/name").is_err());
    assert!(rls_policy_file_path("core", "payments", "bad/name").is_err());
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
fn generated_partitioned_table_sql_includes_partition_key() {
    let inventory = partitioned_payment_inventory();
    let sql = render_table_sql_with_partition(
        "public",
        "payment",
        &inventory.columns,
        inventory.tables[0].partition_key.as_deref(),
    );

    assert!(sql.contains("CREATE TABLE \"public\".\"payment\" ("));
    assert!(sql.contains("\"payment_date\" timestamp without time zone NOT NULL"));
    assert!(sql.contains(") PARTITION BY RANGE (payment_date);"));
    assert!(!sql.contains("DROP TABLE"));
}

#[test]
fn generated_slice16_object_sql_is_deterministic() {
    let inventory = sample_inventory();

    assert!(render_extension_sql(&inventory.extensions[0])
        .contains("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\";"));
    assert!(render_enum_sql(&inventory.enums[0])
        .contains("CREATE TYPE \"dbstate_slice2\".\"account_status\" AS ENUM"));
    assert!(render_enum_sql(&inventory.enums[0]).contains("'active',"));
    assert!(render_domain_sql(&inventory.domains[0])
        .contains("CREATE DOMAIN \"dbstate_slice2\".\"account_year\" AS integer"));
    assert!(render_domain_sql(&inventory.domains[0])
        .contains("CONSTRAINT \"account_year_check\" CHECK ((VALUE >= 1901) AND (VALUE <= 2155))"));
    assert!(!render_domain_sql(&inventory.domains[0]).contains("DROP DOMAIN"));
    assert!(render_aggregate_sql(&inventory.aggregates[0])
        .contains("CREATE AGGREGATE \"dbstate_slice2\".\"account_labels\"(text)"));
    assert!(render_aggregate_sql(&inventory.aggregates[0])
        .contains("SFUNC = \"dbstate_slice2\".\"account_label_concat\""));
    assert!(render_aggregate_sql(&inventory.aggregates[0]).contains("STYPE = text"));
    assert!(!render_aggregate_sql(&inventory.aggregates[0]).contains("DROP AGGREGATE"));
    assert!(render_sequence_sql(&inventory.sequences[0])
        .contains("CREATE SEQUENCE \"dbstate_slice2\".\"account_number_seq\""));
    assert!(render_index_sql(&inventory.indexes[0])
        .contains("CREATE UNIQUE INDEX sample_accounts_account_code_idx"));
    assert!(render_view_sql(&inventory.views[0])
        .contains("CREATE VIEW \"dbstate_slice2\".\"active_accounts\" AS"));
    assert!(
        render_materialized_view_sql(&inventory.materialized_views[0])
            .contains("CREATE MATERIALIZED VIEW \"dbstate_slice2\".\"account_summary\" AS")
    );
    assert!(
        render_materialized_view_sql(&inventory.materialized_views[0]).contains("WITH NO DATA;")
    );
    assert!(render_materialized_view_sql(&inventory.materialized_views[0]).ends_with(";\n"));
    assert!(
        !render_materialized_view_sql(&inventory.materialized_views[0])
            .contains("DROP MATERIALIZED VIEW")
    );
    assert!(
        !render_materialized_view_sql(&inventory.materialized_views[0])
            .contains("REFRESH MATERIALIZED VIEW")
    );
    assert!(render_constraint_sql(&inventory.constraints[0]).contains(
        "ALTER TABLE \"dbstate_slice2\".\"sample_accounts\"\n    ADD CONSTRAINT \"sample_accounts_pkey\" PRIMARY KEY (account_id);"
    ));
    assert!(render_constraint_sql(&inventory.constraints[1])
        .contains("ADD CONSTRAINT \"sample_accounts_code_key\" UNIQUE"));
    assert!(render_constraint_sql(&inventory.constraints[2])
        .contains("ADD CONSTRAINT \"sample_accounts_parent_fkey\" FOREIGN KEY"));
    assert!(render_constraint_sql(&inventory.constraints[3])
        .contains("ADD CONSTRAINT \"sample_accounts_code_check\" CHECK"));
    assert!(!render_constraint_sql(&inventory.constraints[0]).contains("DROP"));
    assert!(render_function_sql(&inventory.functions[0])
        .contains("-- Object type: function\n-- Object name: dbstate_slice2.account_label(account_id integer)"));
    assert!(render_function_sql(&inventory.functions[0])
        .contains("CREATE FUNCTION dbstate_slice2.account_label(account_id integer)"));
    assert!(render_function_sql(&inventory.functions[1]).contains("STRICT"));
    assert!(render_function_sql(&inventory.functions[0]).ends_with(";\n"));
    assert!(!render_function_sql(&inventory.functions[0]).contains("DROP FUNCTION"));
    assert!(render_trigger_sql(&inventory.triggers[0]).contains("-- Object type: trigger"));
    assert!(render_trigger_sql(&inventory.triggers[0]).contains("CREATE TRIGGER"));
    assert!(render_trigger_sql(&inventory.triggers[0]).ends_with(";\n"));
    assert!(!render_trigger_sql(&inventory.triggers[0]).contains("DROP TRIGGER"));
}

#[test]
fn grant_sql_rendering_is_deterministic_and_review_only() {
    let schema_sql = render_grant_sql(&sample_inventory().grants[0]);
    assert!(schema_sql.contains("GRANT USAGE ON SCHEMA \"dbstate_slice2\" TO \"app_reader\";"));
    assert!(schema_sql.ends_with('\n'));
    assert!(!schema_sql.contains("REVOKE"));

    let public_sql = render_grant_sql(&sample_inventory().grants[1]);
    assert!(public_sql
        .contains("GRANT SELECT ON TABLE \"dbstate_slice2\".\"sample_accounts\" TO PUBLIC;"));
    assert!(!public_sql.contains("TO \"PUBLIC\""));

    let sequence_sql = render_grant_sql(&sample_inventory().grants[2]);
    assert!(sequence_sql.contains(
        "GRANT SELECT ON SEQUENCE \"dbstate_slice2\".\"account_number_seq\" TO \"app_writer\";"
    ));
    assert!(sequence_sql.contains(
        "GRANT USAGE ON SEQUENCE \"dbstate_slice2\".\"account_number_seq\" TO \"app_writer\" WITH GRANT OPTION;"
    ));

    let function_sql = render_grant_sql(&sample_inventory().grants[3]);
    assert!(function_sql.contains(
        "GRANT EXECUTE ON FUNCTION \"dbstate_slice2\".\"account_label\"(account_id integer) TO \"app_reader\";"
    ));
}

#[test]
fn rls_policy_sql_rendering_is_deterministic_and_review_only() {
    let select_sql = render_rls_policy_sql(&sample_inventory().rls_policies[0]);
    assert!(select_sql.contains("-- Object type: rlsPolicy"));
    assert!(select_sql.contains("CREATE POLICY \"sample_accounts_public_read\""));
    assert!(select_sql.contains("ON \"dbstate_slice2\".\"sample_accounts\""));
    assert!(select_sql.contains("AS PERMISSIVE"));
    assert!(select_sql.contains("FOR SELECT"));
    assert!(select_sql.contains("TO PUBLIC"));
    assert!(select_sql.contains("USING (true)"));
    assert!(select_sql.ends_with(";\n"));
    assert!(!select_sql.contains("DROP POLICY"));
    assert!(!select_sql.contains("ALTER POLICY"));
    assert!(!select_sql.contains("ENABLE ROW LEVEL SECURITY"));
    assert!(!select_sql.contains("DISABLE ROW LEVEL SECURITY"));
    assert!(!select_sql.contains("FORCE ROW LEVEL SECURITY"));

    let insert_sql = render_rls_policy_sql(&sample_inventory().rls_policies[1]);
    assert!(insert_sql.contains("FOR INSERT"));
    assert!(insert_sql.contains("TO \"app_writer\""));
    assert!(insert_sql.contains("WITH CHECK (account_code <> ''::text)"));

    let restrictive_sql = render_rls_policy_sql(&sample_inventory().rls_policies[2]);
    assert!(restrictive_sql.contains("AS RESTRICTIVE"));
    assert!(restrictive_sql.contains("TO PUBLIC, \"app_writer\""));
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
fn export_write_allows_dirty_working_tree_when_paths_are_unrelated() {
    let dir = create_temp_dir("export-dirty");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report =
        export_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(report.success, "{:?}", report.errors);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("unrelated changes")));
    assert!(!report.created_files.is_empty());
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
fn table_export_includes_domain_dependencies_for_domain_typed_columns() {
    let dir = create_temp_dir("export-table-domain");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let mut inventory = sample_inventory();
    inventory.columns.push(ColumnInfo {
        schema_name: "dbstate_slice2".to_string(),
        table_name: "sample_accounts".to_string(),
        column_name: "account_opened_year".to_string(),
        ordinal_position: 5,
        data_type: "account_year".to_string(),
        is_nullable: true,
        has_default: false,
        default_expression: None,
    });

    let report = export_postgres_with_inventory(
        &dir,
        &inventory,
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        true,
    );

    assert!(report.success, "{:?}", report.errors);
    assert!(report
        .planned_files
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    assert!(report
        .planned_files
        .contains(&"database/objects/domains/dbstate_slice2.account_year.sql".to_string()));
}

#[test]
fn table_export_includes_partitioned_parent_table() {
    let dir = create_temp_dir("export-partitioned-parent");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report = export_postgres_with_inventory(
        &dir,
        &partitioned_payment_inventory(),
        &ExportSelection::Table {
            schema: "public".to_string(),
            table: "payment".to_string(),
        },
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert!(report
        .created_files
        .contains(&"database/objects/tables/public.payment.sql".to_string()));
    let table_sql =
        fs::read_to_string(dir.join("database/objects/tables/public.payment.sql")).expect("table");
    assert!(table_sql.contains("CREATE TABLE \"public\".\"payment\""));
    assert!(table_sql.contains("PARTITION BY RANGE (payment_date);"));
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
    assert!(dir
        .join("database/objects/domains/dbstate_slice2.account_year.sql")
        .is_file());
    assert!(dir
        .join("database/objects/materialized-views/dbstate_slice2.account_summary.sql")
        .is_file());
    assert!(dir
        .join("database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql")
        .is_file());
    assert!(dir
        .join("database/objects/constraints/unique-constraints/dbstate_slice2.sample_accounts.sample_accounts_code_key.sql")
        .is_file());
    assert!(dir
        .join("database/objects/constraints/foreign-keys/dbstate_slice2.sample_accounts.sample_accounts_parent_fkey.sql")
        .is_file());
    assert!(dir
        .join("database/objects/constraints/check-constraints/dbstate_slice2.sample_accounts.sample_accounts_code_check.sql")
        .is_file());
    assert!(dir
        .join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql")
        .is_file());
    assert!(dir
        .join("database/objects/functions/dbstate_slice2.account_label.account_code_text.sql")
        .is_file());
    assert!(dir
        .join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql",
        )
        .is_file());
    assert!(dir
        .join("database/objects/grants/schemas/dbstate_slice2.app_reader.sql")
        .is_file());
    assert!(dir
        .join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql")
        .is_file());
    assert!(dir
        .join("database/objects/grants/sequences/dbstate_slice2.account_number_seq.app_writer.sql")
        .is_file());
    assert!(dir
        .join("database/objects/grants/functions/dbstate_slice2.account_label.account_id_integer.app_reader.sql")
        .is_file());
    assert!(dir
        .join("database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_public_read.sql")
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
    assert!(report
        .planned_creates
        .contains(&"database/objects/domains/dbstate_slice2.account_year.sql".to_string()));
    assert!(report.planned_creates.contains(
        &"database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql"
            .to_string()
    ));
    assert!(report.planned_creates.contains(
        &"database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"
            .to_string()
    ));
    assert!(report.planned_creates.contains(
        &"database/objects/materialized-views/dbstate_slice2.account_summary.sql".to_string()
    ));
    assert!(report.planned_creates.contains(
        &"database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql"
            .to_string()
    ));
    assert!(report.planned_creates.contains(
        &"database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql".to_string()
    ));
    assert!(report.planned_creates.contains(
        &"database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_public_read.sql"
            .to_string()
    ));
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
    assert!(!dir
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());
}

#[test]
fn sync_include_paths_limit_repository_writes_to_exact_selected_paths() {
    let dir = create_temp_dir("sync-selected-paths");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let selected = vec![
        "database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string(),
        "database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"
            .to_string(),
    ];
    let report = sync_postgres_with_inventory_for_paths(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        false,
        &selected,
    );

    assert!(report.success, "{:?}", report.errors);
    let mut created_files = report.created_files.clone();
    created_files.sort();
    assert_eq!(
        created_files,
        vec![
            "database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"
                .to_string(),
            "database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string(),
        ]
    );
    let mut planned_creates = report.planned_creates.clone();
    planned_creates.sort();
    assert_eq!(planned_creates, created_files);
    assert!(report.updated_files.is_empty());
    assert!(dir
        .join("database/objects/tables/dbstate_slice2.sample_accounts.sql")
        .is_file());
    assert!(dir
        .join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql")
        .is_file());
    assert!(!dir
        .join("database/objects/aggregates/dbstate_slice2.account_total.integer.sql")
        .exists());
}

#[test]
fn sync_include_paths_reject_unsafe_duplicate_and_unplanned_paths() {
    let dir = create_temp_dir("sync-selected-path-safety");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    for selected in [
        vec!["C:/DbState/pagila/database/objects/tables/public.country.sql".to_string()],
        vec!["database/objects/../objects/tables/public.country.sql".to_string()],
        vec!["database/releases/objects/public.country.sql".to_string()],
        vec![
            "database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string(),
            "database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string(),
        ],
        vec!["database/objects/tables/missing.sql".to_string()],
    ] {
        let report = sync_postgres_with_inventory_for_paths(
            &dir,
            &sample_inventory(),
            &ExportSelection::All,
            true,
            &selected,
        );
        assert!(!report.success, "selected paths should fail: {selected:?}");
        assert!(
            !report.errors.is_empty(),
            "selected paths should report an error"
        );
    }

    let backslash_selected =
        vec!["database\\objects\\tables\\dbstate_slice2.sample_accounts.sql".to_string()];
    let report = sync_postgres_with_inventory_for_paths(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        true,
        &backslash_selected,
    );
    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.planned_creates,
        vec!["database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()]
    );
}

#[test]
fn repository_sync_write_rejects_explicit_empty_include_selection() {
    let dir = create_temp_dir("sync-empty-selection-service");
    init_git_repo(&dir);
    create_complete_structure(&dir);

    let body = format!(
        r#"{{
            "repositoryPath": "{}",
            "scope": "all",
            "confirmRepositoryWrite": true,
            "confirmationText": "WRITE REPOSITORY FILES",
            "include": []
        }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response(
        "POST",
        "/api/v1/postgres/repository-sync/write",
        &body,
        &dir,
    );

    assert_eq!(response.status_code, 400, "{}", response.body);
    assert!(response
        .body
        .contains("Select at least one repository object file"));
    assert!(!response.body.contains("PostgreSQL connection"));
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
fn sync_write_allows_dirty_working_tree_when_paths_are_unrelated() {
    let dir = create_temp_dir("sync-dirty");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(report.success, "{:?}", report.errors);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("unrelated changes")));
    assert!(!report.created_files.is_empty());
}

#[test]
fn slice35_protected_branch_blocks_write_actions() {
    let dir = create_temp_dir("slice35-protected-branch");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    set_git_head_branch(&dir, "dev");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("Write blocked on protected branch: dev")));
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("git switch -c dbstate/schema-export/")));
    assert!(report.created_files.is_empty());
}

#[test]
fn slice35_dirty_overlapping_target_paths_block_write_actions() {
    let dir = create_temp_dir("slice35-overlap");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let table_path = dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    fs::write(&table_path, "-- stale table definition\n").expect("write stale table file");
    commit_all(&dir, "stale table");
    fs::write(&table_path, "-- local uncommitted table definition\n").expect("dirty table file");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report.errors.iter().any(|error| {
        error.contains("Write blocked because the target file has uncommitted changes")
            && error.contains("database/objects/tables/dbstate_slice2.sample_accounts.sql")
    }));
    assert!(report.updated_files.is_empty());
}

#[test]
fn slice35_untracked_target_path_overwrite_is_blocked() {
    let dir = create_temp_dir("slice35-untracked-target");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let table_path = dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    fs::write(&table_path, "-- untracked local table definition\n")
        .expect("write untracked target file");

    let report = sync_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        false,
    );

    assert!(!report.success);
    assert!(report.errors.iter().any(|error| {
        error.contains("Write blocked because the target file has uncommitted changes")
            && error.contains("database/objects/tables/dbstate_slice2.sample_accounts.sql")
    }));
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
}

#[test]
fn slice35_staged_target_path_changes_block_write_actions() {
    let dir = create_temp_dir("slice35-staged-target");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let table_path = dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    fs::write(&table_path, "-- stale table definition\n").expect("write stale table file");
    commit_all(&dir, "stale table");
    fs::write(&table_path, "-- staged local table definition\n").expect("dirty table file");
    let add = Command::new("git")
        .arg("add")
        .arg("database/objects/tables/dbstate_slice2.sample_accounts.sql")
        .current_dir(&dir)
        .output()
        .expect("stage target file");
    assert!(add.status.success(), "git add target failed");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report.errors.iter().any(|error| {
        error.contains("Write blocked because the target file has uncommitted changes")
            && error.contains("database/objects/tables/dbstate_slice2.sample_accounts.sql")
    }));
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
}

#[test]
fn slice35_deleted_target_path_blocks_write_actions() {
    let dir = create_temp_dir("slice35-deleted-target");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let table_path = dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    fs::write(&table_path, "-- stale table definition\n").expect("write stale table file");
    commit_all(&dir, "stale table");
    fs::remove_file(&table_path).expect("delete target file");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report.errors.iter().any(|error| {
        error.contains("Write blocked because the target file has uncommitted changes")
            && error.contains("database/objects/tables/dbstate_slice2.sample_accounts.sql")
    }));
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
}

#[test]
fn slice35_renamed_target_path_blocks_write_actions() {
    let dir = create_temp_dir("slice35-renamed-target");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let table_path = "database/objects/tables/dbstate_slice2.sample_accounts.sql";
    fs::write(dir.join(table_path), "-- stale table definition\n").expect("write stale table file");
    commit_all(&dir, "stale table");
    let rename = Command::new("git")
        .arg("mv")
        .arg(table_path)
        .arg("database/objects/tables/dbstate_slice2.sample_accounts_renamed.sql")
        .current_dir(&dir)
        .output()
        .expect("rename target file");
    assert!(rename.status.success(), "git mv target failed");

    let status = status_report(&dir, CommandKind::RepoStatus);
    assert!(status.dirty_paths.contains(&table_path.to_string()));
    assert!(status.dirty_paths.contains(
        &"database/objects/tables/dbstate_slice2.sample_accounts_renamed.sql".to_string()
    ));

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(!report.success);
    assert!(report.errors.iter().any(|error| {
        error.contains("Write blocked because the target file has uncommitted changes")
            && error.contains(table_path)
    }));
    assert!(report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
}

#[test]
fn slice35_non_overlapping_dbstate_managed_changes_allow_scoped_write_with_warning() {
    let dir = create_temp_dir("slice35-managed-unrelated");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let unrelated_object = dir.join("database/objects/schemas/local_only.sql");
    fs::write(&unrelated_object, render_schema_sql("local_only"))
        .expect("write unrelated object file");
    commit_all(&dir, "local object");
    fs::write(&unrelated_object, "-- local object work in progress\n")
        .expect("modify unrelated object file");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(report.success, "{:?}", report.errors);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("unrelated changes")));
    assert!(!report.created_files.is_empty());
}

#[test]
fn slice35_status_reports_untracked_paths_with_spaces_and_unicode() {
    let dir = create_temp_dir("slice35-path-visibility");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let nested = dir.join("database/objects/tables/nested folder");
    fs::create_dir_all(&nested).expect("create nested folder");
    fs::write(nested.join("customer email Δ.sql"), "-- local object\n")
        .expect("write unicode path");

    let report = status_report(&dir, CommandKind::RepoStatus);

    assert!(report.is_dirty);
    assert_eq!(report.dirty_paths.len(), 1);
    assert!(report
        .dirty_paths
        .contains(&"database/objects/tables/nested folder/customer email Δ.sql".to_string()));
    assert!(report.to_text().contains("Dirty path count: 1"));
    assert!(report
        .to_json()
        .contains("database/objects/tables/nested folder/customer email Δ.sql"));
}

#[test]
fn slice35_scoped_write_guard_rejects_unsafe_target_paths() {
    let dir = create_temp_dir("slice35-unsafe-targets");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let guard = scoped_write_guard(
        &dir,
        Some("feature/dbstate-test"),
        None,
        &[
            "../outside.sql".to_string(),
            "/absolute/outside.sql".to_string(),
            "C:/outside.sql".to_string(),
        ],
        GitHandoffWorkflow::SchemaExport,
        "unsafe target paths",
    );

    assert!(!guard.allowed);
    assert_eq!(guard.errors.len(), 3);
    assert!(guard
        .errors
        .iter()
        .all(|error| error.contains("target path is not safe")));
}

#[test]
fn slice35_suggested_branch_names_include_workflow_timestamp_id_and_slug() {
    let branch = suggested_branch_name(
        GitHandoffWorkflow::ReferenceDataExport,
        Some("20260717-143240"),
        Some("c2a8f4"),
        "public country",
    );

    assert_eq!(
        branch,
        "dbstate/reference-data-export/20260717-143240-c2a8f4-public-country"
    );
}

#[test]
fn slice35_commit_message_generation_per_workflow() {
    assert_eq!(
        GitHandoffWorkflow::SchemaRelease.commit_title(),
        "review: generate schema release artifacts"
    );
    assert!(GitHandoffWorkflow::SchemaRelease
        .commit_body()
        .contains("database/releases/objects/"));
    assert!(GitHandoffWorkflow::SchemaRelease
        .commit_body()
        .contains("No SQL was executed"));

    assert_eq!(
        GitHandoffWorkflow::SchemaExport.commit_title(),
        "sync: update schema objects from PostgreSQL"
    );
    assert!(GitHandoffWorkflow::SchemaExport
        .commit_body()
        .contains("No database changes were applied"));

    assert_eq!(
        GitHandoffWorkflow::ReferenceDataReview.commit_title(),
        "review: generate reference-data review script"
    );
    let reference_review = GitHandoffWorkflow::ReferenceDataReview.commit_body();
    assert!(reference_review.contains("INSERT candidates"));
    assert!(reference_review.contains("UPDATE candidates"));
    assert!(reference_review.contains("No DELETE statements"));
    assert!(reference_review.contains("No data was applied"));

    assert_eq!(
        GitHandoffWorkflow::ReferenceDataExport.commit_title(),
        "sync: export reference data from PostgreSQL"
    );
    assert!(GitHandoffWorkflow::ReferenceDataExport
        .commit_body()
        .contains("database/reference-data"));
}

#[test]
fn slice35_read_only_actions_are_allowed_on_protected_branch() {
    let dir = create_temp_dir("slice35-readonly-protected");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    set_git_head_branch(&dir, "dev");
    fs::write(dir.join("dirty.txt"), "dirty").expect("dirty unrelated file");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.branch.as_deref(), Some("dev"));
    assert!(report.is_dirty);
}

#[test]
fn slice35_git_handoff_ui_contract_is_present() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "data-testid=\"tab-git-workflow\"",
        "data-testid=\"workspace-git-workflow-pointer\"",
        "data-testid=\"git-workflow-panel\"",
        "data-testid=\"git-handoff-panel\"",
        "data-testid=\"git-workflow-status\"",
        "data-testid=\"git-workflow-ready-state\"",
        "data-testid=\"git-workflow-blocked-state\"",
        "data-testid=\"git-workflow-no-changes-state\"",
        "data-testid=\"git-workflow-loading-state\"",
        "data-testid=\"git-workflow-error-state\"",
        "Open Git Workflow",
        "Current branch",
        "Protected branch",
        "DbState Intended Write Paths",
        "DbState Written/Generated Paths",
        "Recommended Branch",
        "Suggested Manual Git Commands",
        "Suggested Commit Title",
        "Suggested Commit Body",
        "data-testid=\"git-workflow-suggested-branch\"",
        "data-testid=\"git-workflow-suggested-branch-name\"",
        "data-testid=\"git-workflow-copy-branch-name\"",
        "data-testid=\"git-workflow-branch-reason\"",
        "data-testid=\"git-workflow-suggested-files\"",
        "data-testid=\"git-workflow-suggested-files-count\"",
        "data-testid=\"git-workflow-suggested-files-empty\"",
        "data-testid=\"git-workflow-suggested-commit-title\"",
        "data-testid=\"git-workflow-suggested-commit-body\"",
        "data-testid=\"git-workflow-copy-commit-title\"",
        "data-testid=\"git-workflow-copy-commit-message\"",
        "data-testid=\"copy-recommended-branch\"",
        "data-testid=\"copy-manual-git-commands\"",
        "data-testid=\"copy-commit-title\"",
        "data-testid=\"copy-commit-body\"",
        "data-testid=\"copy-full-commit-command\"",
        "data-testid=\"git-handoff-full-commit-command\"",
        "Suggested Full Commit Command - PowerShell",
        "Suggested Pull Request",
        "Suggested PR Title",
        "Suggested PR Body",
        "data-testid=\"copy-pr-title\"",
        "data-testid=\"copy-pr-body\"",
        "data-testid=\"git-handoff-pr-title\"",
        "data-testid=\"git-handoff-pr-body\"",
        "GitHub may not automatically copy the commit body into the Pull Request description.",
        "Generate / Regenerate Commit Message",
        "\"git switch -c \" + branchName",
        "state.gitHandoffBranchCommand",
        "git status --short",
        "git add ",
        "git-handoff-large-add-note",
        "manualGitCommandsForPaths",
        "managedGitAddFolder",
        "values.length <= 20",
        "database/objects/",
        "database/reference-data/",
        "database/releases/objects/",
        "database/releases/reference-data/",
        "Large change set detected. DbState grouped git add commands by managed folder. Review git status before committing.",
        "git commit -m $commitTitle -m $commitBody",
        "normalizeCommitBodyForPowerShell",
        "powerShellCommitBodyExpression",
        "powerShellFullCommitCommand",
        "suggestedPullRequestBody",
        "updateSuggestedPullRequest",
        "isPreWriteBranchGuidance",
        "pullRequestReviewWarnings",
        "pullRequestFileSection",
        "pullRequestChangeSummary",
        "Large file list summarized by category/count",
        "git-handoff-large-add-note",
        "manualGitCommandsForPaths",
        "managedGitAddFolder",
        "values.length <= 20",
        "Large change set detected. DbState grouped git add commands by managed folder. Review git status before committing.",
        "database/objects/",
        "database/reference-data/",
        "database/releases/objects/",
        "database/releases/reference-data/",
        "DbState did not run SQL, apply database changes, mutate PostgreSQL, or run Git commands.",
        "$commitBody = \" + powerShellCommitBodyExpression(body)",
        ") -join [Environment]::NewLine",
        "review: generate schema release artifacts",
        "sync: update schema objects from PostgreSQL",
        "review: generate reference-data review script",
        "sync: export reference data from PostgreSQL",
        "DbState never runs git add, commit, push, pull, fetch, tag, switch, checkout, or branch commands.",
        "Already on working branch: ",
        "writeSucceededOnWorkingBranch",
        "renderGitWorkflowSuggestedFiles(suggestedPaths);",
        "setGitWorkflowState(\"blocked\"",
        "setGitWorkflowState(\"ready\"",
        "setGitWorkflowState(\"no-changes\"",
    ] {
        assert!(
            combined.contains(expected),
            "missing Git Handoff UI contract {expected}"
        );
    }

    assert!(!combined.contains("data-testid=\"schema-database-to-repository-git-file-lists\""));
    assert!(!combined.contains("DbState Intended Objects to be Written"));
    assert!(!combined.contains("DbState Written / Generated Files"));
    assert!(!combined.contains("repository-sync-intended-objects"));
    assert!(!combined.contains("repository-sync-written-files"));
    assert!(!js.contains("renderRepositorySyncGitLists(data);"));
    assert!(js.contains("Repository files written. Review the generated files and Git handoff details below before committing manually."));
    assert!(js.contains("writeGenerateSuccessOpensGitWorkflow(label, data)"));
    assert!(js.contains("openGitWorkflowAfterWrite(label, data);"));
    assert!(js.contains("showStep(\"git-workflow\");"));
    assert!(js.contains("\"Database to Repository Write\""));
    assert!(js.contains("\"Reference-data YAML write\""));
    assert!(js.contains("\"Generate Release Artifact\""));
    assert!(js.contains("\"Reference-data review script write\""));
    assert!(js.contains("\"Initialize DbState Project\""));
    assert!(js.contains(
        "const existingTitle = textOrEmpty(byId(\"git-handoff-commit-title-text\").textContent);"
    ));
    assert!(js.contains(
        "const existingBody = textOrEmpty(byId(\"git-handoff-commit-body\").textContent);"
    ));
    assert!(js.contains("existingTitle && existingTitle !== titlePlaceholder ? existingTitle"));
    assert!(js.contains("existingBody && existingBody !== bodyPlaceholder ? existingBody"));
    assert!(js.contains("Written/generated files"));
    assert!(js.contains("Intended write paths"));
    assert!(js.contains("Change summary: "));
    assert!(js.contains("const filePaths = written.length ? written : intended;"));

    let commands_start = js
        .find("function managedGitAddFolder")
        .expect("manual command helper exists");
    let commands_end = js[commands_start..]
        .find("function pathCategory")
        .map(|index| commands_start + index)
        .expect("manual command block end exists");
    let commands_block = &js[commands_start..commands_end];
    assert!(commands_block.contains("values.length <= 20"));
    assert!(commands_block.contains("\"git add \" + values.join(\" \")"));
    assert!(commands_block.contains("\"git add \" + group"));
    assert!(commands_block.contains("database/objects/"));
    assert!(commands_block.contains("database/reference-data/"));
    assert!(commands_block.contains("database/releases/objects/"));
    assert!(commands_block.contains("database/releases/reference-data/"));
    assert!(!commands_block.contains("git switch"));
    assert!(!commands_block.contains("git commit"));
    assert!(!commands_block.contains("git branch"));
    assert!(!commands_block.contains("git push"));
    assert!(!commands_block.contains("git pull"));
    assert!(!commands_block.contains("git fetch"));
    assert!(!commands_block.contains("git tag"));

    let branch_start = js
        .find("let branchCommand = \"git switch -c \" + branchName;")
        .expect("recommended branch block exists");
    let branch_block = &js[branch_start..branch_start + 500.min(js.len() - branch_start)];
    assert!(branch_block.contains("git switch -c"));
    assert!(js.contains("byId(\"git-handoff-recommended-branch\").textContent = branchDisplay;"));
    assert!(js.contains(
        "byId(\"git-workflow-suggested-branch-name\").textContent = suggestedBranchName;"
    ));
    assert!(
        !js.contains("byId(\"git-workflow-suggested-branch-name\").textContent = branchCommand")
    );
    assert!(js.contains("data && data.success === true && written.length && !protectedBranch"));
    assert!(js.contains("byId(\"git-handoff-large-add-note\").hidden = !manualCommands.grouped;"));
    let full_commit_start = js
        .find("function updateSuggestedFullCommitCommand()")
        .expect("full commit command function exists");
    let full_commit_block =
        &js[full_commit_start..full_commit_start + 900.min(js.len() - full_commit_start)];
    assert!(full_commit_block.contains("powerShellFullCommitCommand(title, body)"));
    assert!(full_commit_block.contains("git-handoff-commit-title-text"));
    assert!(full_commit_block.contains("git-handoff-commit-body"));
    assert!(full_commit_block.contains("git-handoff-full-commit-command"));
    assert!(!full_commit_block.contains("replace(/\\r?\\n/g, \"\\\\n\")"));
    let pr_start = js
        .find("function suggestedPullRequestBody")
        .expect("suggested PR body function exists");
    let pr_block = &js[pr_start..pr_start + 1700.min(js.len() - pr_start)];
    assert!(pr_block.contains("## Summary"));
    assert!(pr_block.contains("## Written/generated files or changed files"));
    assert!(pr_block.contains("## Change summary"));
    assert!(pr_block.contains("## Safety statement"));
    assert!(pr_block.contains("## Review notes/warnings"));
    assert!(pr_block.contains("const warnings = pullRequestReviewWarnings(data || {}, filePaths);"));
    assert!(!pr_block.contains("Array.isArray(data && data.warnings)"));
    assert!(js.contains("suggestedPullRequestBody(safeTitle, safeBody"));
    let pr_filter_start = js
        .find("function isPreWriteBranchGuidance")
        .expect("PR warning filter exists");
    let pr_filter_end = js[pr_filter_start..]
        .find("function suggestedPullRequestBody")
        .map(|index| pr_filter_start + index)
        .expect("PR warning filter block ends");
    let pr_filter_block = &js[pr_filter_start..pr_filter_end];
    assert!(pr_filter_block.contains("git switch -c"));
    assert!(pr_filter_block.contains("suggested manual branch command"));
    assert!(pr_filter_block.contains("branch command before writing files"));
    assert!(pr_filter_block.contains("create branch first"));
    assert!(pr_filter_block.contains("!isPreWriteBranchGuidance(warning)"));
    assert!(pr_filter_block
        .contains("Large file list summarized by category/count; review Git diff before merge."));
}

#[test]
fn slice36_commit_message_outside_git_returns_safe_error() {
    let dir = create_temp_dir("slice36-outside-git");

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 2);
    assert!(output.contains("Current path is not inside a Git repository."));
    assert!(!output.contains("postgres://"));
}

#[test]
fn slice36_commit_message_no_staged_files_returns_clear_error() {
    let dir = create_temp_dir("slice36-no-staged");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 2);
    assert!(output.contains("No staged files were found"));
    assert!(!output.contains("Suggested Commit Title"));
}

#[test]
fn slice36_unstaged_dbstate_files_are_not_included() {
    let dir = create_temp_dir("slice36-unstaged");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let table = dir.join("database/objects/tables/public.customers.sql");
    fs::write(
        &table,
        "CREATE TABLE public.customers (\n  customer_id integer\n);\n",
    )
    .expect("write table");

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 2);
    assert!(output.contains("No staged files were found"));
    assert!(!output.contains("public.customers"));
}

#[test]
fn slice36_staged_table_addition_generates_semantic_message() {
    let dir = create_temp_dir("slice36-table-add");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let relative = "database/objects/tables/public.customers.sql";
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (\n  customer_id integer,\n  email text\n);\n",
    )
    .expect("write table");
    stage_path(&dir, relative);

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 0);
    assert!(output.contains("Suggested Commit Title"));
    assert!(output.contains("feat(public): add table public.customers"));
    assert!(output.contains("Technical changes:"));
    assert!(output.contains("- add table public.customers"));
    assert!(output.contains("Analyzed DbState files"));
    assert!(output.contains(relative));
}

#[test]
fn slice36_plain_style_generates_plain_title() {
    let dir = create_temp_dir("slice36-plain");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let relative = "database/objects/tables/public.customers.sql";
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    stage_path(&dir, relative);

    let (exit_code, output) = run_commit_message_text(&dir, &["--style", "plain"]);

    assert_eq!(exit_code, 0);
    assert!(output.contains("Add table public.customers"));
    assert!(!output.contains("feat("));
}

#[test]
fn slice36_intent_changes_title_not_technical_body() {
    let dir = create_temp_dir("slice36-intent");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let relative = "database/objects/tables/public.customers.sql";
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    stage_path(&dir, relative);

    let (_, default_output) = run_commit_message_text(&dir, &[]);
    let (_, intent_output) =
        run_commit_message_text(&dir, &["--intent", "Support customer email verification"]);

    assert!(intent_output.contains("feat(public): support customer email verification"));
    assert!(intent_output.contains("Support customer email verification"));
    assert!(default_output.contains("- add table public.customers"));
    assert!(intent_output.contains("- add table public.customers"));
}

#[test]
fn slice36_branch_ticket_inference_adds_refs_line() {
    let dir = create_temp_dir("slice36-ticket");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    set_git_head_branch(&dir, "feature/DB-184-customer-email");
    let relative = "database/objects/tables/public.customers.sql";
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    stage_path(&dir, relative);

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 0);
    assert!(output.contains("Refs: DB-184"));
}

#[test]
fn slice36_mixed_staged_files_warns_and_lists_ignored_files() {
    let dir = create_temp_dir("slice36-mixed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let table = "database/objects/tables/public.customers.sql";
    let app = "app/config.txt";
    fs::create_dir_all(dir.join("app")).expect("create app");
    fs::write(
        dir.join(table),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    fs::write(dir.join(app), "feature flag\n").expect("write app file");
    stage_path(&dir, table);
    stage_path(&dir, app);

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 0);
    assert!(output.contains("Ignored non-DbState staged files"));
    assert!(output.contains(app));
    assert!(output.contains("Ignored 1 staged non-DbState file(s)."));
}

#[test]
fn slice36_table_modification_with_added_column_is_described() {
    let dir = create_temp_dir("slice36-add-column");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let relative = "database/objects/tables/public.customers.sql";
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (\n  customer_id integer\n);\n",
    )
    .expect("write table");
    commit_all(&dir, "add table");
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (\n  customer_id integer,\n  email text\n);\n",
    )
    .expect("update table");
    stage_path(&dir, relative);

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 0);
    assert!(output.contains("add column email to table public.customers"));
}

#[test]
fn slice36_deleted_and_renamed_dbstate_objects_are_breaking() {
    let delete_dir = create_temp_dir("slice36-delete");
    init_git_repo(&delete_dir);
    create_complete_structure(&delete_dir);
    let table = "database/objects/tables/public.customers.sql";
    fs::write(
        delete_dir.join(table),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    commit_all(&delete_dir, "add table");
    fs::remove_file(delete_dir.join(table)).expect("delete table");
    stage_path(&delete_dir, table);

    let (delete_exit, delete_output) = run_commit_message_text(&delete_dir, &[]);

    assert_eq!(delete_exit, 0);
    assert!(delete_output.contains("BREAKING CHANGE:"));
    assert!(delete_output.contains("removes table public.customers"));

    let rename_dir = create_temp_dir("slice36-rename");
    init_git_repo(&rename_dir);
    create_complete_structure(&rename_dir);
    fs::write(
        rename_dir.join(table),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    commit_all(&rename_dir, "add table");
    let renamed = "database/objects/tables/public.accounts.sql";
    let rename = Command::new("git")
        .args(["mv", table, renamed])
        .current_dir(&rename_dir)
        .output()
        .expect("git mv table");
    assert!(rename.status.success(), "git mv failed");

    let (rename_exit, rename_output) = run_commit_message_text(&rename_dir, &[]);

    assert_eq!(rename_exit, 0);
    assert!(rename_output.contains("BREAKING CHANGE:"));
    assert!(rename_output.contains("rename table public.customers to public.accounts"));
}

#[test]
fn slice36_semantic_types_cover_index_function_reference_data_and_release_artifacts() {
    let dir = create_temp_dir("slice36-types");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let domain = "database/objects/domains/public.email_domain.sql";
    let aggregate = "database/objects/aggregates/public.group_concat.text.sql";
    let index = "database/objects/indexes/public.customers_email_idx.sql";
    let function = "database/objects/functions/public.normalize_email.email_text.sql";
    let reference = "database/reference-data/tables/public.country.yml";
    let release = "database/releases/reference-data/0001_country.reference-data.sql";
    fs::write(
        dir.join(domain),
        "CREATE DOMAIN \"public\".\"email_domain\" AS text;\n",
    )
    .expect("write domain");
    fs::write(
        dir.join(aggregate),
        "CREATE AGGREGATE \"public\".\"group_concat\"(text) (SFUNC = \"public\".\"_group_concat\", STYPE = text);\n",
    )
    .expect("write aggregate");
    fs::write(
        dir.join(index),
        "CREATE INDEX customers_email_idx ON public.customers (email);\n",
    )
    .expect("write index");
    fs::write(
        dir.join(function),
        "CREATE OR REPLACE FUNCTION public.normalize_email(email text) RETURNS text LANGUAGE sql AS $$ SELECT lower(email) $$;\n",
    )
    .expect("write function");
    fs::create_dir_all(dir.join("database/reference-data/tables")).expect("reference data tables");
    fs::write(dir.join(reference), "rows:\n  - country_id: 1\n").expect("write reference data");
    fs::write(dir.join(release), "-- REVIEW ONLY.\n").expect("write release artifact");
    for relative in [domain, aggregate, index, function, reference, release] {
        stage_path(&dir, relative);
    }

    let (exit_code, output) = run_commit_message_text(&dir, &[]);

    assert_eq!(exit_code, 0);
    assert!(output.contains("perf(database):"));
    assert!(output.contains("add domain public.email_domain"));
    assert!(output.contains("add aggregate public.group_concat.text"));
    assert!(output.contains("add index public.customers_email_idx"));
    assert!(output.contains("add function public.normalize_email.email_text"));
    assert!(output.contains("add reference data public.country"));
    assert!(output.contains("add reference-data review artifact 0001_country"));
}

#[test]
fn slice36_regenerating_after_staged_set_changes_updates_message() {
    let dir = create_temp_dir("slice36-regenerate");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let table = "database/objects/tables/public.customers.sql";
    let index = "database/objects/indexes/public.customers_email_idx.sql";
    fs::write(
        dir.join(table),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    stage_path(&dir, table);
    let (_, first) = run_commit_message_text(&dir, &[]);
    fs::write(
        dir.join(index),
        "CREATE INDEX customers_email_idx ON public.customers (email);\n",
    )
    .expect("write index");
    stage_path(&dir, index);
    let (_, second) = run_commit_message_text(&dir, &[]);

    assert!(first.contains("add table public.customers"));
    assert!(!first.contains("customers_email_idx"));
    assert!(second.contains("customers_email_idx"));
    assert_ne!(first, second);
}

#[test]
fn slice36_json_output_has_required_fields_and_no_secret_content() {
    let dir = create_temp_dir("slice36-json");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let relative = "database/objects/tables/public.customers.sql";
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (\n  url text DEFAULT 'postgres://user:password@example.invalid/db'\n);\n",
    )
    .expect("write table");
    stage_path(&dir, relative);

    let (exit_code, output) = run_commit_message_text(&dir, &["--json"]);

    assert_eq!(exit_code, 0);
    for field in [
        "\"command\":\"commit-message\"",
        "\"success\":true",
        "\"scope\":\"staged\"",
        "\"style\":\"conventional\"",
        "\"branch\":",
        "\"ticket\":",
        "\"title\":",
        "\"body\":",
        "\"message\":",
        "\"breakingChange\":",
        "\"stagedFiles\":",
        "\"analyzedFiles\":",
        "\"ignoredFiles\":",
        "\"warnings\":",
        "\"errors\":",
    ] {
        assert!(output.contains(field), "missing JSON field marker {field}");
    }
    assert!(!output.contains("postgres://"));
    assert!(!output.contains("password@example"));
}

#[test]
fn slice36_commit_message_service_endpoint_is_read_only_and_json() {
    let dir = create_temp_dir("slice36-service");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let relative = "database/objects/tables/public.customers.sql";
    fs::write(
        dir.join(relative),
        "CREATE TABLE public.customers (id integer);\n",
    )
    .expect("write table");
    stage_path(&dir, relative);
    let body = format!(
        r#"{{ "repositoryPath": "{}" }}"#,
        escape_json(&display_path(&dir))
    );

    let response = service_response("POST", "/api/v1/git/commit-message", &body, &dir);

    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("\"command\":\"commit-message\""));
    assert!(response.body.contains("add table public.customers"));
}

#[test]
fn slice36_git_workflow_ui_uses_staged_commit_message_endpoint() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "Commit message is generated from staged changes.",
        "No staged changes found. Stage reviewed DbState paths manually, then regenerate.",
        "data-testid=\"copy-full-commit-message\"",
        "data-testid=\"copy-full-commit-command\"",
        "data-testid=\"git-handoff-full-commit-command\"",
        "Suggested Full Commit Command - PowerShell",
        "Suggested Pull Request",
        "Suggested PR Title",
        "Suggested PR Body",
        "data-testid=\"copy-pr-title\"",
        "data-testid=\"copy-pr-body\"",
        "data-testid=\"git-handoff-pr-title\"",
        "data-testid=\"git-handoff-pr-body\"",
        "GitHub may not automatically copy the commit body into the Pull Request description.",
        "data-testid=\"git-handoff-staged-change-summary\"",
        "commitMessage: \"/api/v1/git/commit-message\"",
        "regenerateCommitMessageFromStagedChanges",
        "approvedEndpoints.commitMessage",
        "Analyzed DbState files",
        "Ignored non-DbState staged files",
        "updateSuggestedFullCommitCommand",
        "normalizeCommitBodyForPowerShell",
        "powerShellDoubleQuoted",
        "powerShellSingleQuoted",
        "powerShellCommitBodyExpression",
        "powerShellFullCommitCommand",
        "suggestedPullRequestBody",
        "updateSuggestedPullRequest",
        "isPreWriteBranchGuidance",
        "pullRequestReviewWarnings",
        "pullRequestFileSection",
        "pullRequestChangeSummary",
        "Large file list summarized by category/count",
        "DbState did not run SQL, apply database changes, mutate PostgreSQL, or run Git commands.",
        "## Summary",
        "## Written/generated files or changed files",
        "## Change summary",
        "## Safety statement",
        "## Review notes/warnings",
        "$commitTitle = ",
        "$commitBody = \" + powerShellCommitBodyExpression(body)",
        "git commit -m $commitTitle -m $commitBody",
        "return \"@(\\r\\n\" + lines.map(powerShellSingleQuoted).join(\"\\r\\n\") + \"\\r\\n) -join [Environment]::NewLine\";",
        "writeGenerateSuccessOpensGitWorkflow",
        "data.success !== true",
        "Repository files written. Review the generated files and Git handoff details below before committing manually.",
        "Already on working branch: ",
        "writeSucceededOnWorkingBranch",
    ] {
        assert!(
            combined.contains(expected),
            "missing Slice 36 UI contract {expected}"
        );
    }
    assert!(js.contains("style: \"conventional\""));
    assert!(!js.contains("git add ."));
    let commands_start = js
        .find("function managedGitAddFolder")
        .expect("manual commands helper exists");
    let commands_end = js[commands_start..]
        .find("function pathCategory")
        .map(|index| commands_start + index)
        .expect("manual commands block ends");
    let commands_block = &js[commands_start..commands_end];
    assert!(commands_block.contains("values.length <= 20"));
    assert!(commands_block.contains("\"git add \" + values.join(\" \")"));
    assert!(commands_block.contains("\"git add \" + group"));
    assert!(commands_block.contains("database/objects/"));
    assert!(commands_block.contains("database/reference-data/"));
    assert!(commands_block.contains("database/releases/objects/"));
    assert!(commands_block.contains("database/releases/reference-data/"));
    assert!(!commands_block.contains("git commit"));
    assert!(!commands_block.contains("git switch"));
    assert!(!js.contains("replace(/\\r?\\n/g, \"\\\\n\")"));
    assert!(js.contains("return \"@'\\r\\n\" + normalized + \"\\r\\n'@\";"));
    assert!(js.contains("const raw = textOrEmpty(value).replace(/\\\\n/g, \"\\n\")"));
    assert!(js.contains("line.replace(/[ \\t]+$/g, \"\")"));
    assert!(js.contains("previousBlank"));
    assert!(js.contains("lines.some(function (line) { return line === \"'@\"; })"));
    assert!(js.contains("values.length <= 12"));
    assert!(js.contains("pathCategory(path)"));
    assert!(js.contains("prBodyTarget.textContent = suggestedPullRequestBody"));
    assert!(js.contains("!isPreWriteBranchGuidance(warning)"));
    assert!(js.contains("suggested manual branch command"));
    assert!(js.contains("branch command before writing files"));
    assert!(js.contains("create branch first"));
    assert!(js.contains("data && data.success === true && written.length && !protectedBranch"));
    assert!(!js.contains("gh pr create"));
    assert!(!js.contains("api.github.com"));
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
    assert!(report
        .created_files
        .contains(&"database/objects/domains/dbstate_slice2.account_year.sql".to_string()));
    assert!(report.created_files.contains(
        &"database/objects/aggregates/dbstate_slice2.account_labels.text.sql".to_string()
    ));
    assert!(report.created_files.contains(
        &"database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"
            .to_string()
    ));
    assert!(report.created_files.contains(
        &"database/objects/functions/dbstate_slice2.account_label.account_code_text.sql"
            .to_string()
    ));
    assert!(report.created_files.contains(
        &"database/objects/materialized-views/dbstate_slice2.account_summary.sql".to_string()
    ));
    assert!(report.created_files.contains(
        &"database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql"
            .to_string()
    ));
    assert!(report.created_files.contains(
        &"database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql".to_string()
    ));
    assert!(report.created_files.contains(
        &"database/objects/grants/functions/dbstate_slice2.account_label.account_id_integer.app_reader.sql"
            .to_string()
    ));
    assert!(report.created_files.contains(
        &"database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_public_read.sql"
            .to_string()
    ));
    assert!(dir
        .join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql",
        )
        .is_file());
    assert!(dir
        .join("database/objects/materialized-views/dbstate_slice2.account_summary.sql")
        .is_file());
    assert!(dir
        .join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql")
        .is_file());
}

#[test]
fn sync_creates_partitioned_parent_table_file() {
    let dir = create_temp_dir("sync-partitioned-parent");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let report = sync_postgres_with_inventory(
        &dir,
        &partitioned_payment_inventory(),
        &ExportSelection::Table {
            schema: "public".to_string(),
            table: "payment".to_string(),
        },
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert!(report
        .created_files
        .contains(&"database/objects/tables/public.payment.sql".to_string()));
    let table_sql =
        fs::read_to_string(dir.join("database/objects/tables/public.payment.sql")).expect("table");
    assert!(table_sql.contains("PARTITION BY RANGE (payment_date);"));
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
    assert!(dir.join("database/releases/objects").is_dir());
    assert!(dir.join("database/releases/reference-data").is_dir());
    assert!(fs::read_dir(dir.join("database/releases/objects"))
        .expect("read object releases")
        .next()
        .is_none());
    assert!(fs::read_dir(dir.join("database/releases/reference-data"))
        .expect("read reference-data releases")
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
fn compare_classifies_grant_objects() {
    let dir = create_temp_dir("compare-grants");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql"),
        render_grant_sql(&sample_inventory().grants[1]),
    )
    .expect("write in-sync grant");
    fs::write(
        dir.join("database/objects/grants/schemas/dbstate_slice2.app_reader.sql"),
        "-- changed grant\nGRANT CREATE ON SCHEMA \"dbstate_slice2\" TO \"app_reader\";\n",
    )
    .expect("write different grant");
    fs::write(
        dir.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.repo_only.sql"),
        "GRANT SELECT ON TABLE \"dbstate_slice2\".\"sample_accounts\" TO \"repo_only\";\n",
    )
    .expect("write repo-only grant");
    commit_all(&dir, "grant compare files");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success, "{:?}", report.errors);
    assert!(report.in_sync.contains(
        &"database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql".to_string()
    ));
    assert!(report
        .repo_different
        .contains(&"database/objects/grants/schemas/dbstate_slice2.app_reader.sql".to_string()));
    assert!(report.repo_only.contains(
        &"database/objects/grants/tables/dbstate_slice2.sample_accounts.repo_only.sql".to_string()
    ));
    assert!(report.database_only.contains(
        &"database/objects/grants/functions/dbstate_slice2.account_label.account_id_integer.app_reader.sql"
            .to_string()
    ));
}

#[test]
fn compare_classifies_rls_policy_objects() {
    let dir = create_temp_dir("compare-rls-policies");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_public_read.sql"),
        render_rls_policy_sql(&sample_inventory().rls_policies[0]),
    )
    .expect("write in-sync policy");
    fs::write(
        dir.join("database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_insert_check.sql"),
        "-- changed policy\nCREATE POLICY \"sample_accounts_insert_check\" ON \"dbstate_slice2\".\"sample_accounts\" AS PERMISSIVE FOR INSERT TO \"app_writer\" WITH CHECK (false);\n",
    )
    .expect("write different policy");
    fs::write(
        dir.join(
            "database/objects/rls-policies/dbstate_slice2.sample_accounts.repo_only_policy.sql",
        ),
        render_rls_policy_sql(&RlsPolicyInfo {
            policy_name: "repo_only_policy".to_string(),
            ..sample_inventory().rls_policies[0].clone()
        }),
    )
    .expect("write repo-only policy");
    commit_all(&dir, "RLS policy compare files");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success, "{:?}", report.errors);
    assert!(report.in_sync.contains(
        &"database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_public_read.sql".to_string()
    ));
    assert!(report.repo_different.contains(
        &"database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_insert_check.sql".to_string()
    ));
    assert!(report.repo_only.contains(
        &"database/objects/rls-policies/dbstate_slice2.sample_accounts.repo_only_policy.sql"
            .to_string()
    ));
    assert!(report.database_only.contains(
        &"database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_restrictive_update.sql".to_string()
    ));
}

#[test]
fn compare_classifies_partitioned_parent_tables() {
    let table_path = "database/objects/tables/public.payment.sql";
    let inventory = partitioned_payment_inventory();
    let partitioned_sql = render_table_sql_with_partition(
        "public",
        "payment",
        &inventory.columns,
        inventory.tables[0].partition_key.as_deref(),
    );

    let repo_only_dir = create_temp_dir("compare-partitioned-repo-only");
    init_git_repo(&repo_only_dir);
    create_complete_structure(&repo_only_dir);
    fs::write(
        repo_only_dir.join("database/objects/schemas/public.sql"),
        render_schema_sql("public"),
    )
    .expect("write schema");
    fs::write(repo_only_dir.join(table_path), &partitioned_sql).expect("write table");
    commit_all(&repo_only_dir, "repo-only partitioned table");
    let mut target_without_payment = inventory.clone();
    target_without_payment.tables.clear();
    target_without_payment.columns.clear();
    target_without_payment.constraints.clear();
    let repo_only = compare_postgres_with_inventory(
        &repo_only_dir,
        &target_without_payment,
        &ExportSelection::All,
    );
    assert!(repo_only.success, "{:?}", repo_only.errors);
    assert!(repo_only.repo_only.contains(&table_path.to_string()));

    let database_only_dir = create_temp_dir("compare-partitioned-database-only");
    init_git_repo(&database_only_dir);
    create_complete_structure(&database_only_dir);
    fs::write(
        database_only_dir.join("database/objects/schemas/public.sql"),
        render_schema_sql("public"),
    )
    .expect("write schema");
    commit_all(&database_only_dir, "schema only");
    let database_only =
        compare_postgres_with_inventory(&database_only_dir, &inventory, &ExportSelection::All);
    assert!(database_only.success, "{:?}", database_only.errors);
    assert!(database_only
        .database_only
        .contains(&table_path.to_string()));

    let changed_dir = create_temp_dir("compare-partitioned-changed");
    init_git_repo(&changed_dir);
    create_complete_structure(&changed_dir);
    fs::write(
        changed_dir.join("database/objects/schemas/public.sql"),
        render_schema_sql("public"),
    )
    .expect("write schema");
    fs::write(
        changed_dir.join(table_path),
        partitioned_sql.replace("RANGE (payment_date)", "LIST (payment_id)"),
    )
    .expect("write changed table");
    commit_all(&changed_dir, "changed partitioned table");
    let changed = compare_postgres_with_inventory(&changed_dir, &inventory, &ExportSelection::All);
    assert!(changed.success, "{:?}", changed.errors);
    assert!(changed.repo_different.contains(&table_path.to_string()));
}

#[test]
fn repository_discovery_finds_rls_policy_files_and_skips_invalid_names() {
    let dir = create_temp_dir("discover-rls-policies");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/rls-policies/dbstate_slice2.sample_accounts.policy_one.sql"),
        render_rls_policy_sql(&RlsPolicyInfo {
            policy_name: "policy_one".to_string(),
            ..sample_inventory().rls_policies[0].clone()
        }),
    )
    .expect("write first policy");
    fs::write(
        dir.join("database/objects/rls-policies/dbstate_slice2.sample_accounts.policy_two.sql"),
        render_rls_policy_sql(&RlsPolicyInfo {
            policy_name: "policy_two".to_string(),
            ..sample_inventory().rls_policies[0].clone()
        }),
    )
    .expect("write second policy");
    fs::write(
        dir.join("database/objects/rls-policies/bad.name.sql"),
        "-- bad policy\n",
    )
    .expect("write bad policy");

    let import = discover_repository_objects(&dir).expect("discover repository objects");

    assert!(import
        .objects
        .contains_key("rlsPolicy:dbstate_slice2.sample_accounts.policy_one"));
    assert!(import
        .objects
        .contains_key("rlsPolicy:dbstate_slice2.sample_accounts.policy_two"));
    assert!(import
        .skipped
        .contains(&"database/objects/rls-policies/bad.name.sql".to_string()));
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
    fs::write(
        dir.join("database/objects/domains/dbstate_slice2.account_year.sql"),
        render_domain_sql(&sample_inventory().domains[0]),
    )
    .expect("write domain");
    fs::write(
        dir.join("database/objects/aggregates/dbstate_slice2.account_labels.text.sql"),
        render_aggregate_sql(&sample_inventory().aggregates[0]),
    )
    .expect("write aggregate");
    fs::write(
        dir.join("database/objects/aggregates/dbstate_slice2.account_labels.text.sql"),
        render_aggregate_sql(&sample_inventory().aggregates[0]),
    )
    .expect("write aggregate");
    fs::write(
        dir.join(
            "database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql",
        ),
        render_constraint_sql(&sample_inventory().constraints[0]),
    )
    .expect("write primary key constraint");
    fs::write(
        dir.join(
            "database/objects/constraints/unique-constraints/dbstate_slice2.sample_accounts.sample_accounts_code_key.sql",
        ),
        render_constraint_sql(&sample_inventory().constraints[1]),
    )
    .expect("write unique constraint");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        render_function_sql(&sample_inventory().functions[0]),
    )
    .expect("write function");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_code_text.sql"),
        render_function_sql(&sample_inventory().functions[1]),
    )
    .expect("write overloaded function");
    fs::write(
        dir.join("database/objects/materialized-views/dbstate_slice2.account_summary.sql"),
        render_materialized_view_sql(&sample_inventory().materialized_views[0]),
    )
    .expect("write materialized view");
    fs::write(
        dir.join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql",
        ),
        render_trigger_sql(&sample_inventory().triggers[0]),
    )
    .expect("write trigger");
    fs::write(
        dir.join("database/objects/triggers/dbstate_slice2.other_accounts.sample_accounts_audit_trigger.sql"),
        "CREATE TRIGGER sample_accounts_audit_trigger AFTER INSERT ON dbstate_slice2.other_accounts FOR EACH ROW EXECUTE FUNCTION dbstate_slice2.account_label(account_id);\n",
    )
    .expect("write same trigger name on different relation");
    fs::write(
        dir.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql"),
        render_grant_sql(&sample_inventory().grants[1]),
    )
    .expect("write public table grant");
    fs::write(
        dir.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.app_reader.sql"),
        "GRANT SELECT ON TABLE \"dbstate_slice2\".\"sample_accounts\" TO \"app_reader\";\n",
    )
    .expect("write second table grant");
    fs::write(
        dir.join("database/objects/grants/functions/dbstate_slice2.account_label.account_id_integer.app_reader.sql"),
        render_grant_sql(&sample_inventory().grants[3]),
    )
    .expect("write function grant");

    let import = discover_repository_objects(&dir).expect("discover objects");

    assert!(import.objects.contains_key("schema:dbstate_slice2"));
    assert!(import
        .objects
        .contains_key("table:dbstate_slice2.sample_accounts"));
    assert!(import
        .objects
        .contains_key("domain:dbstate_slice2.account_year"));
    assert!(import
        .objects
        .contains_key("aggregate:dbstate_slice2.account_labels.text"));
    assert!(import
        .objects
        .contains_key("constraint:dbstate_slice2.sample_accounts.sample_accounts_pkey"));
    assert!(import
        .objects
        .contains_key("constraint:dbstate_slice2.sample_accounts.sample_accounts_code_key"));
    assert!(import
        .objects
        .contains_key("function:dbstate_slice2.account_label.account_id_integer"));
    assert!(import
        .objects
        .contains_key("function:dbstate_slice2.account_label.account_code_text"));
    assert!(import
        .objects
        .contains_key("materializedView:dbstate_slice2.account_summary"));
    assert!(import
        .objects
        .contains_key("trigger:dbstate_slice2.sample_accounts.sample_accounts_audit_trigger"));
    assert!(import
        .objects
        .contains_key("trigger:dbstate_slice2.other_accounts.sample_accounts_audit_trigger"));
    assert!(import
        .objects
        .contains_key("grant:table.dbstate_slice2.sample_accounts.public"));
    assert!(import
        .objects
        .contains_key("grant:table.dbstate_slice2.sample_accounts.app_reader"));
    assert!(import
        .objects
        .contains_key("grant:function.dbstate_slice2.account_label.account_id_integer.app_reader"));
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
    fs::write(dir.join("database/objects/domains/a.b.c.sql"), "-- bad\n")
        .expect("write bad domain file");
    fs::write(
        dir.join("database/objects/aggregates/a.b.sql"),
        "-- bad aggregate\n",
    )
    .expect("write bad aggregate file");
    fs::write(
        dir.join("database/objects/constraints/primary-keys/a.b.sql"),
        "-- bad\n",
    )
    .expect("write bad constraint file");
    fs::write(
        dir.join("database/objects/functions/a.b.sql"),
        "-- bad function\n",
    )
    .expect("write bad function file");
    fs::write(
        dir.join("database/objects/materialized-views/a.b.c.sql"),
        "-- bad materialized view\n",
    )
    .expect("write bad materialized view file");
    fs::write(
        dir.join("database/objects/triggers/a.b.sql"),
        "-- bad trigger\n",
    )
    .expect("write bad trigger file");
    fs::write(
        dir.join("database/objects/grants/functions/a.b.c.sql"),
        "-- bad grant\n",
    )
    .expect("write bad grant file");

    let import = discover_repository_objects(&dir).expect("discover objects");

    assert!(import
        .skipped
        .contains(&"database/objects/tables/bad.txt".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/tables/a.b.c.sql".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/domains/a.b.c.sql".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/aggregates/a.b.sql".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/constraints/primary-keys/a.b.sql".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/functions/a.b.sql".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/materialized-views/a.b.c.sql".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/triggers/a.b.sql".to_string()));
    assert!(import
        .skipped
        .contains(&"database/objects/grants/functions/a.b.c.sql".to_string()));
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
    fs::write(
        dir.join("database/objects/domains/dbstate_slice2.account_year.sql"),
        render_domain_sql(&sample_inventory().domains[0]),
    )
    .expect("write domain");
    fs::write(
        dir.join("database/objects/aggregates/dbstate_slice2.account_labels.text.sql"),
        render_aggregate_sql(&sample_inventory().aggregates[0]),
    )
    .expect("write aggregate");
    fs::write(
        dir.join(
            "database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql",
        ),
        render_constraint_sql(&sample_inventory().constraints[0]),
    )
    .expect("write constraint");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        render_function_sql(&sample_inventory().functions[0]),
    )
    .expect("write function");
    fs::write(
        dir.join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql",
        ),
        render_trigger_sql(&sample_inventory().triggers[0]),
    )
    .expect("write trigger");
    commit_all(&dir, "desired state files");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success);
    assert!(report
        .in_sync
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(report
        .in_sync
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    assert!(report
        .in_sync
        .contains(&"database/objects/domains/dbstate_slice2.account_year.sql".to_string()));
    assert!(report.in_sync.contains(
        &"database/objects/aggregates/dbstate_slice2.account_labels.text.sql".to_string()
    ));
    assert!(report.in_sync.contains(
        &"database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql"
            .to_string()
    ));
    assert!(report.in_sync.contains(
        &"database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"
            .to_string()
    ));
    assert!(report.in_sync.contains(
        &"database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql"
            .to_string()
    ));
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
    fs::write(
        dir.join("database/objects/domains/dbstate_slice2.account_year.sql"),
        "-- stale domain\n",
    )
    .expect("write stale domain");
    fs::write(
        dir.join("database/objects/aggregates/dbstate_slice2.account_labels.text.sql"),
        "-- stale aggregate\n",
    )
    .expect("write stale aggregate");
    fs::write(
        dir.join(
            "database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql",
        ),
        "-- stale constraint\n",
    )
    .expect("write stale constraint");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        "-- stale function\n",
    )
    .expect("write stale function");
    fs::write(
        dir.join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql",
        ),
        "-- stale trigger\n",
    )
    .expect("write stale trigger");
    commit_all(&dir, "stale desired state");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success);
    assert!(report
        .repo_different
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    assert!(report
        .repo_different
        .contains(&"database/objects/domains/dbstate_slice2.account_year.sql".to_string()));
    assert!(report.repo_different.contains(
        &"database/objects/aggregates/dbstate_slice2.account_labels.text.sql".to_string()
    ));
    assert!(report.repo_different.contains(
        &"database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql"
            .to_string()
    ));
    assert!(report.repo_different.contains(
        &"database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"
            .to_string()
    ));
    assert!(report.repo_different.contains(
        &"database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql"
            .to_string()
    ));
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
    fs::write(
        dir.join("database/objects/domains/dbstate_slice2.local_year.sql"),
        "CREATE DOMAIN \"dbstate_slice2\".\"local_year\" AS integer;\n",
    )
    .expect("write local only domain");
    fs::write(
        dir.join("database/objects/aggregates/dbstate_slice2.local_labels.text.sql"),
        "CREATE AGGREGATE \"dbstate_slice2\".\"local_labels\"(text) (SFUNC = \"dbstate_slice2\".\"account_label_concat\", STYPE = text);\n",
    )
    .expect("write local only aggregate");
    fs::write(
        dir.join(
            "database/objects/constraints/check-constraints/dbstate_slice2.sample_accounts.local_only_check.sql",
        ),
        "-- local only constraint\n",
    )
    .expect("write local only constraint");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.local_only.no_args.sql"),
        "CREATE FUNCTION dbstate_slice2.local_only()\n RETURNS text\n LANGUAGE sql\nAS $function$\n    SELECT 'local';\n$function$;\n",
    )
    .expect("write local only function");
    fs::write(
        dir.join("database/objects/triggers/dbstate_slice2.sample_accounts.local_only_trigger.sql"),
        "CREATE TRIGGER local_only_trigger AFTER INSERT ON dbstate_slice2.sample_accounts FOR EACH ROW EXECUTE FUNCTION dbstate_slice2.account_label(account_id);\n",
    )
    .expect("write local only trigger");
    commit_all(&dir, "local only desired state");

    let report = compare_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All);

    assert!(report.success);
    assert!(report
        .repo_only
        .contains(&"database/objects/tables/dbstate_slice2.local_only.sql".to_string()));
    assert!(report
        .repo_only
        .contains(&"database/objects/domains/dbstate_slice2.local_year.sql".to_string()));
    assert!(report
        .repo_only
        .contains(&"database/objects/aggregates/dbstate_slice2.local_labels.text.sql".to_string()));
    assert!(report.repo_only.contains(
        &"database/objects/constraints/check-constraints/dbstate_slice2.sample_accounts.local_only_check.sql"
            .to_string()
    ));
    assert!(report
        .repo_only
        .contains(&"database/objects/functions/dbstate_slice2.local_only.no_args.sql".to_string()));
    assert!(report.repo_only.contains(
        &"database/objects/triggers/dbstate_slice2.sample_accounts.local_only_trigger.sql"
            .to_string()
    ));
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
    assert!(report
        .database_only
        .contains(&"database/objects/domains/dbstate_slice2.account_year.sql".to_string()));
    assert!(report.database_only.contains(
        &"database/objects/aggregates/dbstate_slice2.account_labels.text.sql".to_string()
    ));
    assert!(report.database_only.contains(
        &"database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql"
            .to_string()
    ));
    assert!(report.database_only.contains(
        &"database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"
            .to_string()
    ));
    assert!(report.database_only.contains(
        &"database/objects/functions/dbstate_slice2.account_label.account_code_text.sql"
            .to_string()
    ));
    assert!(report.database_only.contains(
        &"database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql"
            .to_string()
    ));
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
fn function_release_plan_operation_badges_are_deterministic() {
    let repo_only_dir = create_temp_dir("plan-function-repo-only");
    init_git_repo(&repo_only_dir);
    create_complete_structure(&repo_only_dir);
    fs::write(
        repo_only_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        repo_only_dir
            .join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        render_function_sql(&sample_inventory().functions[0]),
    )
    .expect("write repo-only function");
    commit_all(&repo_only_dir, "repo-only function");
    let mut target_inventory = sample_inventory();
    target_inventory.functions.clear();

    let repo_only = plan_postgres_with_inventory(
        &repo_only_dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
    );

    assert!(repo_only.success, "{:?}", repo_only.errors);
    let item = repo_only
        .plan_items
        .iter()
        .find(|item| item.object_ref == "function:dbstate_slice2.account_label.account_id_integer")
        .expect("function plan item");
    assert_eq!(item.operation_kind, "createFunctionReviewSql");
    assert_eq!(item.operation_label, "Create Function");
    assert_eq!(item.safety_badge, "Review SQL");
    assert!(item
        .operation_explanation
        .contains("review-only CREATE FUNCTION"));
    assert!(item
        .operation_reasons
        .iter()
        .any(|reason| reason.contains("does not generate DROP FUNCTION")));

    let changed_dir = create_temp_dir("plan-function-changed");
    init_git_repo(&changed_dir);
    create_complete_structure(&changed_dir);
    fs::write(
        changed_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        changed_dir
            .join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        "CREATE FUNCTION dbstate_slice2.account_label(account_id integer)\n RETURNS text\n LANGUAGE sql\nAS $function$\n    SELECT 'changed';\n$function$;\n",
    )
    .expect("write changed function");
    commit_all(&changed_dir, "changed function");

    let changed = plan_postgres_with_inventory(
        &changed_dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
    );

    assert!(changed.success, "{:?}", changed.errors);
    let changed_item = changed
        .plan_items
        .iter()
        .find(|item| item.object_ref == "function:dbstate_slice2.account_label.account_id_integer")
        .expect("changed function item");
    assert_eq!(changed_item.operation_kind, "manualReviewRequired");
    assert_eq!(changed_item.safety_badge, "Manual Review");
    assert!(changed_item
        .operation_explanation
        .contains("changed functions are manual-review only"));

    let database_only_dir = create_temp_dir("plan-function-database-only");
    init_git_repo(&database_only_dir);
    create_complete_structure(&database_only_dir);
    fs::write(
        database_only_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&database_only_dir, "schema only");

    let database_only = plan_postgres_with_inventory(
        &database_only_dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
    );

    assert!(database_only.success, "{:?}", database_only.errors);
    let database_only_item = database_only
        .plan_items
        .iter()
        .find(|item| item.object_ref == "function:dbstate_slice2.account_label.account_id_integer")
        .expect("database-only function item");
    assert_eq!(database_only_item.operation_kind, "databaseOnlyReview");
    assert_eq!(database_only_item.safety_badge, "Database Only");
    assert!(database_only_item
        .operation_explanation
        .contains("will not generate destructive SQL"));
}

#[test]
fn trigger_release_plan_operation_badges_are_deterministic() {
    let trigger_ref =
        "trigger:dbstate_slice2.sample_accounts.sample_accounts_audit_trigger".to_string();
    let trigger_path =
        "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql";

    let repo_only_dir = create_temp_dir("plan-trigger-repo-only");
    init_git_repo(&repo_only_dir);
    create_complete_structure(&repo_only_dir);
    fs::write(
        repo_only_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        repo_only_dir.join(trigger_path),
        render_trigger_sql(&sample_inventory().triggers[0]),
    )
    .expect("write repo-only trigger");
    commit_all(&repo_only_dir, "repo-only trigger");
    let mut target_inventory = sample_inventory();
    target_inventory.triggers.clear();

    let repo_only = plan_postgres_with_inventory(
        &repo_only_dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(vec![trigger_ref.clone()], Vec::new()).expect("selection"),
    );

    assert!(repo_only.success, "{:?}", repo_only.errors);
    let item = repo_only
        .plan_items
        .iter()
        .find(|item| item.object_ref == trigger_ref)
        .expect("trigger plan item");
    assert_eq!(item.operation_kind, "createTriggerReviewSql");
    assert_eq!(item.operation_label, "Create Trigger");
    assert_eq!(item.safety_badge, "Review SQL");
    assert!(item
        .operation_explanation
        .contains("review-only CREATE TRIGGER"));
    assert!(item
        .operation_reasons
        .iter()
        .any(|reason| reason.contains("does not generate DROP TRIGGER")));

    let changed_dir = create_temp_dir("plan-trigger-changed");
    init_git_repo(&changed_dir);
    create_complete_structure(&changed_dir);
    fs::write(
        changed_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(changed_dir.join(trigger_path), "-- stale trigger\n").expect("write trigger");
    commit_all(&changed_dir, "changed trigger");

    let changed = plan_postgres_with_inventory(
        &changed_dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec![trigger_ref.clone()], Vec::new()).expect("selection"),
    );

    assert!(changed.success, "{:?}", changed.errors);
    let changed_item = changed
        .plan_items
        .iter()
        .find(|item| item.object_ref == trigger_ref)
        .expect("changed trigger item");
    assert_eq!(changed_item.operation_kind, "manualReviewRequired");
    assert_eq!(changed_item.safety_badge, "Manual Review");
    assert!(changed_item
        .operation_explanation
        .contains("changed triggers are manual-review only"));

    let database_only_dir = create_temp_dir("plan-trigger-database-only");
    init_git_repo(&database_only_dir);
    create_complete_structure(&database_only_dir);
    fs::write(
        database_only_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&database_only_dir, "schema only");

    let database_only = plan_postgres_with_inventory(
        &database_only_dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec![trigger_ref.clone()], Vec::new()).expect("selection"),
    );

    assert!(database_only.success, "{:?}", database_only.errors);
    let database_only_item = database_only
        .plan_items
        .iter()
        .find(|item| item.object_ref == trigger_ref)
        .expect("database-only trigger item");
    assert_eq!(database_only_item.operation_kind, "databaseOnlyReview");
    assert_eq!(database_only_item.safety_badge, "Database Only");
    assert!(database_only_item
        .operation_reasons
        .iter()
        .any(|reason| reason.contains("DROP TRIGGER generation is not available")));
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
fn release_write_allows_dirty_working_tree_when_paths_are_unrelated() {
    let dir = create_temp_dir("release-dirty");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("write dirty file");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["schema:local_only".to_string()], Vec::new())
            .expect("plan selection"),
        "slice7",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("unrelated changes")));
    assert!(!report.created_artifacts.is_empty());
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
        .contains(&"database/releases/objects/0001_slice7.sql".to_string()));
    assert!(report.created_artifacts.is_empty());
    assert!(!dir
        .join("database/releases/objects/0001_slice7.sql")
        .exists());
    assert!(dir.join("database/releases/objects").is_dir());
    let json = report.to_json();
    assert!(json.contains("\"operationKind\":\"createReviewSql\""));
    assert!(json.contains("\"safetyBadge\":\"Review SQL\""));
    assert!(json.contains("review-only SQL"));
    assert!(json.contains("DbState does not execute SQL"));
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
            "database/releases/objects/0001_slice7_test.sql".to_string(),
            "database/releases/objects/0001_slice7_test.summary.md".to_string(),
            "database/releases/objects/0001_slice7_test.risk.json".to_string(),
            "database/releases/objects/0001_slice7_test.manifest.json".to_string(),
        ]
    );
    assert!(!dir.join("database/releases/0001_slice7_test.sql").exists());
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice7_test.sql"))
        .expect("sql");
    let summary =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice7_test.summary.md"))
            .expect("summary");
    let risk = fs::read_to_string(dir.join("database/releases/objects/0001_slice7_test.risk.json"))
        .expect("risk");
    let manifest =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice7_test.manifest.json"))
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
fn release_generates_review_only_create_for_repo_only_partitioned_table() {
    let dir = create_temp_dir("release-partitioned-table");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let inventory = partitioned_payment_inventory();
    fs::write(
        dir.join("database/objects/schemas/public.sql"),
        render_schema_sql("public"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/public.payment.sql"),
        render_table_sql_with_partition(
            "public",
            "payment",
            &inventory.columns,
            inventory.tables[0].partition_key.as_deref(),
        ),
    )
    .expect("write partitioned table");
    commit_all(&dir, "repo-only partitioned table");
    let mut target_without_payment = inventory.clone();
    target_without_payment.tables.clear();
    target_without_payment.columns.clear();
    target_without_payment.constraints.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_without_payment,
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["table:public.payment".to_string()], Vec::new())
            .expect("plan selection"),
        "partitioned_payment",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    let sql =
        fs::read_to_string(dir.join("database/releases/objects/0001_partitioned_payment.sql"))
            .expect("read release sql");
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS \"public\".\"payment\""));
    assert!(sql.contains("PARTITION BY RANGE (payment_date);"));
    assert!(!sql.contains("DROP TABLE"));
}

#[test]
fn release_keeps_changed_and_database_only_partitioned_tables_manual_review_only() {
    let inventory = partitioned_payment_inventory();
    let table_path = "database/objects/tables/public.payment.sql";

    let changed_dir = create_temp_dir("release-partitioned-changed");
    init_git_repo(&changed_dir);
    create_complete_structure(&changed_dir);
    fs::write(
        changed_dir.join("database/objects/schemas/public.sql"),
        render_schema_sql("public"),
    )
    .expect("write schema");
    fs::write(
        changed_dir.join(table_path),
        render_table_sql_with_partition(
            "public",
            "payment",
            &inventory.columns,
            Some("LIST (payment_id)"),
        ),
    )
    .expect("write changed partitioned table");
    commit_all(&changed_dir, "changed partitioned table");
    let changed = release_postgres_with_inventory(
        &changed_dir,
        &inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["table:public.payment".to_string()], Vec::new())
            .expect("plan selection"),
        "partitioned_changed",
        false,
    );
    assert!(changed.success, "{:?}", changed.errors);
    let changed_sql = fs::read_to_string(
        changed_dir.join("database/releases/objects/0001_partitioned_changed.sql"),
    )
    .expect("read changed sql");
    assert!(changed_sql.contains("REVIEW REQUIRED"));
    assert!(!changed_sql.contains("DROP TABLE"));

    let database_only_dir = create_temp_dir("release-partitioned-database-only");
    init_git_repo(&database_only_dir);
    create_complete_structure(&database_only_dir);
    fs::write(
        database_only_dir.join("database/objects/schemas/public.sql"),
        render_schema_sql("public"),
    )
    .expect("write schema");
    commit_all(&database_only_dir, "schema only");
    let database_only = release_postgres_with_inventory(
        &database_only_dir,
        &inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["table:public.payment".to_string()], Vec::new())
            .expect("plan selection"),
        "partitioned_database_only",
        false,
    );
    assert!(database_only.success, "{:?}", database_only.errors);
    let database_only_sql = fs::read_to_string(
        database_only_dir.join("database/releases/objects/0001_partitioned_database_only.sql"),
    )
    .expect("read database-only sql");
    assert!(database_only_sql.contains("object exists only in target database"));
    assert!(!database_only_sql.contains("DROP TABLE"));
}

#[test]
fn release_write_backfills_missing_objects_release_folder() {
    let dir = create_temp_dir("release-backfill-objects-folder");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    commit_all(&dir, "repo-only schema");
    fs::remove_dir_all(dir.join("database/releases/objects")).expect("remove objects release dir");
    fs::remove_dir_all(dir.join("database/releases/reference-data"))
        .expect("remove reference-data release dir");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["schema:local_only".to_string()], Vec::new())
            .expect("plan selection"),
        "slice34e",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert!(dir.join("database/releases/objects").is_dir());
    assert!(report
        .created_artifacts
        .contains(&"database/releases/objects/0001_slice34e.sql".to_string()));
    assert!(dir
        .join("database/releases/objects/0001_slice34e.sql")
        .exists());
    assert!(report.warnings.iter().any(|warning| {
        warning.contains("database/releases/objects")
            || warning.contains("release artifact subfolders")
    }));
}

#[test]
fn repo_only_domain_release_generates_review_only_create_domain_sql() {
    let dir = create_temp_dir("release-domain-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/domains/dbstate_slice2.account_year.sql"),
        render_domain_sql(&sample_inventory().domains[0]),
    )
    .expect("write repo-only domain");
    commit_all(&dir, "repo-only domain");
    let mut target_inventory = sample_inventory();
    target_inventory.domains.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["domain:dbstate_slice2.account_year".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice37_domain",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "createDomainReviewSql");
    assert_eq!(report.plan_items[0].operation_label, "Create Domain");
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice37_domain.sql"))
        .expect("sql");
    assert!(sql.contains("CREATE DOMAIN \"dbstate_slice2\".\"account_year\" AS integer"));
    assert!(sql.contains("CONSTRAINT \"account_year_check\""));
    assert!(!sql.contains("\nDROP DOMAIN"));
}

#[test]
fn changed_domain_release_remains_manual_review_only() {
    let dir = create_temp_dir("release-domain-changed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/domains/dbstate_slice2.account_year.sql"),
        "CREATE DOMAIN \"dbstate_slice2\".\"account_year\" AS text;\n",
    )
    .expect("write changed domain");
    commit_all(&dir, "changed domain");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["domain:dbstate_slice2.account_year".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice37_domain_changed",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "manualReviewRequired");
    assert!(report.plan_items[0]
        .operation_explanation
        .contains("changed domains are manual-review only"));
    let sql =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice37_domain_changed.sql"))
            .expect("sql");
    assert!(sql.contains("changed domains are manual-review only"));
    assert!(!sql.contains("\nDROP DOMAIN"));
    assert!(!sql.contains("\nALTER DOMAIN"));
}

#[test]
fn database_only_domain_release_does_not_generate_drop_domain() {
    let dir = create_temp_dir("release-domain-db-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&dir, "complete structure");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["domain:dbstate_slice2.account_year".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice37_domain_database_only",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].compare_classification, "databaseOnly");
    assert_eq!(report.plan_items[0].operation_kind, "databaseOnlyReview");
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice37_domain_database_only.sql"),
    )
    .expect("sql");
    assert!(sql.contains("domain exists only in target database"));
    assert!(!sql.contains("\nDROP DOMAIN"));
}

#[test]
fn repo_only_aggregate_release_generates_review_only_create_aggregate_sql() {
    let dir = create_temp_dir("release-aggregate-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/aggregates/dbstate_slice2.account_labels.text.sql"),
        render_aggregate_sql(&sample_inventory().aggregates[0]),
    )
    .expect("write repo-only aggregate");
    commit_all(&dir, "repo-only aggregate");
    let mut target_inventory = sample_inventory();
    target_inventory.aggregates.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["aggregate:dbstate_slice2.account_labels.text".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice37_aggregate",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.plan_items[0].operation_kind,
        "createAggregateReviewSql"
    );
    assert_eq!(report.plan_items[0].operation_label, "Create Aggregate");
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice37_aggregate.sql"))
        .expect("sql");
    assert!(sql.contains("CREATE AGGREGATE \"dbstate_slice2\".\"account_labels\"(text)"));
    assert!(!sql.contains("\nDROP AGGREGATE"));
}

#[test]
fn changed_and_database_only_aggregate_release_remains_manual_review_only() {
    let changed_dir = create_temp_dir("release-aggregate-changed");
    init_git_repo(&changed_dir);
    create_complete_structure(&changed_dir);
    fs::write(
        changed_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        changed_dir.join("database/objects/aggregates/dbstate_slice2.account_labels.text.sql"),
        "CREATE AGGREGATE \"dbstate_slice2\".\"account_labels\"(text) (SFUNC = \"dbstate_slice2\".\"other_concat\", STYPE = text);\n",
    )
    .expect("write changed aggregate");
    commit_all(&changed_dir, "changed aggregate");

    let changed = release_postgres_with_inventory(
        &changed_dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["aggregate:dbstate_slice2.account_labels.text".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice37_aggregate_changed",
        false,
    );

    assert!(changed.success, "{:?}", changed.errors);
    assert!(changed.plan_items[0]
        .operation_explanation
        .contains("changed aggregates are manual-review only"));
    let changed_sql = fs::read_to_string(
        changed_dir.join("database/releases/objects/0001_slice37_aggregate_changed.sql"),
    )
    .expect("sql");
    assert!(changed_sql.contains("changed aggregates are manual-review only"));
    assert!(!changed_sql.contains("\nDROP AGGREGATE"));

    let db_only_dir = create_temp_dir("release-aggregate-db-only");
    init_git_repo(&db_only_dir);
    create_complete_structure(&db_only_dir);
    fs::write(
        db_only_dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&db_only_dir, "complete structure");

    let database_only = release_postgres_with_inventory(
        &db_only_dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["aggregate:dbstate_slice2.account_labels.text".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice37_aggregate_database_only",
        false,
    );

    assert!(database_only.success, "{:?}", database_only.errors);
    assert_eq!(
        database_only.plan_items[0].compare_classification,
        "databaseOnly"
    );
    let database_only_sql = fs::read_to_string(
        db_only_dir.join("database/releases/objects/0001_slice37_aggregate_database_only.sql"),
    )
    .expect("sql");
    assert!(database_only_sql
        .to_lowercase()
        .contains("aggregate exists only in target database"));
    assert!(!database_only_sql.contains("\nDROP AGGREGATE"));
}

#[test]
fn repo_only_constraint_release_generates_review_only_add_constraint_sql() {
    let dir = create_temp_dir("release-constraint-add");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    let constraint = ConstraintInfo {
        schema_name: "dbstate_slice2".to_string(),
        table_name: "sample_accounts".to_string(),
        constraint_name: "sample_accounts_extra_check".to_string(),
        constraint_type: "checkConstraint".to_string(),
        definition: "CHECK ((account_code <> 'blocked'::text))".to_string(),
        columns: vec!["account_code".to_string()],
        referenced_schema: None,
        referenced_table: None,
        referenced_columns: Vec::new(),
    };
    fs::write(
        dir.join(
            "database/objects/constraints/check-constraints/dbstate_slice2.sample_accounts.sample_accounts_extra_check.sql",
        ),
        render_constraint_sql(&constraint),
    )
    .expect("write repo-only constraint");
    commit_all(&dir, "repo-only constraint");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "constraint:dbstate_slice2.sample_accounts.sample_accounts_extra_check".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice28_constraint",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.plan_items[0].operation_kind,
        "createConstraintReviewSql"
    );
    assert_eq!(report.plan_items[0].operation_label, "Add Constraint");
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice28_constraint.sql"))
        .expect("sql");
    assert!(sql.contains("-- Review-only constraint suggestion."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("ALTER TABLE \"dbstate_slice2\".\"sample_accounts\""));
    assert!(sql.contains(
        "ADD CONSTRAINT \"sample_accounts_extra_check\" CHECK ((account_code <> 'blocked'::text));"
    ));
    assert!(!sql.contains("DROP CONSTRAINT"));
    assert!(!sql.contains("DELETE FROM"));
    assert!(!sql.contains("INSERT INTO"));
}

#[test]
fn changed_constraint_release_remains_manual_review_only() {
    let dir = create_temp_dir("release-constraint-changed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    let mut changed = sample_inventory().constraints[0].clone();
    changed.definition = "PRIMARY KEY (account_code)".to_string();
    fs::write(
        dir.join(
            "database/objects/constraints/primary-keys/dbstate_slice2.sample_accounts.sample_accounts_pkey.sql",
        ),
        render_constraint_sql(&changed),
    )
    .expect("write changed constraint");
    commit_all(&dir, "changed constraint");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["constraint:dbstate_slice2.sample_accounts.sample_accounts_pkey".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice28_changed_constraint",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "manualReviewRequired");
    assert!(report.plan_items[0]
        .operation_explanation
        .contains("changed constraints are manual-review only"));
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice28_changed_constraint.sql"),
    )
    .expect("sql");
    assert!(sql.contains("changed constraints are manual-review only"));
    assert!(!sql.contains("DROP CONSTRAINT"));
    assert!(!sql.contains("ADD CONSTRAINT \"sample_accounts_pkey\""));
}

#[test]
fn database_only_constraint_release_does_not_generate_drop_constraint() {
    let dir = create_temp_dir("release-constraint-db-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&dir, "complete structure");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["constraint:dbstate_slice2.sample_accounts.sample_accounts_pkey".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice28_database_only_constraint",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].compare_classification, "databaseOnly");
    assert_eq!(report.plan_items[0].operation_kind, "databaseOnlyReview");
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice28_database_only_constraint.sql"),
    )
    .expect("sql");
    assert!(sql.contains("object exists only in target database"));
    assert!(!sql.contains("DROP CONSTRAINT"));
}

#[test]
fn release_candidate_selection_respects_selected_constraint_refs() {
    let dir = create_temp_dir("release-constraint-selection");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    let selected = ConstraintInfo {
        schema_name: "dbstate_slice2".to_string(),
        table_name: "sample_accounts".to_string(),
        constraint_name: "selected_constraint_check".to_string(),
        constraint_type: "checkConstraint".to_string(),
        definition: "CHECK ((account_code <> 'selected'::text))".to_string(),
        columns: vec!["account_code".to_string()],
        referenced_schema: None,
        referenced_table: None,
        referenced_columns: Vec::new(),
    };
    let unselected = ConstraintInfo {
        constraint_name: "unselected_constraint_check".to_string(),
        definition: "CHECK ((account_code <> 'unselected'::text))".to_string(),
        ..selected.clone()
    };
    fs::write(
        dir.join(
            "database/objects/constraints/check-constraints/dbstate_slice2.sample_accounts.selected_constraint_check.sql",
        ),
        render_constraint_sql(&selected),
    )
    .expect("write selected constraint");
    fs::write(
        dir.join(
            "database/objects/constraints/check-constraints/dbstate_slice2.sample_accounts.unselected_constraint_check.sql",
        ),
        render_constraint_sql(&unselected),
    )
    .expect("write unselected constraint");
    commit_all(&dir, "repo-only constraints");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["constraint:dbstate_slice2.sample_accounts.selected_constraint_check".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice28_constraint_selection",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.included_objects,
        vec!["constraint:dbstate_slice2.sample_accounts.selected_constraint_check".to_string()]
    );
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice28_constraint_selection.sql"),
    )
    .expect("sql");
    assert!(sql.contains("selected_constraint_check"));
    assert!(!sql.contains("unselected_constraint_check"));
}

#[test]
fn repo_only_function_release_generates_review_only_create_function_sql() {
    let dir = create_temp_dir("release-function-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        render_function_sql(&sample_inventory().functions[0]),
    )
    .expect("write repo-only function");
    commit_all(&dir, "repo-only function");
    let mut target_inventory = sample_inventory();
    target_inventory.functions.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice29_function",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.plan_items[0].operation_kind,
        "createFunctionReviewSql"
    );
    assert_eq!(report.plan_items[0].operation_label, "Create Function");
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice29_function.sql"))
        .expect("sql");
    assert!(sql.contains("-- Review-only function suggestion."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("-- Review before applying manually outside DbState."));
    assert!(sql.contains("CREATE FUNCTION dbstate_slice2.account_label(account_id integer)"));
    assert!(!sql.contains("\nDROP FUNCTION"));
    assert!(!sql.contains("DELETE FROM"));
    assert!(!sql.contains("INSERT INTO"));
}

#[test]
fn changed_function_release_remains_manual_review_only() {
    let dir = create_temp_dir("release-function-changed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        "CREATE FUNCTION dbstate_slice2.account_label(account_id integer)\n RETURNS text\n LANGUAGE sql\nAS $function$\n    SELECT 'changed';\n$function$;\n",
    )
    .expect("write changed function");
    commit_all(&dir, "changed function");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice29_changed_function",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "manualReviewRequired");
    assert!(report.plan_items[0]
        .operation_explanation
        .contains("changed functions are manual-review only"));
    let sql =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice29_changed_function.sql"))
            .expect("sql");
    assert!(sql.contains("changed functions are manual-review only"));
    assert!(!sql.contains("\nDROP FUNCTION"));
    assert!(!sql.contains("CREATE FUNCTION dbstate_slice2.account_label"));
}

#[test]
fn database_only_function_release_does_not_generate_drop_function() {
    let dir = create_temp_dir("release-function-db-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&dir, "schema only");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice29_database_only_function",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].compare_classification, "databaseOnly");
    assert_eq!(report.plan_items[0].operation_kind, "databaseOnlyReview");
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice29_database_only_function.sql"),
    )
    .expect("sql");
    assert!(sql.contains("object exists only in target database"));
    assert!(!sql.contains("DROP FUNCTION"));
}

#[test]
fn release_candidate_selection_respects_selected_function_refs() {
    let dir = create_temp_dir("release-function-selection");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_id_integer.sql"),
        render_function_sql(&sample_inventory().functions[0]),
    )
    .expect("write selected function");
    fs::write(
        dir.join("database/objects/functions/dbstate_slice2.account_label.account_code_text.sql"),
        render_function_sql(&sample_inventory().functions[1]),
    )
    .expect("write unselected function");
    commit_all(&dir, "repo-only functions");
    let mut target_inventory = sample_inventory();
    target_inventory.functions.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice29_function_selection",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.included_objects,
        vec!["function:dbstate_slice2.account_label.account_id_integer".to_string()]
    );
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice29_function_selection.sql"),
    )
    .expect("sql");
    assert!(sql.contains("account_id integer"));
    assert!(!sql.contains("account_code text"));
}

#[test]
fn repo_only_trigger_release_generates_review_only_create_trigger_sql() {
    let dir = create_temp_dir("release-trigger-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql",
        ),
        render_trigger_sql(&sample_inventory().triggers[0]),
    )
    .expect("write repo-only trigger");
    commit_all(&dir, "repo-only trigger");
    let mut target_inventory = sample_inventory();
    target_inventory.triggers.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "trigger:dbstate_slice2.sample_accounts.sample_accounts_audit_trigger".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice30_trigger",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.plan_items[0].operation_kind,
        "createTriggerReviewSql"
    );
    assert_eq!(report.plan_items[0].operation_label, "Create Trigger");
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice30_trigger.sql"))
        .expect("sql");
    assert!(sql.contains("-- Review-only trigger suggestion."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("-- Review before applying manually outside DbState."));
    assert!(sql.contains("CREATE TRIGGER sample_accounts_audit_trigger"));
    assert!(!sql.contains("\nDROP TRIGGER"));
    assert!(!sql.contains("\nALTER TRIGGER"));
    assert!(!sql.contains("DELETE FROM"));
    assert!(!sql.contains("INSERT INTO"));
}

#[test]
fn changed_trigger_release_remains_manual_review_only() {
    let dir = create_temp_dir("release-trigger-changed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.sample_accounts_audit_trigger.sql",
        ),
        "-- stale trigger\n",
    )
    .expect("write changed trigger");
    commit_all(&dir, "changed trigger");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "trigger:dbstate_slice2.sample_accounts.sample_accounts_audit_trigger".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice30_changed_trigger",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "manualReviewRequired");
    assert!(report.plan_items[0]
        .operation_explanation
        .contains("changed triggers are manual-review only"));
    let sql =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice30_changed_trigger.sql"))
            .expect("sql");
    assert!(sql.contains("changed triggers are manual-review only"));
    assert!(!sql.contains("\nDROP TRIGGER"));
    assert!(!sql.contains("\nALTER TRIGGER"));
    assert!(!sql.contains("CREATE TRIGGER sample_accounts_audit_trigger"));
}

#[test]
fn database_only_trigger_release_does_not_generate_drop_trigger() {
    let dir = create_temp_dir("release-trigger-db-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&dir, "schema only");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "trigger:dbstate_slice2.sample_accounts.sample_accounts_audit_trigger".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice30_database_only_trigger",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].compare_classification, "databaseOnly");
    assert_eq!(report.plan_items[0].operation_kind, "databaseOnlyReview");
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice30_database_only_trigger.sql"),
    )
    .expect("sql");
    assert!(sql.contains("object exists only in target database"));
    assert!(!sql.contains("DROP TRIGGER"));
    assert!(!sql.contains("ALTER TRIGGER"));
}

#[test]
fn release_candidate_selection_respects_selected_trigger_refs() {
    let dir = create_temp_dir("release-trigger-selection");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    let mut selected = sample_inventory().triggers[0].clone();
    selected.trigger_name = "selected_audit_trigger".to_string();
    selected.definition =
        "CREATE TRIGGER selected_audit_trigger AFTER INSERT ON dbstate_slice2.sample_accounts FOR EACH ROW EXECUTE FUNCTION dbstate_slice2.account_label(account_id)"
            .to_string();
    let mut unselected = selected.clone();
    unselected.trigger_name = "unselected_audit_trigger".to_string();
    unselected.definition =
        "CREATE TRIGGER unselected_audit_trigger AFTER INSERT ON dbstate_slice2.sample_accounts FOR EACH ROW EXECUTE FUNCTION dbstate_slice2.account_label(account_id)"
            .to_string();
    fs::write(
        dir.join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.selected_audit_trigger.sql",
        ),
        render_trigger_sql(&selected),
    )
    .expect("write selected trigger");
    fs::write(
        dir.join(
            "database/objects/triggers/dbstate_slice2.sample_accounts.unselected_audit_trigger.sql",
        ),
        render_trigger_sql(&unselected),
    )
    .expect("write unselected trigger");
    commit_all(&dir, "repo-only triggers");
    let mut target_inventory = sample_inventory();
    target_inventory.triggers.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["trigger:dbstate_slice2.sample_accounts.selected_audit_trigger".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice30_trigger_selection",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.included_objects,
        vec!["trigger:dbstate_slice2.sample_accounts.selected_audit_trigger".to_string()]
    );
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice30_trigger_selection.sql"),
    )
    .expect("sql");
    assert!(sql.contains("selected_audit_trigger"));
    assert!(!sql.contains("unselected_audit_trigger"));
}

#[test]
fn repo_only_materialized_view_release_generates_review_only_create_sql() {
    let dir = create_temp_dir("release-matview-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/materialized-views/dbstate_slice2.account_summary.sql"),
        render_materialized_view_sql(&sample_inventory().materialized_views[0]),
    )
    .expect("write repo-only materialized view");
    commit_all(&dir, "repo-only materialized view");
    let mut target_inventory = sample_inventory();
    target_inventory.materialized_views.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["materializedView:dbstate_slice2.account_summary".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice31_matview",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.plan_items[0].operation_kind,
        "createMaterializedViewReviewSql"
    );
    assert_eq!(
        report.plan_items[0].operation_label,
        "Create Materialized View"
    );
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice31_matview.sql"))
        .expect("sql");
    assert!(sql.contains("-- Review-only materialized view suggestion."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("-- DbState does not refresh materialized views."));
    assert!(sql.contains("CREATE MATERIALIZED VIEW \"dbstate_slice2\".\"account_summary\" AS"));
    assert!(sql.contains("WITH NO DATA;"));
    assert!(!sql.contains("DROP MATERIALIZED VIEW"));
    assert!(!sql.contains("REFRESH MATERIALIZED VIEW"));
}

#[test]
fn changed_materialized_view_release_remains_manual_review_only() {
    let dir = create_temp_dir("release-matview-changed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/materialized-views/dbstate_slice2.account_summary.sql"),
        "-- stale materialized view\n",
    )
    .expect("write changed materialized view");
    commit_all(&dir, "changed materialized view");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["materializedView:dbstate_slice2.account_summary".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice31_changed_matview",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "manualReviewRequired");
    assert!(report.plan_items[0]
        .operation_explanation
        .contains("changed materialized views are manual-review only"));
    let sql =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice31_changed_matview.sql"))
            .expect("sql");
    assert!(sql.contains("changed materialized views are manual-review only"));
    assert!(!sql.contains("DROP MATERIALIZED VIEW"));
    assert!(!sql.contains("REFRESH MATERIALIZED VIEW"));
}

#[test]
fn database_only_materialized_view_release_does_not_generate_drop_or_refresh() {
    let dir = create_temp_dir("release-matview-db-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&dir, "schema only");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["materializedView:dbstate_slice2.account_summary".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice31_database_only_matview",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].compare_classification, "databaseOnly");
    assert_eq!(report.plan_items[0].operation_kind, "databaseOnlyReview");
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice31_database_only_matview.sql"),
    )
    .expect("sql");
    assert!(sql.contains("object exists only in target database"));
    assert!(!sql.contains("DROP MATERIALIZED VIEW"));
    assert!(!sql.contains("REFRESH MATERIALIZED VIEW"));
}

#[test]
fn release_candidate_selection_respects_selected_materialized_view_refs() {
    let dir = create_temp_dir("release-matview-selection");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    let mut selected = sample_inventory().materialized_views[0].clone();
    selected.materialized_view_name = "selected_summary".to_string();
    let mut unselected = selected.clone();
    unselected.materialized_view_name = "unselected_summary".to_string();
    fs::write(
        dir.join("database/objects/materialized-views/dbstate_slice2.selected_summary.sql"),
        render_materialized_view_sql(&selected),
    )
    .expect("write selected materialized view");
    fs::write(
        dir.join("database/objects/materialized-views/dbstate_slice2.unselected_summary.sql"),
        render_materialized_view_sql(&unselected),
    )
    .expect("write unselected materialized view");
    commit_all(&dir, "repo-only materialized views");
    let mut target_inventory = sample_inventory();
    target_inventory.materialized_views.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["materializedView:dbstate_slice2.selected_summary".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice31_matview_selection",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.included_objects,
        vec!["materializedView:dbstate_slice2.selected_summary".to_string()]
    );
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice31_matview_selection.sql"),
    )
    .expect("sql");
    assert!(sql.contains("selected_summary"));
    assert!(!sql.contains("unselected_summary"));
}

#[test]
fn repo_only_grant_release_generates_review_only_grant_sql() {
    let dir = create_temp_dir("release-grant-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql"),
        render_grant_sql(&sample_inventory().grants[1]),
    )
    .expect("write repo-only grant");
    commit_all(&dir, "repo-only grant");
    let mut target_inventory = sample_inventory();
    target_inventory.grants.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["grant:table.dbstate_slice2.sample_accounts.public".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice32_grant",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.plan_items[0].operation_kind,
        "grantPrivilegesReviewSql"
    );
    assert_eq!(report.plan_items[0].operation_label, "Grant Privileges");
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice32_grant.sql"))
        .expect("sql");
    assert!(sql.contains("-- Review-only grant suggestion."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("GRANT SELECT ON TABLE \"dbstate_slice2\".\"sample_accounts\" TO PUBLIC;"));
    assert!(!sql.contains("REVOKE"));
    assert!(!sql.contains("WITH ADMIN OPTION"));
}

#[test]
fn changed_grant_release_remains_manual_review_only() {
    let dir = create_temp_dir("release-grant-changed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql"),
        "-- stale grant\nGRANT INSERT ON TABLE \"dbstate_slice2\".\"sample_accounts\" TO PUBLIC;\n",
    )
    .expect("write changed grant");
    commit_all(&dir, "changed grant");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["grant:table.dbstate_slice2.sample_accounts.public".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice32_changed_grant",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "manualReviewRequired");
    assert!(report.plan_items[0]
        .operation_explanation
        .contains("changed grants are manual-review only"));
    let sql =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice32_changed_grant.sql"))
            .expect("sql");
    assert!(sql.contains("changed grants are manual-review only"));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("REVOKE")));
}

#[test]
fn database_only_grant_release_does_not_generate_revoke() {
    let dir = create_temp_dir("release-grant-db-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&dir, "schema only");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["grant:table.dbstate_slice2.sample_accounts.public".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice32_database_only_grant",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_kind, "databaseOnlyReview");
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice32_database_only_grant.sql"),
    )
    .expect("sql");
    assert!(sql.contains("grant exists only in target database"));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("REVOKE")));
}

#[test]
fn release_candidate_selection_respects_selected_grant_refs() {
    let dir = create_temp_dir("release-grant-selection");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/grants/schemas/dbstate_slice2.app_reader.sql"),
        render_grant_sql(&sample_inventory().grants[0]),
    )
    .expect("write selected grant");
    fs::write(
        dir.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql"),
        render_grant_sql(&sample_inventory().grants[1]),
    )
    .expect("write unselected grant");
    commit_all(&dir, "repo-only grants");
    let mut target_inventory = sample_inventory();
    target_inventory.grants.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["grant:schema.dbstate_slice2.app_reader".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice32_grant_selection",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.included_objects,
        vec!["grant:schema.dbstate_slice2.app_reader".to_string()]
    );
    let sql =
        fs::read_to_string(dir.join("database/releases/objects/0001_slice32_grant_selection.sql"))
            .expect("sql");
    assert!(sql.contains("GRANT USAGE ON SCHEMA \"dbstate_slice2\" TO \"app_reader\";"));
    assert!(!sql.contains("sample_accounts"));
}

#[test]
fn repo_only_rls_policy_release_generates_review_only_create_policy_sql() {
    let dir = create_temp_dir("release-rls-policy-create");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_public_read.sql"),
        render_rls_policy_sql(&sample_inventory().rls_policies[0]),
    )
    .expect("write repo-only RLS policy");
    commit_all(&dir, "repo-only RLS policy");
    let mut target_inventory = sample_inventory();
    target_inventory.rls_policies.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "rlsPolicy:dbstate_slice2.sample_accounts.sample_accounts_public_read".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice33_rls_policy",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.plan_items[0].operation_kind,
        "createRlsPolicyReviewSql"
    );
    assert_eq!(report.plan_items[0].operation_label, "Create RLS Policy");
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice33_rls_policy.sql"))
        .expect("sql");
    assert!(sql.contains("-- Review-only RLS policy suggestion."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("CREATE POLICY \"sample_accounts_public_read\""));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("DROP POLICY")));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("ALTER POLICY")));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("ALTER TABLE")
            && line.contains("ROW LEVEL SECURITY")));
}

#[test]
fn changed_rls_policy_release_remains_manual_review_only() {
    let dir = create_temp_dir("release-rls-policy-changed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/rls-policies/dbstate_slice2.sample_accounts.sample_accounts_public_read.sql"),
        "-- stale policy\nCREATE POLICY \"sample_accounts_public_read\" ON \"dbstate_slice2\".\"sample_accounts\" AS PERMISSIVE FOR SELECT TO PUBLIC USING (false);\n",
    )
    .expect("write changed policy");
    commit_all(&dir, "changed RLS policy");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "rlsPolicy:dbstate_slice2.sample_accounts.sample_accounts_public_read".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice33_changed_rls_policy",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_label, "Manual Review");
    assert!(report.plan_items[0]
        .operation_explanation
        .contains("changed RLS policies are manual-review only"));
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice33_changed_rls_policy.sql"),
    )
    .expect("sql");
    assert!(sql.contains("changed RLS policies are manual-review only"));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("DROP POLICY")));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("ALTER POLICY")));
}

#[test]
fn database_only_rls_policy_release_does_not_generate_drop_policy() {
    let dir = create_temp_dir("release-rls-policy-db-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    commit_all(&dir, "complete structure");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec![
                "rlsPolicy:dbstate_slice2.sample_accounts.sample_accounts_public_read".to_string(),
            ],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice33_database_only_rls_policy",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.plan_items[0].operation_label, "Database Only");
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice33_database_only_rls_policy.sql"),
    )
    .expect("sql");
    assert!(sql.contains("RLS policy exists only in target database"));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("DROP POLICY")));
    assert!(!sql
        .lines()
        .any(|line| line.trim_start().starts_with("ALTER POLICY")));
}

#[test]
fn release_candidate_selection_respects_selected_rls_policy_refs() {
    let dir = create_temp_dir("release-rls-policy-selection");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    let mut selected = sample_inventory().rls_policies[0].clone();
    selected.policy_name = "selected_policy".to_string();
    let mut unselected = sample_inventory().rls_policies[0].clone();
    unselected.policy_name = "unselected_policy".to_string();
    fs::write(
        dir.join(
            "database/objects/rls-policies/dbstate_slice2.sample_accounts.selected_policy.sql",
        ),
        render_rls_policy_sql(&selected),
    )
    .expect("write selected policy");
    fs::write(
        dir.join(
            "database/objects/rls-policies/dbstate_slice2.sample_accounts.unselected_policy.sql",
        ),
        render_rls_policy_sql(&unselected),
    )
    .expect("write unselected policy");
    commit_all(&dir, "repo-only RLS policies");
    let mut target_inventory = sample_inventory();
    target_inventory.rls_policies.clear();

    let report = release_postgres_with_inventory(
        &dir,
        &target_inventory,
        &ExportSelection::All,
        &PlanSelection::from_options(
            vec!["rlsPolicy:dbstate_slice2.sample_accounts.selected_policy".to_string()],
            Vec::new(),
        )
        .expect("plan selection"),
        "slice33_rls_policy_selection",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.included_objects,
        vec!["rlsPolicy:dbstate_slice2.sample_accounts.selected_policy".to_string()]
    );
    let sql = fs::read_to_string(
        dir.join("database/releases/objects/0001_slice33_rls_policy_selection.sql"),
    )
    .expect("sql");
    assert!(sql.contains("selected_policy"));
    assert!(!sql.contains("unselected_policy"));
}

#[test]
fn release_does_not_overwrite_existing_artifacts() {
    let dir = create_temp_dir("release-sequence");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::create_dir_all(dir.join("database/releases/objects")).expect("create objects release dir");
    fs::write(
        dir.join("database/releases/objects/0001_slice7.sql"),
        "-- existing\n",
    )
    .expect("write existing sql");
    fs::write(
        dir.join("database/releases/objects/0001_slice7.summary.md"),
        "existing\n",
    )
    .expect("write existing summary");
    fs::write(
        dir.join("database/releases/objects/0001_slice7.risk.json"),
        "{}\n",
    )
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
        .contains(&"database/releases/objects/0002_slice7.sql".to_string()));
}

#[test]
fn release_selected_refs_include_only_selected_candidate() {
    let dir = create_temp_dir("release-selected-only");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_one.sql"),
        render_schema_sql("local_one"),
    )
    .expect("write first repo-only schema");
    fs::write(
        dir.join("database/objects/schemas/local_two.sql"),
        render_schema_sql("local_two"),
    )
    .expect("write second repo-only schema");
    commit_all(&dir, "two repo-only schemas");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["schema:local_one".to_string()], Vec::new())
            .expect("plan selection"),
        "slice27_selected",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(
        report.included_objects,
        vec!["schema:local_one".to_string()]
    );
    assert!(!report
        .included_objects
        .contains(&"schema:local_two".to_string()));
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice27_selected.sql"))
        .expect("read sql");
    assert!(sql.contains("schema:local_one"));
    assert!(sql.contains("CREATE SCHEMA IF NOT EXISTS \"local_one\";"));
    assert!(!sql.contains("schema:local_two"));
    assert!(!sql.contains("CREATE SCHEMA IF NOT EXISTS \"local_two\";"));
}

#[test]
fn release_selected_ref_not_in_current_plan_is_rejected_safely() {
    let dir = create_temp_dir("release-selected-missing");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_one.sql"),
        render_schema_sql("local_one"),
    )
    .expect("write repo-only schema");
    commit_all(&dir, "one repo-only schema");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["schema:missing_selected".to_string()], Vec::new())
            .expect("plan selection"),
        "slice27_missing",
        false,
    );

    assert!(!report.success);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("was not found in the selected compare scope")));
    assert!(report.created_artifacts.is_empty());
    assert!(!dir
        .join("database/releases/objects/0001_slice27_missing.sql")
        .exists());
}

#[test]
fn release_service_rejects_empty_selected_object_refs() {
    let dir = create_temp_dir("release-service-empty-selection");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let body = format!(
        r#"{{ "repositoryPath": "{}", "releaseName": "slice27_empty", "scope": "all", "selectedObjectRefs": [] }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response("POST", "/api/v1/postgres/release/preview", &body, &dir);

    assert_eq!(response.status_code, 400);
    assert!(response
        .body
        .contains("Select at least one release candidate"));
    assert!(!dir
        .join("database/releases/objects/0001_slice27_empty.sql")
        .exists());
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
    let blocked = report
        .blocked_items
        .iter()
        .find(|item| item.object_ref == "table:dbstate_slice2.sample_accounts")
        .expect("blocked table item");
    assert_eq!(blocked.operation_kind, "blocked");
    assert_eq!(blocked.safety_badge, "Blocked");
    assert_eq!(blocked.safety_level, "blocked");
    assert!(blocked
        .operation_explanation
        .contains("Release artifact generation is blocked"));
    assert!(!blocked.operation_reasons.is_empty());
}

#[test]
fn additive_nullable_column_release_generates_review_only_add_column_sql() {
    let dir = create_temp_dir("release-add-column-nullable");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let mut repository_columns = sample_inventory().columns;
    repository_columns.push(ColumnInfo {
        schema_name: "dbstate_slice2".to_string(),
        table_name: "sample_accounts".to_string(),
        column_name: "notes".to_string(),
        ordinal_position: 5,
        data_type: "text".to_string(),
        is_nullable: true,
        has_default: false,
        default_expression: None,
    });
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        render_table_sql("dbstate_slice2", "sample_accounts", &repository_columns),
    )
    .expect("write additive table");
    commit_all(&dir, "additive table");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
        "slice24_nullable",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    let json = report.to_json();
    assert!(json.contains("\"operationKind\":\"additiveAddColumnReviewSql\""));
    assert!(json.contains("\"safetyBadge\":\"Additive ADD COLUMN\""));
    assert!(json.contains("review-only ADD COLUMN"));
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice24_nullable.sql"))
        .expect("read sql");
    assert!(sql.contains("-- Review-only additive column suggestion."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("-- Review before applying manually outside DbState."));
    assert!(sql.contains(
        "ALTER TABLE \"dbstate_slice2\".\"sample_accounts\"\n    ADD COLUMN \"notes\" text;"
    ));
    for forbidden in [
        "DROP TABLE",
        "DROP SCHEMA",
        "DROP COLUMN",
        "ALTER TABLE DROP",
        "TRUNCATE",
        "DELETE FROM",
        "UPDATE ",
        "INSERT INTO",
        "MERGE",
    ] {
        assert!(!sql.contains(forbidden), "forbidden SQL found: {forbidden}");
    }
}

#[test]
fn not_null_column_without_default_release_stays_manual_review_only() {
    let dir = create_temp_dir("release-add-column-not-null-no-default");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let mut repository_columns = sample_inventory().columns;
    repository_columns.push(ColumnInfo {
        schema_name: "dbstate_slice2".to_string(),
        table_name: "sample_accounts".to_string(),
        column_name: "required_code".to_string(),
        ordinal_position: 5,
        data_type: "text".to_string(),
        is_nullable: false,
        has_default: false,
        default_expression: None,
    });
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        render_table_sql("dbstate_slice2", "sample_accounts", &repository_columns),
    )
    .expect("write unsafe additive table");
    commit_all(&dir, "unsafe additive table");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
        "slice24_required",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    let json = report.to_json();
    assert!(json.contains("\"operationKind\":\"manualReviewRequired\""));
    assert!(json.contains("\"safetyBadge\":\"Manual Review\""));
    assert!(json.contains("NOT NULL with no default"));
    assert!(json.contains("manual review"));
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice24_required.sql"))
        .expect("read sql");
    assert!(sql.contains("NOT NULL with no default"));
    assert!(sql.contains("manual"));
    assert!(!sql.contains("ADD COLUMN \"required_code\""));
}

#[test]
fn changed_existing_column_release_stays_manual_review_only() {
    let dir = create_temp_dir("release-changed-existing-column");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let mut repository_columns = sample_inventory().columns;
    let display_name = repository_columns
        .iter_mut()
        .find(|column| column.column_name == "display_name")
        .expect("display_name column");
    display_name.data_type = "integer".to_string();
    fs::write(
        dir.join("database/objects/schemas/dbstate_slice2.sql"),
        render_schema_sql("dbstate_slice2"),
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        render_table_sql("dbstate_slice2", "sample_accounts", &repository_columns),
    )
    .expect("write changed table");
    commit_all(&dir, "changed existing column");

    let report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
        "slice24_changed",
        false,
    );

    assert!(report.success, "{:?}", report.errors);
    let json = report.to_json();
    assert!(json.contains("\"operationKind\":\"manualReviewRequired\""));
    assert!(json.contains("\"safetyBadge\":\"Manual Review\""));
    assert!(json.contains("existing-column changes are manual-review only"));
    assert!(!json.contains("\"safetyBadge\":\"Additive ADD COLUMN\""));
    let sql = fs::read_to_string(dir.join("database/releases/objects/0001_slice24_changed.sql"))
        .expect("read sql");
    assert!(sql.contains("difference is not clearly additive"));
    assert!(sql.contains("existing column \"display_name\" differs"));
    assert!(!sql.contains("\nALTER TABLE "));
    assert!(!sql.contains("ALTER COLUMN"));
}

#[test]
fn release_json_includes_expected_fields_and_no_secrets() {
    let mut report = empty_release_report(true);
    report.release_name = "slice7".to_string();
    report.release_scope = "all".to_string();
    report
        .planned_artifacts
        .push("database/releases/objects/0001_slice7.sql".to_string());
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
fn reference_registry_parser_accepts_slice34_setup_aliases() {
    let yaml = "tables:
  - schema: public
    name: countries
    keyColumns:
      - country_code
    ignoredColumns:
      - updated_at
    maskedColumns: []
";

    let registry = parse_reference_data_registry(yaml).expect("registry aliases");

    assert_eq!(registry.version, 1);
    assert_eq!(registry.tables.len(), 1);
    assert_eq!(registry.tables[0].name, "public.countries");
    assert_eq!(registry.tables[0].file, "tables/public.countries.yml");
    assert_eq!(
        registry.tables[0].key_columns,
        vec!["country_code".to_string()]
    );
    assert_eq!(
        registry.tables[0].ignore_columns,
        vec!["updated_at".to_string()]
    );
    assert!(registry.tables[0].masked_columns.is_empty());
}

#[test]
fn reference_data_database_tables_json_includes_registry_status() {
    let report = ReferenceDataDatabaseTablesReport {
        command: "reference-data database-tables".to_string(),
        success: true,
        repository_path: "repo".to_string(),
        git_root: Some("repo".to_string()),
        is_git_repository: true,
        branch: Some("dev".to_string()),
        working_tree_status: WorkingTreeStatus::Clean,
        is_dirty: false,
        database_type: "postgresql".to_string(),
        tables: vec![ReferenceDataCandidateTable {
            schema: "public".to_string(),
            name: "country".to_string(),
            table_name: "public.country".to_string(),
            columns: Vec::new(),
            suggested_key_columns: vec!["country_id".to_string()],
            warnings: Vec::new(),
        }],
        configured_tables: vec![ReferenceDataConfiguredTableStatus {
            schema: "public".to_string(),
            name: "country".to_string(),
            table_name: "public.country".to_string(),
            file: "tables/public.country.yml".to_string(),
            key_columns: vec!["country_id".to_string()],
            ignored_columns: vec!["last_update".to_string()],
            masked_columns: vec!["secret_note".to_string()],
        }],
        warnings: Vec::new(),
        errors: Vec::new(),
    };
    let json = report.to_json();

    for expected in [
        "\"configuredTables\"",
        "\"tableName\":\"public.country\"",
        "\"keyColumns\":[\"country_id\"]",
        "\"ignoredColumns\":[\"last_update\"]",
        "\"maskedColumns\":[\"secret_note\"]",
    ] {
        assert!(
            json.contains(expected),
            "missing registry status JSON field {expected}"
        );
    }
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
fn reference_data_status_endpoint_reports_missing_valid_and_invalid_registry() {
    let missing = create_temp_dir("reference-data-status-missing");
    init_git_repo(&missing);
    fs::create_dir_all(missing.join("database/reference-data/tables")).expect("create ref dirs");

    let missing_response =
        service_response("POST", "/api/v1/reference-data/status", "{}", &missing);
    assert_eq!(missing_response.status_code, 200);
    assert!(missing_response
        .body
        .contains("\"status\":\"missingRegistry\""));
    assert!(missing_response.body.contains("\"repositoryPathUsed\""));
    assert!(missing_response.body.contains("\"registryExists\":false"));
    assert!(missing_response
        .body
        .contains("Reference-data registry was not found"));
    assert!(missing_response.body.contains("\"configuredTables\":[]"));

    let valid = create_temp_dir("reference-data-status-valid");
    init_git_repo(&valid);
    create_complete_structure(&valid);
    fs::write(
        valid.join("database/reference-data/dbstate.reference-data.yml"),
        valid_reference_registry_yaml(),
    )
    .expect("write registry");
    let valid_response = service_response("POST", "/api/v1/reference-data/status", "{}", &valid);
    assert_eq!(valid_response.status_code, 200);
    assert!(valid_response.body.contains("\"success\":true"));
    assert!(valid_response.body.contains("\"status\":\"ready\""));
    assert!(valid_response.body.contains("\"registryExists\":true"));
    assert!(valid_response.body.contains("\"repositoryPathUsed\""));
    assert!(valid_response.body.contains("\"schema\":\"dbstate_ref\""));
    assert!(valid_response.body.contains("\"name\":\"payment_methods\""));
    assert!(valid_response.body.contains("\"keyColumns\":[\"code\"]"));
    assert!(valid_response
        .body
        .contains("\"ignoredColumns\":[\"updated_at\"]"));
    assert!(valid_response
        .body
        .contains("\"maskedColumns\":[\"secret_note\"]"));

    let invalid = create_temp_dir("reference-data-status-invalid");
    init_git_repo(&invalid);
    create_complete_structure(&invalid);
    fs::write(
        invalid.join("database/reference-data/dbstate.reference-data.yml"),
        "tables:\n  - name: public.bad\n",
    )
    .expect("write invalid registry");
    let invalid_response =
        service_response("POST", "/api/v1/reference-data/status", "{}", &invalid);
    assert_eq!(invalid_response.status_code, 200);
    assert!(invalid_response.body.contains("\"success\":false"));
    assert!(invalid_response
        .body
        .contains("\"status\":\"invalidRegistry\""));
    assert!(invalid_response
        .body
        .contains("missing required field 'key'"));
    assert!(!invalid_response.body.contains("postgres://"));
}

#[test]
fn reference_data_status_endpoint_uses_provided_repository_path() {
    let fallback = create_temp_dir("reference-data-status-fallback");
    init_git_repo(&fallback);
    fs::create_dir_all(fallback.join("database/reference-data/tables")).expect("fallback dirs");

    let selected = create_temp_dir("reference-data-status-selected");
    init_git_repo(&selected);
    create_complete_structure(&selected);
    fs::write(
        selected.join("database/reference-data/dbstate.reference-data.yml"),
        valid_reference_registry_yaml(),
    )
    .expect("write selected registry");
    let body = format!(r#"{{ "repositoryPath": "{}" }}"#, display_path(&selected));

    let response = service_response("POST", "/api/v1/reference-data/status", &body, &fallback);

    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("\"success\":true"));
    assert!(response.body.contains("\"registryExists\":true"));
    assert!(response.body.contains("\"status\":\"ready\""));
    assert!(response
        .body
        .contains("\"registryPath\":\"database/reference-data/dbstate.reference-data.yml\""));
    assert!(response.body.contains(&escape_json(&display_path(
        &selected.canonicalize().unwrap()
    ))));
    assert!(!response
        .body
        .contains("Reference-data registry was not found"));
    assert!(!response
        .body
        .contains(&escape_json(&display_path(&fallback))));
}

#[test]
fn service_data_compare_selected_tables_are_safely_validated() {
    let dir = create_temp_dir("reference-data-service-selected-tables");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/reference-data/dbstate.reference-data.yml"),
        valid_reference_registry_yaml(),
    )
    .expect("write registry");

    let empty = service_response(
        "POST",
        "/api/v1/postgres/data-compare",
        r#"{ "postgresUrl": "postgres://user:secret@example.invalid/db", "selectedTables": [] }"#,
        &dir,
    );
    assert_eq!(empty.status_code, 400);
    assert!(empty
        .body
        .contains("Select at least one configured reference-data table"));
    assert!(!empty.body.contains("secret"));
    assert!(!empty.body.contains("postgres://"));

    let unconfigured = service_response(
        "POST",
        "/api/v1/postgres/data-compare",
        r#"{ "postgresUrl": "postgres://user:secret@example.invalid/db", "selectedTables": ["public.not_configured"] }"#,
        &dir,
    );
    assert_eq!(unconfigured.status_code, 200);
    assert!(unconfigured.body.contains("\"success\":false"));
    assert!(unconfigured
        .body
        .contains("Selected reference-data table is not configured"));
    assert!(!unconfigured.body.contains("secret"));
    assert!(!unconfigured.body.contains("postgres://"));
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
fn reference_table_file_parser_accepts_canonical_key_values_rows() {
    let registry = parse_reference_data_registry(
        "version: 1
tables:
  - schema: public
    name: country
    keyColumns:
      - country_id
    ignoredColumns:
      - last_update
    maskedColumns: []
",
    )
    .expect("registry");
    let config = registry.tables[0].clone();
    let yaml = "version: 1
table: public.country
rows:
  - key:
      country_id: 1
    values:
      country_id: 1
      country: Afghanistan
      last_update: \"2006-02-15 09:44:00+00\"
  - key:
      country_id: 2
    values:
      country_id: 2
      country: Algeria
      last_update: \"2006-02-15 09:44:00+00\"
";

    let state = parse_reference_data_table_state(yaml, &config).expect("canonical state");

    assert_eq!(state.table_name, "public.country");
    assert_eq!(state.key_columns, vec!["country_id".to_string()]);
    assert_eq!(state.rows.len(), 2);
    assert_eq!(
        reference_row_key(&state.rows[0], &config.key_columns).expect("row key"),
        "country_id=1"
    );
}

#[test]
fn reference_table_file_parser_accepts_scalar_key_rows() {
    let registry = parse_reference_data_registry(
        "version: 1
tables:
  - schema: public
    name: country
    keyColumns:
      - country_id
",
    )
    .expect("registry");
    let config = registry.tables[0].clone();
    let yaml = "version: 1
table: public.country
rows:
  - key: \"1\"
    values:
      country: Afghanistan
  - key: \"2\"
    values:
      country: Algeria
";

    let state = parse_reference_data_table_state(yaml, &config).expect("scalar key state");

    assert_eq!(
        reference_row_key(&state.rows[0], &config.key_columns).expect("row key"),
        "country_id=1"
    );
    assert_eq!(
        state.rows[0].values.get("country").cloned().flatten(),
        Some("Afghanistan".to_string())
    );
}

#[test]
fn reference_table_file_parser_accepts_flat_row_alias_from_registry_keys() {
    let registry = parse_reference_data_registry(
        "version: 1
tables:
  - schema: public
    name: country
    keyColumns:
      - country_id
",
    )
    .expect("registry");
    let config = registry.tables[0].clone();
    let yaml = "version: 1
table: public.country
rows:
  - country_id: 1
    country: Afghanistan
";

    let state = parse_reference_data_table_state(yaml, &config).expect("flat row alias");

    assert_eq!(
        reference_row_key(&state.rows[0], &config.key_columns).expect("row key"),
        "country_id=1"
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
fn reference_table_file_parser_missing_key_error_names_file_and_row() {
    let registry = parse_reference_data_registry(
        "version: 1
tables:
  - schema: public
    name: country
    keyColumns:
      - country_id
",
    )
    .expect("registry");
    let config = registry.tables[0].clone();
    let yaml = "version: 1
table: public.country
rows:
  - values:
      country_id: 1
      country: Afghanistan
";

    let error = parse_reference_data_table_state(yaml, &config).expect_err("missing key");

    assert!(error.contains("database/reference-data/tables/public.country.yml"));
    assert!(error.contains("row 1"));
    assert!(error.contains("missing required field 'key'"));
    assert!(error.contains("Expected row format: key + values"));
}

#[test]
fn reference_table_file_parser_missing_table_error_names_file_and_hint() {
    let registry = parse_reference_data_registry(
        "version: 1
tables:
  - schema: public
    name: country
    keyColumns:
      - country_id
",
    )
    .expect("registry");
    let config = registry.tables[0].clone();
    let yaml = "version: 1
rows: []
";

    let error = parse_reference_data_table_state(yaml, &config).expect_err("missing table");

    assert!(error.contains("database/reference-data/tables/public.country.yml"));
    assert!(error.contains("table"));
    assert!(error.contains("non-empty string") || error.contains("missing required field 'table'"));
    assert!(error.contains("Expected top-level field 'table: public.country'"));
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
    assert_eq!(
        different.key_values.get("code").map(String::as_str),
        Some("QRPH")
    );
    assert_eq!(
        different.repository_values.get("name").map(String::as_str),
        Some("QRPh Desired")
    );
    assert_eq!(
        different.database_values.get("name").map(String::as_str),
        Some("QRPh Live")
    );
    assert_eq!(
        different
            .repository_values
            .get("updated_at")
            .map(String::as_str),
        Some("[ignored]")
    );
    assert!(different
        .ignored_columns
        .contains(&"updated_at".to_string()));
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
    assert!(json.contains("\"keyValues\""));
    assert!(json.contains("\"repositoryValues\""));
    assert!(json.contains("\"databaseValues\""));
    assert!(json.contains("\"ignoredColumns\""));
    assert!(json.contains("[masked]"));
    assert!(!json.contains("repo-secret-cash"));
    assert!(!json.contains("database-secret-value"));
    assert!(!text.contains("repo-secret-cash"));
    assert!(!text.contains("database-secret-value"));
}

#[test]
fn slice34c_reference_data_review_script_preview_generates_insert_and_update_candidates() {
    let compare = reference_data_review_compare_fixture();
    let report =
        reference_data_review_script_preview_from_compare(&compare, "reference_data_review")
            .expect("review script preview");
    let sql = &report.script_content;

    assert!(report.success);
    assert_eq!(report.insert_count, 1);
    assert_eq!(report.update_count, 1);
    assert_eq!(report.database_only_count, 1);
    assert_eq!(report.delete_generated_count, 0);
    assert!(sql.contains("-- REVIEW ONLY."));
    assert!(sql.contains("-- DbState does not execute this SQL."));
    assert!(sql.contains("-- Source: Repository reference-data"));
    assert!(sql.contains("-- Target: PostgreSQL database"));
    assert!(sql.contains("INSERT INTO \"dbstate_ref\".\"payment_methods\""));
    assert!(sql.contains("\"code\", \"is_active\", \"name\", \"sort_order\""));
    assert!(sql.contains("'Wire''s Transfer'"));
    assert!(sql.contains("UPDATE \"dbstate_ref\".\"payment_methods\""));
    assert!(sql.contains("SET \"name\" = 'QRPh Desired'"));
    assert!(sql.contains("WHERE \"code\" = 'QRPH';"));
    assert!(!sql.contains("\"updated_at\""));
    assert!(!sql.contains("\"secret_note\""));
    assert!(!sql.contains("repo-secret"));
    assert!(!sql.contains("database-secret"));
}

#[test]
fn slice34c_database_only_rows_generate_manual_review_comments_not_delete() {
    let compare = reference_data_review_compare_fixture();
    let report =
        reference_data_review_script_preview_from_compare(&compare, "reference_data_review")
            .expect("review script preview");
    let sql = &report.script_content;

    assert_eq!(report.manual_review_count, 1);
    assert_eq!(report.database_only_count, 1);
    assert_eq!(report.delete_generated_count, 0);
    assert!(sql.contains("-- REVIEW REQUIRED: database-only reference-data row."));
    assert!(sql
        .contains("-- DbState does not generate DELETE for reference-data rows in Private Beta."));
    assert!(sql.contains("-- No DELETE generated."));
    assert!(sql.contains("public.city.country_id -> dbstate_ref.payment_methods.code"));
    assert!(!sql.contains("DELETE FROM"));
    assert!(!sql.contains("MERGE"));
    assert!(report.risk_content.contains("\"reviewOnly\":true"));
    assert!(report.risk_content.contains("\"executesSql\":false"));
    assert!(report.risk_content.contains("\"mutatesPostgres\":false"));
    assert!(report.risk_content.contains("\"deleteGenerated\":false"));
}

#[test]
fn slice34c_review_script_json_contract_is_safe_and_data_oriented() {
    let compare = reference_data_review_compare_fixture();
    let report =
        reference_data_review_script_preview_from_compare(&compare, "reference_data_review")
            .expect("review script preview");
    let json = report.to_json();

    for expected in [
        "\"scriptContent\"",
        "\"summaryContent\"",
        "\"riskContent\"",
        "\"manifestContent\"",
        "\"insertCount\":1",
        "\"updateCount\":1",
        "\"manualReviewCount\":1",
        "\"deleteGeneratedCount\":0",
        "database/releases/reference-data/0001_reference_data_review.reference-data.sql",
    ] {
        assert!(
            json.contains(expected),
            "missing review script JSON field {expected}"
        );
    }
    assert!(json.contains("[masked]") || !json.contains("secret_note"));
    assert!(!json.contains("repo-secret"));
    assert!(!json.contains("database-secret"));
    assert!(!json.contains("postgres://"));
}

#[test]
fn slice34c_review_script_write_requires_typed_confirmation_before_connection() {
    let dir = create_temp_dir("slice34c-review-script-confirmation");
    create_complete_structure(&dir);
    let body = format!(
        r#"{{ "repositoryPath": "{}", "postgresUrl": "postgres://user:secret@example.invalid/db", "selectedTables": ["public.payment_method"], "scriptName": "reference_data_review", "confirmationText": "WRONG" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response(
        "POST",
        "/api/v1/reference-data/review-script/write",
        &body,
        &dir,
    );

    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("GENERATE REVIEW DATA SCRIPT"));
    assert!(!dir
        .join("database/releases/reference-data/0001_reference_data_review.reference-data.sql")
        .exists());
}

#[test]
fn slice34c_review_script_write_creates_artifacts_under_releases_and_preview_reads_sql() {
    let dir = create_temp_dir("slice34c-review-script-write");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let mut compare = reference_data_review_compare_fixture();
    compare.repository_path = display_path(&dir);
    compare.git_root = Some(display_path(&dir));
    compare.is_git_repository = true;
    let report =
        reference_data_review_script_write_from_compare(&dir, &compare, "reference_data_review")
            .expect("write review script artifacts");

    assert!(report.success);
    assert_eq!(report.created_artifacts.len(), 4);
    for path in &report.created_artifacts {
        assert!(path.starts_with("database/releases/reference-data/"));
        assert!(dir.join(path).exists(), "missing generated artifact {path}");
    }
    assert!(dir
        .join("database/releases/reference-data/0001_reference_data_review.reference-data.sql")
        .exists());
    assert!(!dir
        .join("database/releases/0001_reference_data_review.reference-data.sql")
        .exists());
    assert!(!dir
        .join("database/reference-data/0001_reference_data_review.reference-data.sql")
        .exists());
    let body = format!(
        r#"{{ "repositoryPath": "{}", "artifactPath": "database/releases/reference-data/0001_reference_data_review.reference-data.sql" }}"#,
        escape_json(&display_path(&dir))
    );
    let preview = service_response("POST", "/api/v1/releases/artifact-preview", &body, &dir);

    assert_eq!(preview.status_code, 200);
    assert!(preview.body.contains("REVIEW ONLY"));
    assert!(!preview.body.contains("Apply"));
    assert!(!preview.body.contains("Execute"));
    assert!(!preview.body.contains("Sync to Database"));
}

#[test]
fn slice34e_reference_data_review_write_backfills_reference_data_release_folder() {
    let dir = create_temp_dir("slice34e-reference-data-release-folder");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::remove_dir_all(dir.join("database/releases/reference-data"))
        .expect("remove reference-data release dir");
    let mut compare = reference_data_review_compare_fixture();
    compare.repository_path = display_path(&dir);
    compare.git_root = Some(display_path(&dir));
    compare.is_git_repository = true;

    let report =
        reference_data_review_script_write_from_compare(&dir, &compare, "reference_data_review")
            .expect("write review script artifacts");

    assert!(report.success, "{:?}", report.errors);
    assert!(dir.join("database/releases/reference-data").is_dir());
    assert!(report
        .created_artifacts
        .iter()
        .all(|path| path.starts_with("database/releases/reference-data/")));
    assert!(dir
        .join("database/releases/reference-data/0001_reference_data_review.reference-data.sql")
        .exists());
}

#[test]
fn slice34d_release_artifact_sequences_are_independent_by_kind() {
    let dir = create_temp_dir("slice34d-independent-artifact-sequences");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    commit_all(&dir, "repo-only schema");

    let schema_report = release_postgres_with_inventory(
        &dir,
        &sample_inventory(),
        &ExportSelection::All,
        &PlanSelection::from_options(vec!["schema:local_only".to_string()], Vec::new())
            .expect("plan selection"),
        "shared_sequence",
        false,
    );
    assert!(schema_report.success, "{:?}", schema_report.errors);
    assert!(schema_report
        .created_artifacts
        .contains(&"database/releases/objects/0001_shared_sequence.sql".to_string()));

    let mut compare = reference_data_review_compare_fixture();
    compare.repository_path = display_path(&dir);
    compare.git_root = Some(display_path(&dir));
    compare.is_git_repository = true;
    let reference_report =
        reference_data_review_script_write_from_compare(&dir, &compare, "shared_sequence")
            .expect("write reference-data artifacts");

    assert!(reference_report.success, "{:?}", reference_report.errors);
    assert!(reference_report.created_artifacts.contains(
        &"database/releases/reference-data/0001_shared_sequence.reference-data.sql".to_string()
    ));
    assert!(dir
        .join("database/releases/objects/0001_shared_sequence.sql")
        .exists());
    assert!(dir
        .join("database/releases/reference-data/0001_shared_sequence.reference-data.sql")
        .exists());
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
        ("POST", "/api/v1/git/commit-message"),
        ("POST", "/api/v1/init/plan"),
        ("POST", "/api/v1/init/write"),
        ("POST", "/api/v1/postgres/inspect"),
        ("POST", "/api/v1/postgres/compare"),
        ("POST", "/api/v1/postgres/plan"),
        ("POST", "/api/v1/reference-data/status"),
        ("POST", "/api/v1/reference-data/database-tables"),
        ("POST", "/api/v1/reference-data/export/preview"),
        ("POST", "/api/v1/reference-data/export/write"),
        ("POST", "/api/v1/postgres/data-compare"),
        ("POST", "/api/v1/postgres/object-ddl"),
        ("POST", "/api/v1/repository/object-files/list"),
        ("POST", "/api/v1/repository/object-files/preview"),
        ("POST", "/api/v1/postgres/repository-sync/preview"),
        ("POST", "/api/v1/postgres/repository-sync/write"),
        ("POST", "/api/v1/postgres/release/preview"),
        ("POST", "/api/v1/postgres/release/write"),
        ("POST", "/api/v1/releases/artifact-preview"),
    ] {
        assert!(routes.contains(&expected), "missing route {expected:?}");
    }

    for (_, path) in routes {
        assert!(!path.contains("/api/v1/postgres/export"));
        assert!(!path.contains("/api/v1/postgres/sync"));
        assert!(!path.contains("apply"));
    }
}

#[test]
fn shell_control_service_routes_return_expected_json_contracts() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    env::remove_var("DBSTATE_POSTGRES_URL");
    let dir = create_temp_dir("shell-control-service-routes");
    init_git_repo(&dir);

    let health = service_response("GET", "/api/v1/health", "", &dir);
    assert_eq!(health.status_code, 200);
    assert_common_json_contract(&health.body);
    assert!(health.body.contains("\"service\":\"dbstate\""));

    let workspace = service_response("POST", "/api/v1/workspace/validate", "{}", &dir);
    assert_eq!(workspace.status_code, 200);
    assert_common_json_contract(&workspace.body);
    assert!(workspace.body.contains("dbstateProjectStatus"));
    assert!(workspace.body.contains("isGitRepository"));
    assert!(workspace.body.contains("missingPaths"));

    let repo = service_response("POST", "/api/v1/repo/status", "{}", &dir);
    assert_eq!(repo.status_code, 200);
    assert_common_json_contract(&repo.body);
    assert!(repo.body.contains("\"command\":\"repo status\""));

    let init_plan = service_response("POST", "/api/v1/init/plan", r#"{ "dryRun": true }"#, &dir);
    assert_eq!(init_plan.status_code, 200);
    assert_common_json_contract(&init_plan.body);
    assert!(init_plan.body.contains("\"command\":\"init\""));
    assert!(init_plan.body.contains("\"plannedCreates\""));

    let connection = service_response("POST", "/api/v1/connections/test", "{}", &dir);
    assert_eq!(connection.status_code, 400);
    assert_common_json_contract(&connection.body);
    assert!(connection.body.contains("Missing PostgreSQL connection"));
}

#[test]
fn release_artifact_preview_endpoint_reads_release_sql_artifact() {
    let dir = create_temp_dir("slice25-release-artifact-preview");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let artifact = dir.join("database/releases/0001_test.sql");
    let content = "-- DbState release artifact\n-- DbState does not execute this SQL.\nSELECT 1;\n";
    fs::write(&artifact, content).expect("write release artifact");

    let body = format!(
        r#"{{ "repositoryPath": "{}", "artifactPath": "database/releases/0001_test.sql" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response("POST", "/api/v1/releases/artifact-preview", &body, &dir);

    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("\"success\":true"));
    assert!(response.body.contains("\"artifactType\":\"sql\""));
    assert!(response.body.contains("\"fileName\":\"0001_test.sql\""));
    assert!(response.body.contains("\"truncated\":false"));
    assert!(response.body.contains(&escape_json(content)));
    assert_eq!(
        fs::read_to_string(&artifact).expect("read release artifact after preview"),
        content
    );
}

#[test]
fn slice34d_release_artifact_preview_accepts_nested_release_artifacts() {
    let dir = create_temp_dir("slice34d-nested-artifact-preview");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::create_dir_all(dir.join("database/releases/objects")).expect("create objects dir");
    fs::create_dir_all(dir.join("database/releases/reference-data"))
        .expect("create reference-data dir");
    let artifacts = [
        (
            "database/releases/objects/0001_schema.sql",
            "sql",
            "-- schema sql\n",
        ),
        (
            "database/releases/reference-data/0001_reference.reference-data.sql",
            "sql",
            "-- reference data sql\n",
        ),
        (
            "database/releases/reference-data/0001_reference.reference-data.summary.md",
            "summary",
            "# summary\n",
        ),
        (
            "database/releases/reference-data/0001_reference.reference-data.risk.json",
            "risk",
            "{}\n",
        ),
        (
            "database/releases/reference-data/0001_reference.reference-data.manifest.json",
            "manifest",
            "{}\n",
        ),
    ];
    for (relative_path, expected_type, content) in artifacts {
        fs::write(dir.join(relative_path), content).expect("write nested artifact");
        let body = format!(
            r#"{{ "repositoryPath": "{}", "artifactPath": "{}" }}"#,
            escape_json(&display_path(&dir)),
            relative_path
        );
        let response = service_response("POST", "/api/v1/releases/artifact-preview", &body, &dir);
        assert_eq!(response.status_code, 200, "{}", response.body);
        assert!(response
            .body
            .contains(&format!("\"artifactType\":\"{}\"", expected_type)));
        assert!(response.body.contains(&escape_json(relative_path)));
        assert!(response.body.contains(&escape_json(content)));
    }
}

#[test]
fn release_artifact_preview_endpoint_rejects_traversal_and_absolute_paths() {
    let dir = create_temp_dir("slice25-release-artifact-preview-traversal");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(dir.join("database/releases/0001_test.sql"), "SELECT 1;\n")
        .expect("write release artifact");

    for artifact_path in [
        "../Cargo.toml",
        "database/releases/../../Cargo.toml",
        "database/releases/%2e%2e/Cargo.toml",
        "C:/Windows/win.ini",
        "/etc/passwd",
    ] {
        let body = format!(
            r#"{{ "repositoryPath": "{}", "artifactPath": "{}" }}"#,
            escape_json(&display_path(&dir)),
            escape_json(artifact_path)
        );
        let response = service_response("POST", "/api/v1/releases/artifact-preview", &body, &dir);
        assert_eq!(
            response.status_code, 400,
            "expected rejection for {artifact_path}, got {}",
            response.body
        );
        assert!(response.body.contains("\"success\":false"));
    }
}

#[test]
fn release_artifact_preview_endpoint_rejects_non_release_paths() {
    let dir = create_temp_dir("slice25-release-artifact-preview-non-release");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::create_dir_all(dir.join("database/objects/schemas")).expect("create schemas");
    fs::write(
        dir.join("database/objects/schemas/public.sql"),
        "CREATE SCHEMA public;\n",
    )
    .expect("write object file");

    let body = format!(
        r#"{{ "repositoryPath": "{}", "artifactPath": "database/objects/schemas/public.sql" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response("POST", "/api/v1/releases/artifact-preview", &body, &dir);

    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("\"success\":false"));
    assert!(response.body.contains("database/releases"));
}

#[test]
fn release_artifact_preview_endpoint_rejects_unsupported_extension() {
    let dir = create_temp_dir("slice25-release-artifact-preview-extension");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(dir.join("database/releases/notes.txt"), "notes\n").expect("write notes");

    let body = format!(
        r#"{{ "repositoryPath": "{}", "artifactPath": "database/releases/notes.txt" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response("POST", "/api/v1/releases/artifact-preview", &body, &dir);

    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("\"success\":false"));
    assert!(response.body.contains(".sql"));
    assert_eq!(
        fs::read_to_string(dir.join("database/releases/notes.txt")).expect("read notes"),
        "notes\n"
    );
}

#[test]
fn repository_object_files_list_returns_empty_state_for_empty_objects_root() {
    let dir = create_temp_dir("repository-object-list-empty");
    init_git_repo(&dir);
    fs::create_dir_all(dir.join("database/objects")).expect("create objects root");

    let body = format!(
        r#"{{ "repositoryPath": "{}" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response("POST", "/api/v1/repository/object-files/list", &body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("\"root\":\"database/objects\""));
    assert!(response.body.contains("\"entries\":[]"));
    assert!(response.body.contains("\"readOnly\":true"));
    assert!(response.body.contains("\"sqlExecutionAvailable\":false"));
    assert!(response.body.contains("\"gitMutationAvailable\":false"));
}

#[test]
fn repository_object_files_list_returns_initialized_directories_without_sql_files() {
    let dir = create_temp_dir("repository-object-list-initialized-empty");
    init_git_repo(&dir);
    init_project(&dir, false).expect("initialize project");

    let body = format!(
        r#"{{ "repositoryPath": "{}" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response("POST", "/api/v1/repository/object-files/list", &body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    for expected in EXPECTED_PATHS
        .iter()
        .filter(|expected| expected.kind == PathKind::Directory)
    {
        assert!(
            response
                .body
                .contains(&format!("\"path\":\"{}\"", expected.relative)),
            "missing initialized directory {} in {}",
            expected.relative,
            response.body
        );
    }
    assert!(response.body.contains("\"kind\":\"directory\""));
    assert!(!response.body.contains("\"kind\":\"file\""));
    assert!(!response.body.contains(".sql"));
    assert!(response.body.contains("\"readOnly\":true"));
    assert!(response.body.contains("\"sqlExecutionAvailable\":false"));
    assert!(response.body.contains("\"gitMutationAvailable\":false"));
}

#[test]
fn repository_object_files_list_returns_sql_files_and_directories_in_stable_order() {
    let dir = create_temp_dir("repository-object-list-stable");
    init_git_repo(&dir);
    fs::create_dir_all(dir.join("database/objects/tables")).expect("create tables dir");
    fs::create_dir_all(dir.join("database/objects/schemas")).expect("create schemas dir");
    fs::create_dir_all(dir.join("database/objects/constraints/primary-keys"))
        .expect("create constraints dir");
    fs::create_dir_all(dir.join("database/releases")).expect("create releases dir");
    fs::write(
        dir.join("database/objects/tables/public.zeta.sql"),
        "CREATE TABLE zeta(id integer);\n",
    )
    .expect("write zeta");
    fs::write(
        dir.join("database/objects/tables/public.actor.SQL"),
        "CREATE TABLE actor(id integer);\n",
    )
    .expect("write uppercase sql");
    fs::write(
        dir.join("database/objects/tables/notes.md"),
        "not a sql object\n",
    )
    .expect("write ignored file");
    fs::write(
        dir.join("database/objects/schemas/public.sql"),
        "CREATE SCHEMA public;\n",
    )
    .expect("write schema");
    fs::write(
        dir.join("database/objects/constraints/primary-keys/public.actor.actor_pkey.sql"),
        "ALTER TABLE public.actor ADD CONSTRAINT actor_pkey PRIMARY KEY (id);\n",
    )
    .expect("write constraint");
    fs::write(dir.join("database/releases/ignored.sql"), "SELECT 1;\n").expect("write release");

    let body = format!(
        r#"{{ "repositoryPath": "{}" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response("POST", "/api/v1/repository/object-files/list", &body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert!(response
        .body
        .contains("\"path\":\"database/objects/constraints\""));
    assert!(response
        .body
        .contains("\"path\":\"database/objects/schemas\""));
    assert!(response
        .body
        .contains("\"path\":\"database/objects/tables\""));
    assert!(response
        .body
        .contains("\"path\":\"database/objects/tables/public.actor.SQL\""));
    assert!(response.body.contains("\"objectCategory\":\"table\""));
    assert!(response.body.contains(
        "\"path\":\"database/objects/constraints/primary-keys/public.actor.actor_pkey.sql\""
    ));
    assert!(!response.body.contains("notes.md"));
    assert!(!response.body.contains("database/releases/ignored.sql"));

    let constraints_index = response
        .body
        .find("database/objects/constraints")
        .expect("constraints listed");
    let schemas_index = response
        .body
        .find("database/objects/schemas")
        .expect("schemas listed");
    let tables_index = response
        .body
        .find("database/objects/tables/")
        .expect("tables listed");
    assert!(constraints_index < schemas_index);
    assert!(schemas_index < tables_index);
    let actor_index = response
        .body
        .find("database/objects/tables/public.actor.SQL")
        .expect("actor listed");
    let zeta_index = response
        .body
        .find("database/objects/tables/public.zeta.sql")
        .expect("zeta listed");
    assert!(actor_index < zeta_index);
}

#[test]
fn repository_object_file_preview_reads_sql_under_database_objects_only() {
    let dir = create_temp_dir("repository-object-preview-table");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let content =
        "-- DbState PostgreSQL desired-state object\nCREATE TABLE public.actor(id integer);\n";
    fs::write(
        dir.join("database/objects/tables/public.actor.sql"),
        content,
    )
    .expect("write table");

    let body = format!(
        r#"{{ "repositoryPath": "{}", "path": "database/objects/tables/public.actor.sql" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response(
        "POST",
        "/api/v1/repository/object-files/preview",
        &body,
        &dir,
    );

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response
        .body
        .contains("\"path\":\"database/objects/tables/public.actor.sql\""));
    assert!(response.body.contains("\"fileName\":\"public.actor.sql\""));
    assert!(response.body.contains("\"objectCategory\":\"table\""));
    assert!(response.body.contains("\"readOnly\":true"));
    assert!(response.body.contains("\"sqlExecutionAvailable\":false"));
    assert!(response.body.contains(&escape_json(content)));
    assert_eq!(
        fs::read_to_string(dir.join("database/objects/tables/public.actor.sql"))
            .expect("read previewed file"),
        content
    );
}

#[test]
fn repository_object_file_preview_accepts_windows_separators_for_object_paths() {
    let dir = create_temp_dir("repository-object-preview-windows-path");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let content = "CREATE SCHEMA public;\n";
    fs::write(dir.join("database/objects/schemas/public.sql"), content).expect("write schema");

    let body = format!(
        r#"{{ "repositoryPath": "{}", "path": "database\\objects\\schemas\\public.sql" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response(
        "POST",
        "/api/v1/repository/object-files/preview",
        &body,
        &dir,
    );

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert!(response
        .body
        .contains("\"path\":\"database/objects/schemas/public.sql\""));
    assert!(response.body.contains(&escape_json(content)));
}

#[test]
fn repository_object_file_preview_rejects_unsafe_paths_and_file_types() {
    let dir = create_temp_dir("repository-object-preview-rejects");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/tables/public.actor.sql"),
        "CREATE TABLE public.actor(id integer);\n",
    )
    .expect("write table");
    fs::write(dir.join("database/releases/0001.sql"), "SELECT 1;\n").expect("write release");
    fs::write(
        dir.join("database/objects/tables/public.actor.txt"),
        "text\n",
    )
    .expect("write txt");

    for unsafe_path in [
        "..\\..\\secrets.txt",
        "database\\releases\\0001.sql",
        "database/objects/../../.git/config",
        "database/objects/%2e%2e/.git/config",
        "database/objects/tables/public.actor.txt",
        "database/objects/tables/missing.sql",
        "database/objects/tables",
        ".git/config",
        "C:\\Windows\\System32\\drivers\\etc\\hosts",
        "\\\\server\\share\\file.sql",
    ] {
        let body = format!(
            r#"{{ "repositoryPath": "{}", "path": "{}" }}"#,
            escape_json(&display_path(&dir)),
            escape_json(unsafe_path)
        );
        let response = service_response(
            "POST",
            "/api/v1/repository/object-files/preview",
            &body,
            &dir,
        );
        assert_eq!(
            response.status_code, 400,
            "expected rejection for {unsafe_path}, got {}",
            response.body
        );
        assert!(response.body.contains("\"success\":false"));
        assert!(!response.body.contains("postgres://"));
    }
}

#[test]
fn repository_object_file_preview_rejects_oversized_sql_file() {
    let dir = create_temp_dir("repository-object-preview-oversized");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let large_content =
        "x".repeat(crate::repository_browser::REPOSITORY_OBJECT_PREVIEW_MAX_BYTES as usize + 1);
    fs::write(
        dir.join("database/objects/tables/public.large.sql"),
        large_content,
    )
    .expect("write large file");

    let body = format!(
        r#"{{ "repositoryPath": "{}", "path": "database/objects/tables/public.large.sql" }}"#,
        escape_json(&display_path(&dir))
    );
    let response = service_response(
        "POST",
        "/api/v1/repository/object-files/preview",
        &body,
        &dir,
    );

    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("1 MiB"));
    assert!(response.body.contains("\"success\":false"));
}

#[test]
fn repository_object_files_endpoints_reject_invalid_workspaces_and_missing_objects_root() {
    let non_git = create_temp_dir("repository-object-list-non-git");
    let response = service_response(
        "POST",
        "/api/v1/repository/object-files/list",
        "{}",
        &non_git,
    );
    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("local Git working tree"));

    let missing_objects = create_temp_dir("repository-object-list-missing-root");
    init_git_repo(&missing_objects);
    let response = service_response(
        "POST",
        "/api/v1/repository/object-files/list",
        "{}",
        &missing_objects,
    );
    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("database/objects"));
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
fn database_state_ci_guidance_page_is_documentation_only() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "data-testid=\"tab-database-state-ci\"",
        "10. Database State CI",
        "id=\"step-database-state-ci\"",
        "data-testid=\"database-state-ci-page\"",
        "dbstate ci validate --repository",
        "--postgres-url",
        "--disposable",
        "Run text CI",
        "Run JSON CI",
        "Run Markdown report CI",
        "docker run --name \" + containerName + \" `",
        "pg_isready -U postgres -d \" + disposableDatabase",
        "docker rm -f \" + containerName + \" 2>$null",
        "dbstate.exe",
        "C:\\DbState\\YourDatabaseRepo",
        "C:\\DbState\\dbstate-ci\\database-state-ci-report.md",
        "Source profile",
        "Source database",
        "Disposable validation database",
        "Disposable container",
        "Disposable PostgreSQL URL",
        "The selected connection profile identifies the source database you are versioning.",
        "Database State CI does not run against that database.",
        "These commands create a separate disposable PostgreSQL database and run CI there.",
        "your_database_ci_validation",
        "dbstate-ci-your-database",
        "$DbStateCiPostgresPassword",
        "<DISPOSABLE_POSTGRES_PASSWORD>",
        "DbState does not store this value.",
        "POSTGRES_PASSWORD=\" + passwordVariable",
        "postgres://postgres:\" + passwordVariable + \"@127.0.0.1:55432/\" + disposableDatabase",
        "const passwordVariable = \"$DbStateCiPostgresPassword\";",
        "databaseStateCiSafeDatabaseName",
        "databaseStateCiSafeSlug",
        "const sourceProfile = selectedProfile();",
        "sourceProfile.database",
        "database-state-ci-report.md",
        "Release artifacts are not executed",
        "Reference-data review scripts are not executed",
        "The web app does not run CI",
        "I am using a disposable PostgreSQL database.",
        "I am not pointing at development, staging, production, or a shared database.",
        "Compare-back drift details",
        "ignored PostgreSQL bootstrap public schema grant noise during CI compare-back",
        "data-copy-target=\"database-state-ci-start-command\"",
        "data-copy-target=\"database-state-ci-text-command\"",
        "data-copy-target=\"database-state-ci-json-command\"",
        "data-copy-target=\"database-state-ci-report-command\"",
        "data-copy-target=\"database-state-ci-clean-command\"",
        "updateDatabaseStateCiCommands",
        "document.getElementById(\"workspace-path\").addEventListener(\"input\", function ()",
        "state.repositoryObjectFilesLoaded = false;",
        "updateDatabaseStateCiCommands();",
    ] {
        assert!(
            combined.contains(expected),
            "missing Database State CI guidance contract {expected}"
        );
    }

    for forbidden in [
        "D:\\SourceCodes\\DbState",
        "D:\\DbState\\pagila",
        "POSTGRES_PASSWORD=postgres",
        "postgres://postgres:postgres@",
        "data-action=\"ci-validate\"",
        "Run CI</button>",
        "Apply / Execute",
        "Sync to Database",
        "Apply to Database",
        "Execute SQL",
        "localStorage",
        "sessionStorage",
        "save password",
        "remember password",
    ] {
        assert!(
            !combined.contains(forbidden),
            "Database State CI guidance must not expose forbidden UI or persistence {forbidden}"
        );
    }
}

#[test]
fn database_state_ci_guidance_adds_no_backend_execution_endpoint() {
    let routes = service_route_definitions();

    assert!(!routes
        .iter()
        .any(|(_, path)| path.contains("/ci") || path.contains("ci/validate")));
    assert!(!ui_js().contains("ciValidate"));
    assert!(!ui_js().contains("/api/v1/ci"));
}

#[test]
fn database_state_ci_page_preserves_existing_primary_pages() {
    let html = ui_html();
    let combined = format!("{html}\n{}", ui_js());

    for expected in [
        "Schema Compare: Repository to Database",
        "Schema Compare: Database to Repository",
        "Object Diff",
        "Release Plan",
        "Git Workflow",
        "Reference Data Compare: Repository to Database",
        "Reference Data Compare: Database to Repository",
        "Reference-Data Row Data Diff",
    ] {
        assert!(
            combined.contains(expected),
            "existing page contract regressed: {expected}"
        );
    }
}

#[test]
fn private_beta_ui_polish_contract_is_present() {
    let html = ui_html();
    let css = ui_css();
    let js = ui_js();
    let combined = format!("{html}\n{css}\n{js}");

    assert!(html.contains("data-testid=\"workspace-check-status\""));
    assert!(html.contains("data-action=\"check-status\""));
    assert!(html.contains("Check Status"));
    assert!(!html.contains("data-testid=\"workspace-check\""));
    assert!(!html.contains("data-testid=\"workspace-repo-status\""));
    assert!(!html.contains(">Check Workspace</button>"));
    assert!(!html.contains(">Repo Status</button>"));
    assert!(js.contains("document.querySelector(\"[data-action='check-status']\")"));
    assert!(js.contains("runCheckStatus();"));
    assert!(js.contains("requestJson(shellActionEndpointByAction[\"workspace-status\"]"));
    assert!(js.contains("requestJson(shellActionEndpointByAction[\"repo-status\"]"));

    for expected in [
        "data-action=\"directory-roots\"",
        "data-action=\"directory-up\"",
        "data-action=\"directory-refresh\"",
        "data-action=\"directory-select\"",
        "textOrEmpty(root.name).toLowerCase() === \"service working directory\"",
        ".directory-entry.directory-entry-selected",
        "#directory-picker-path.directory-entry-selected",
        "classList.add(\"directory-entry-selected\")",
    ] {
        assert!(
            combined.contains(expected),
            "missing directory picker polish contract {expected}"
        );
    }

    for expected in [
        "data-testid=\"operation-alert\"",
        "id=\"operation-alert-title\"",
        "id=\"operation-alert-status\"",
        "id=\"operation-alert-warnings\"",
        "id=\"operation-alert-errors\"",
        "id=\"operation-alert-message\"",
        ".operation-alert-success",
        ".operation-alert-warning",
        ".operation-alert-error",
        "function updateOperationAlert",
        "updateOperationAlert(label, data);",
        "Running \" + label + \"...",
        "id=\"last-operation\"",
    ] {
        assert!(
            combined.contains(expected),
            "missing operation alert/status contract {expected}"
        );
    }

    assert!(!html.contains("aria-label=\"Object type filters\""));
    assert!(!html.contains("indexes future"));
    assert!(!html.contains("views future"));
    assert!(!html.contains("functions future"));
    assert!(!html.contains("triggers future"));
    assert!(html.contains("for=\"compare-schema\""));
    assert!(html.contains("for=\"compare-table\""));
    assert!(html.contains("for=\"plan-include\""));
    assert!(html.contains("for=\"plan-exclude\""));
    assert!(html.contains("for=\"data-scope\""));
    assert!(html.contains("for=\"data-table\""));
    assert!(html.contains("Refresh Database Inventory"));
    assert!(html.contains("Run Compare"));
    assert!(html.contains("Run Plan"));

    assert!(html.contains("Database State CI"));
    assert!(combined.contains("$DbStateCiPostgresPassword"));
    assert!(combined.contains("<DISPOSABLE_POSTGRES_PASSWORD>"));
    assert!(!combined.contains("POSTGRES_PASSWORD=postgres"));
    assert!(!combined.contains("postgres://postgres:postgres@"));
    assert!(!combined.contains("/api/v1/ci"));
    assert!(!combined.contains("Run CI</button>"));
    assert!(!combined.contains("Apply to Database"));
    assert!(!combined.contains("Execute SQL"));
    assert!(!combined.contains("Sync to Database"));
    assert!(!combined.contains("localStorage"));
    assert!(!combined.contains("sessionStorage"));
}

#[test]
fn warnings_panel_tracks_latest_operation_result_contract() {
    let html = ui_html();
    let js = ui_js();

    assert!(html.contains("data-testid=\"warnings-panel\""));
    assert!(html.contains("data-testid=\"warnings-list\""));
    assert!(html.contains("No warnings yet."));
    assert!(html.contains("data-testid=\"operation-alert\""));
    assert!(html.contains("id=\"operation-alert-warnings\""));

    assert!(js.contains("updateOperationAlert(label, data);"));
    assert!(js.contains("renderWarnings(data || {});"));
    assert!(js.contains("addMany(data.warnings, \"Warning\");"));
    assert!(js.contains("addMany(data.errors, \"Error\", \"error\");"));
    assert!(js.contains("addMany(data.dependencyWarnings, \"Dependency warning\");"));
    assert!(js.contains("addMany(data.blockedItems, \"Blocked item\", \"error\");"));
    assert!(js.contains("addMany(data.deferredObjectTypes, \"Deferred object type\");"));
    assert!(js.contains("const seenWarningKeys = new Set();"));
    assert!(js.contains("const warningKey = (kind || \"warning\") + \"|\" + text;"));
    assert!(js.contains("seenWarningKeys.has(warningKey)"));
    assert!(js.contains("seenWarningKeys.add(warningKey);"));
    assert!(js.contains("function reportedWarningCount()"));
    assert!(js.contains("data.warningCount"));
    assert!(js.contains("data.warningsCount"));
    assert!(js.contains(
        "Latest operation reported \" + count + \" warning(s). See the operation summary above."
    ));
    assert!(js.contains("target.textContent = \"No warnings yet.\""));

    let check_status = js_handler_for_action(js, "check-status");
    assert!(check_status.contains("runCheckStatus();"));
    assert!(js.contains("updateStatus(\"Check Status\", data);"));
    assert!(js.contains("warnings: warnings"));
    assert!(
        js.contains("concat(Array.isArray(workspaceData.warnings) ? workspaceData.warnings : [])")
    );
    assert!(js.contains("concat(Array.isArray(repoData.warnings) ? repoData.warnings : [])"));

    assert!(html.contains("Check Status"));
    assert!(!html.contains(">Check Workspace</button>"));
    assert!(!html.contains(">Repo Status</button>"));
    assert!(html.contains("Database State CI"));
    assert!(js.contains("$DbStateCiPostgresPassword"));
    assert!(!format!("{html}\n{js}").contains("POSTGRES_PASSWORD=postgres"));
    assert!(!format!("{html}\n{js}").contains("postgres://postgres:postgres@"));
    assert!(!js.contains("/api/v1/ci"));
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
        "/api/v1/reference-data/status",
        "/api/v1/reference-data/database-tables",
        "/api/v1/reference-data/export/preview",
        "/api/v1/reference-data/export/write",
        "/api/v1/postgres/data-compare",
        "/api/v1/reference-data/review-script/preview",
        "/api/v1/reference-data/review-script/write",
        "/api/v1/postgres/object-ddl",
        "/api/v1/repository/object-files/list",
        "/api/v1/repository/object-files/preview",
        "/api/v1/postgres/repository-sync/preview",
        "/api/v1/postgres/repository-sync/write",
        "/api/v1/postgres/release/preview",
        "/api/v1/postgres/release/write",
        "/api/v1/releases/artifact-preview",
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
        "/api/v1/release/",
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
    ] {
        assert!(
            !js.contains(forbidden),
            "UI JavaScript contains forbidden pattern {forbidden}"
        );
    }
}

#[test]
fn slice25_ui_contains_release_artifact_preview_contract() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    assert!(html.contains("data-testid=\"release-artifact-preview\""));
    assert!(html.contains("data-testid=\"release-artifact-preview-content\""));
    assert!(html.contains("Artifact Preview"));
    assert!(js.contains("releaseArtifactPreview: \"/api/v1/releases/artifact-preview\""));
    assert!(js.contains("data-action\", \"release-artifact-preview\""));
    assert!(js.contains("previewReleaseArtifact"));
    assert!(
        js.contains("Preview is available after Generate Release Artifact writes review files.")
    );
    assert!(js
        .contains("Dry-run completed. Review artifact paths were planned; no files were written."));
    assert!(js.contains("Release artifact dry-run failed. No files were written."));
    assert!(js.contains("Release artifacts generated successfully."));
    assert!(js.contains("Release artifact generation failed. No files were written."));
    assert!(js.contains("No artifact paths were returned."));
    assert!(js.contains("Zero artifacts were created; check the errors and warnings"));

    for forbidden_action in [
        "release-artifact-execute",
        "release-artifact-apply",
        "release-artifact-edit",
        "release-artifact-save",
        "release-artifact-delete",
        "release-artifact-git",
    ] {
        assert!(
            !combined.contains(forbidden_action),
            "release artifact preview UI exposes forbidden action {forbidden_action}"
        );
    }
}

#[test]
fn repository_files_ui_contract_is_read_only_and_route_scoped() {
    let html = ui_html();
    let css = ui_css();
    let js = ui_js();
    let combined = format!("{html}\n{css}\n{js}");

    for expected in [
        "data-testid=\"repository-files-tab\"",
        "data-testid=\"repository-files-panel\"",
        "data-testid=\"repository-files-refresh\"",
        "data-testid=\"repository-files-status\"",
        "data-testid=\"repository-files-loading\"",
        "data-testid=\"repository-files-loaded\"",
        "data-testid=\"repository-files-error\"",
        "data-testid=\"repository-files-tree\"",
        "data-testid=\"repository-files-empty-state\"",
        "data-testid=\"repository-file-selected-path\"",
        "data-testid=\"repository-file-preview\"",
        "data-testid=\"repository-file-preview-loading\"",
        "data-testid=\"repository-file-preview-error\"",
        "data-testid=\"repository-file-preview-read-only\"",
        "Repository Files",
        "Refresh Repository Files",
        "No repository object files were found.",
        "Select a SQL file to preview its contents.",
        "Read-only repository preview",
        "These are repository files, not live database objects.",
    ] {
        assert!(
            html.contains(expected),
            "missing repository files UI contract {expected}"
        );
    }

    assert!(js.contains("repositoryObjectFilesList: \"/api/v1/repository/object-files/list\""));
    assert!(js.contains("repositoryObjectFilePreview: \"/api/v1/repository/object-files/preview\""));
    assert!(js.contains("function repositoryObjectPathSlug(path)"));
    assert!(js.contains("return \"repository-file-row-\" + repositoryObjectPathSlug(path);"));
    assert!(js.contains("return \"repository-directory-row-\" + repositoryObjectPathSlug(path);"));
    assert!(js.contains("return \"repository-directory-path-\" + repositoryObjectPathSlug(path);"));
    assert!(js.contains("return \"repository-directory-name-\" + repositoryObjectPathSlug(path);"));
    assert!(js.contains("setRepositoryFilesState(\"loading\""));
    assert!(js.contains("setRepositoryFilesState(\"loaded\""));
    assert!(js.contains("setRepositoryFilesState(\"error\""));
    assert!(js.contains("data-action\", \"repository-file-preview\""));
    assert!(js.contains("refreshRepositoryObjectFiles"));
    assert!(js.contains("previewRepositoryObjectFile"));
    assert!(css.contains(".repository-files-layout"));
    assert!(css.contains(".repository-file-row.repository-file-selected"));

    for forbidden in [
        "repository-file-edit",
        "repository-file-save",
        "repository-file-delete",
        "repository-file-rename",
        "repository-file-move",
        "repository-file-execute",
        "/api/v1/repository/object-files/write",
        "/api/v1/repository/object-files/delete",
        "/api/v1/repository/object-files/execute",
    ] {
        assert!(
            !combined.contains(forbidden),
            "Repository Files UI exposes forbidden control or route {forbidden}"
        );
    }
}

#[test]
fn slice34e_release_plan_buttons_dispatch_for_schema_repository_to_database() {
    let html = ui_html();
    let js = ui_js();

    assert_eq!(html.matches("data-action=\"release-preview\"").count(), 1);
    assert_eq!(html.matches("data-action=\"release-write\"").count(), 1);
    assert_eq!(
        html.matches("data-testid=\"release-plan-dry-run\"").count(),
        1
    );
    assert_eq!(
        html.matches("data-testid=\"generate-release-artifact\"")
            .count(),
        1
    );
    assert!(html.contains(
        "<button type=\"button\" data-action=\"release-preview\" data-testid=\"release-plan-dry-run\">Dry-run Release Artifact</button>"
    ));
    assert!(html.contains(
        "<button type=\"button\" data-action=\"release-write\" data-testid=\"generate-release-artifact\" disabled>Generate Release Artifact</button>"
    ));

    assert!(js.contains(
        "document.querySelector(\"[data-action='release-preview']\").addEventListener(\"click\", function (event)"
    ));
    assert!(js.contains(
        "document.querySelector(\"[data-action='release-write']\").addEventListener(\"click\", function (event)"
    ));
    assert!(js.contains("event.preventDefault();"));
    assert!(js.contains("if (!isSchemaRepositoryToDatabaseMode(currentWorkflowMode())) {"));
    assert!(js.contains(
        "run(\"Release artifact dry-run\", approvedEndpoints.releasePreview, releaseBody(false), { step: \"release-plan\", releaseResult: true });"
    ));
    assert!(js.contains(
        "run(\"Generate Release Artifact\", approvedEndpoints.releaseWrite, releaseBody(true), { step: \"release-plan\", releaseResult: true });"
    ));
    assert!(js.contains("releasePlanTab.hidden = !releasePlanApplies"));
    assert!(js.contains("return isSchemaRepositoryToDatabaseMode(mode);"));
    assert!(!js.contains("currentWorkflowMode() !== \"compare\""));
}

#[test]
fn slice26_ui_contains_release_operation_badge_contract() {
    let html = ui_html();
    let css = ui_css();
    let js = ui_js();
    let combined = format!("{html}\n{css}\n{js}");

    assert!(html.contains("Operation / safety"));
    assert!(html.contains("Release operation badges"));
    assert!(html.contains("Review SQL"));
    assert!(html.contains("Additive ADD COLUMN"));
    assert!(html.contains("Manual Review"));
    assert!(html.contains("Blocked"));
    assert!(html.contains("DbState does not execute SQL"));
    assert!(css.contains(".release-operation-badge"));
    assert!(js.contains("operationExplanation"));
    assert!(js.contains("operationReasons"));
    assert!(js.contains("safetyBadge"));

    for forbidden_action in [
        "release-artifact-execute",
        "release-artifact-apply",
        "release-artifact-edit",
        "release-artifact-save",
        "release-artifact-delete",
        "release-artifact-git",
        "data-action=\"apply\"",
        "data-action=\"execute\"",
    ] {
        assert!(
            !combined.contains(forbidden_action),
            "release operation UI exposes forbidden action {forbidden_action}"
        );
    }
}

#[test]
fn slice27_ui_contains_release_candidate_selection_contract() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    assert!(html.contains("data-testid=\"release-candidate-selection-controls\""));
    assert!(html.contains("data-testid=\"release-selected-count\""));
    assert!(html.contains("data-testid=\"release-select-all-eligible\""));
    assert!(html.contains("data-testid=\"release-clear-selection\""));
    assert!(html.contains("Selected candidates only are included"));
    assert!(js.contains("selectedObjectRefs"));
    assert!(js.contains("releaseSelectedRefs"));
    assert!(js.contains("data-testid\", \"release-candidate-checkbox\""));
    assert!(js.contains("Select at least one release candidate."));
    assert!(js.contains("releaseCandidateEligible"));

    for forbidden_action in [
        "release-artifact-execute",
        "release-artifact-apply",
        "release-artifact-edit",
        "release-artifact-delete",
        "data-action=\"apply\"",
        "data-action=\"execute\"",
    ] {
        assert!(
            !combined.contains(forbidden_action),
            "release candidate selection UI exposes forbidden action {forbidden_action}"
        );
    }
}

#[test]
fn slice28a_ui_contains_related_object_comparison_contract() {
    let html = ui_html();
    let css = ui_css();
    let js = ui_js();
    let combined = format!("{html}\n{css}\n{js}");

    assert!(html.contains("Related Object Comparison"));
    assert!(html.contains("related-object-comparison-body"));
    assert!(html.contains("Source Related Objects"));
    assert!(html.contains("Target Related Objects"));
    assert!(html.contains("Constraints"));
    assert!(html.contains("Indexes"));
    assert!(css.contains(".related-object-status"));
    assert!(css.contains(".related-object-type-group"));
    assert!(css.contains(".related-object-type-table"));
    assert!(js.contains("relatedObjectComparisonRows"));
    assert!(js.contains("relatedObjectRowsByGroup"));
    assert!(js.contains("renderRelatedObjectComparison"));
    assert!(js.contains("relatedObjectComparisonStatus"));
    assert!(js.contains("No \" + sideLabel + \" \" + group.group + \" available."));
    assert!(js.contains("heading.textContent = group.group;"));
    assert!(!js.contains("row.group + \" - \" + row.name"));
    assert!(js.contains("\"Source only\""));
    assert!(js.contains("\"Target only\""));
    assert!(js.contains("\"In sync\""));
    assert!(js.contains("\"Different\""));
    assert!(js.contains("\"Not available\""));

    for forbidden_action in [
        "release-artifact-execute",
        "release-artifact-apply",
        "data-action=\"apply\"",
        "data-action=\"execute\"",
    ] {
        assert!(
            !combined.contains(forbidden_action),
            "related object UI exposes forbidden action {forbidden_action}"
        );
    }
}

#[test]
fn slice28a_related_object_panels_alignment_contract() {
    let html = ui_html();
    let css = ui_css();

    assert!(html.contains("related-objects-two-column"));
    assert!(html.contains("related-object-side-panel"));
    assert!(css.contains(".related-objects-grid"));
    assert!(css.contains("grid-template-columns: repeat(2, minmax(0, 1fr));"));
    assert!(css.contains("align-items: stretch;"));
    assert!(css.contains(".related-object-side-panel"));
    assert!(css.contains("flex-direction: column;"));
    assert!(css.contains(".related-object-type-group"));
    assert!(css.contains(".related-object-type-table"));
    assert!(css.contains("table-layout: fixed;"));
    assert!(css.contains("#related-objects-view[hidden]"));
}

#[test]
fn slice28a_release_candidate_table_readability_contract() {
    let html = ui_html();
    let css = ui_css();
    let js = ui_js();

    assert!(html.contains("release-candidates-table"));
    assert!(html.contains("release-candidate-select-col"));
    assert!(html.contains("release-candidate-explanation-col"));
    assert!(html.contains("release-candidate-reasons-col"));
    assert!(html.contains("release-candidate-warnings-col"));
    assert!(html.contains("data-testid=\"release-selected-count\""));
    assert!(html.contains("data-testid=\"release-select-all-eligible\""));
    assert!(html.contains("data-testid=\"release-clear-selection\""));
    assert!(css.contains(".release-candidates-table"));
    assert!(css.contains("table-layout: fixed;"));
    assert!(css.contains("white-space: normal;"));
    assert!(css.contains("overflow-wrap: anywhere;"));
    assert!(css.contains(".release-candidate-explanation-col"));
    assert!(css.contains(".release-candidate-reasons-col"));
    assert!(css.contains(".release-candidate-warnings-col"));
    assert!(js.contains("release-candidate-explanation-cell"));
    assert!(js.contains("release-candidate-reasons-cell"));
    assert!(js.contains("release-candidate-warnings-cell"));
}

#[test]
fn ui_contains_stable_playwright_demo_selectors() {
    let html = ui_html();
    let js = ui_js();
    let expected_html_selectors = [
        "data-testid=\"tab-workspace\"",
        "data-testid=\"workspace-path-input\"",
        "data-testid=\"workspace-browse\"",
        "data-testid=\"workspace-health\"",
        "data-testid=\"workspace-check-status\"",
        "data-testid=\"workspace-status\"",
        "data-testid=\"workspace-git-branch\"",
        "data-testid=\"workspace-protected-branch\"",
        "data-testid=\"workspace-project-status\"",
        "data-testid=\"operation-alert\"",
        "data-testid=\"workspace-init-plan\"",
        "data-testid=\"workspace-initialize-project\"",
        "data-testid=\"directory-picker\"",
        "data-testid=\"directory-picker-path-input\"",
        "data-testid=\"directory-picker-parent\"",
        "data-testid=\"directory-select-current\"",
        "data-testid=\"directory-picker-cancel\"",
        "data-testid=\"directory-picker-error\"",
        "data-testid=\"tab-source-target\"",
        "data-testid=\"workflow-mode\"",
        "data-testid=\"active-workflow-mode\"",
        "data-testid=\"connection-mode\"",
        "data-testid=\"profile-select\"",
        "data-testid=\"profile-password-input\"",
        "data-testid=\"source-connection-input\"",
        "data-testid=\"target-connection-input\"",
        "data-testid=\"source-target-run\"",
        "data-testid=\"database-connection-status\"",
        "data-testid=\"database-connection-success\"",
        "data-testid=\"database-connection-error\"",
        "data-testid=\"tab-compare-options\"",
        "data-testid=\"preview-repository-sync\"",
        "data-testid=\"repository-sync-preview-status\"",
        "data-testid=\"repository-sync-preview-loading\"",
        "data-testid=\"repository-sync-preview-success\"",
        "data-testid=\"repository-sync-preview-error\"",
        "data-testid=\"write-repository-changes\"",
        "data-testid=\"repository-write-confirmation-input\"",
        "data-testid=\"repository-write-preflight\"",
        "data-testid=\"repository-write-loading\"",
        "data-testid=\"repository-write-success\"",
        "data-testid=\"repository-write-error\"",
        "data-testid=\"repository-write-written-count\"",
        "data-testid=\"repository-write-skipped-count\"",
        "data-testid=\"tab-results\"",
        "data-testid=\"results-object-type-filter\"",
        "data-testid=\"results-status-filter\"",
        "data-testid=\"results-search-input\"",
        "data-testid=\"results-table\"",
        "data-testid=\"results-count\"",
        "data-testid=\"results-visible-row-count\"",
        "data-testid=\"results-included-count\"",
        "data-testid=\"results-clear-selection\"",
        "data-testid=\"results-selected-count\"",
        "data-testid=\"results-selection-status\"",
        "data-testid=\"results-selection-clearing\"",
        "data-testid=\"results-selection-cleared\"",
        "data-testid=\"results-selection-error\"",
        "data-testid=\"results-status-legend\"",
        "data-testid=\"comparison-summary\"",
        "data-testid=\"tab-object-diff\"",
        "data-testid=\"object-diff-panel\"",
        "data-testid=\"object-diff-object-ref\"",
        "data-testid=\"object-diff-object-type\"",
        "data-testid=\"object-diff-status\"",
        "data-testid=\"object-diff-loading\"",
        "data-testid=\"object-diff-error\"",
        "data-testid=\"object-diff-left\"",
        "data-testid=\"object-diff-right\"",
        "data-testid=\"object-diff-repository\"",
        "data-testid=\"object-diff-database\"",
        "data-testid=\"selected-json-item\"",
        "data-testid=\"repository-files-tab\"",
        "data-testid=\"repository-files-panel\"",
        "data-testid=\"repository-files-refresh\"",
        "data-testid=\"repository-files-tree\"",
        "data-testid=\"repository-files-empty-state\"",
        "data-testid=\"repository-file-selected-path\"",
        "data-testid=\"repository-file-preview\"",
        "data-testid=\"repository-file-preview-loading\"",
        "data-testid=\"repository-file-preview-error\"",
        "data-testid=\"repository-file-preview-read-only\"",
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
        "data-testid=\"tab-git-workflow\"",
        "data-testid=\"git-current-branch\"",
        "data-testid=\"git-default-branch\"",
        "data-testid=\"git-protected-branch-status\"",
        "data-testid=\"git-working-tree-status\"",
        "data-testid=\"git-detached-head-status\"",
    ];

    for selector in expected_html_selectors {
        assert!(
            html.contains(selector),
            "missing stable demo selector {selector}"
        );
    }

    for expected in [
            "function stableSelectorHash(value)",
            "function selectorSlug(value, fallback)",
            "function resultRowSelectorSlug(row)",
            "tr.setAttribute(\"data-testid\", \"results-row-\" + selectorSlug)",
            "include.setAttribute(\"data-testid\", \"results-include-\" + selectorSlug)",
            "\"results-row-status-\" + selectorSlug",
            "\"results-row-type-\" + selectorSlug",
            "\"results-row-path-\" + selectorSlug",
            "\"results-open-\" + selectorSlug",
            "button.setAttribute(\"data-testid\", directoryEntryTestId(root.path))",
            "row.setAttribute(\"data-testid\", directoryEntryTestId(directory.path))",
            "td.setAttribute(\"data-testid\", \"results-row-first-selectable\")",
            "function setObjectDiffDdlTestIds(direction)",
            "sourceDetail.setAttribute(\"data-testid\", direction.sourceDdlSide === \"database\" ? \"object-diff-database\" : \"object-diff-repository\")",
            "targetDetail.setAttribute(\"data-testid\", direction.targetDdlSide === \"repository\" ? \"object-diff-repository\" : \"object-diff-database\")",
            "byId(\"object-diff-object-ref\").textContent = resultRowIdentity(row)",
            "setHidden(\"database-connection-success\", stateName !== \"success\")",
            "setHidden(\"repository-sync-preview-success\", stateName !== \"success\")",
            "setHidden(\"repository-write-success\", stateName !== \"success\")",
            "function clearResultsSelection()",
            "state.included = new Set()",
            "body.include = selectedRepositorySyncPaths()",
            "byId(\"results-selected-count\").textContent = String(selectedPaths.length)",
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
fn schema_database_to_repository_selector_contract_is_documented_in_ui_source() {
    let html = ui_html();
    let js = ui_js();

    assert!(html.contains(r#"<option value="schemaDatabaseToRepository">Schema Compare: Database to Repository</option>"#));
    assert!(html.contains(r#"id="active-workflow-mode" data-testid="active-workflow-mode""#));
    assert!(js.contains(
        r#"byId("active-workflow-mode").setAttribute("data-workflow-mode", selectedMode)"#
    ));

    for object_ref in [
        "table:public.country",
        "table:public.actor",
        "constraint:primaryKey.public.country.country_pkey",
        "constraint:foreignKey.public.city.city_country_id_fkey",
        "function:public.last_updated()",
        "trigger:public.actor.last_updated",
        "grant:table.public.country.app_reader",
        "rlsPolicy:public.account.account_tenant_policy",
    ] {
        let readable = selector_readable_slug(object_ref);
        assert!(
            !readable.is_empty(),
            "object ref should produce readable selector slug: {object_ref}"
        );
    }

    assert!(js.contains("return textOrEmpty(row.objectRef)"));
    assert!(js.contains("|| textOrEmpty(row.relativePath)"));
    assert!(js.contains(
        "[row.objectType, row.schema, row.parentName, row.name].filter(Boolean).join(\":\")"
    ));
}

#[test]
fn selector_slugging_uses_deterministic_hash_suffix_for_collisions() {
    let js = ui_js();

    assert!(js.contains("let hash = 2166136261"));
    assert!(js.contains("hash = Math.imul(hash, 16777619) >>> 0"));
    assert!(js.contains("return base + \"-\" + stableSelectorHash(text);"));
    assert!(js.contains("function repositoryObjectPathSlug(path)"));
    assert!(js.contains("return selectorSlug(friendlyPath(path), \"repository-object\");"));

    let normalized_a = selector_readable_slug("table:public.a-b");
    let normalized_b = selector_readable_slug("table:public.a_b");
    assert_eq!(normalized_a, normalized_b);
    assert_ne!(
        stable_selector_hash("table:public.a-b"),
        stable_selector_hash("table:public.a_b")
    );
}

#[test]
fn password_selectors_do_not_derive_from_secret_values() {
    let html = ui_html();
    let js = ui_js();

    assert!(html
        .contains(r#"id="profile-password" type="password" data-testid="profile-password-input""#));
    assert!(!js.contains("profile-password-\" +"));
    assert!(!js.contains("profilePassword"));
    assert!(js.contains("const password = value(\"profile-password\")"));
    assert!(!js.contains("setAttribute(\"data-testid\", value(\"profile-password\")"));
}

#[test]
fn git_workflow_recommendation_selectors_expose_exact_machine_values() {
    let html = ui_html();
    let js = ui_js();

    for selector in [
        "data-testid=\"git-workflow-panel\"",
        "data-testid=\"git-workflow-status\"",
        "data-testid=\"git-workflow-ready-state\"",
        "data-testid=\"git-workflow-blocked-state\"",
        "data-testid=\"git-workflow-no-changes-state\"",
        "data-testid=\"git-workflow-loading-state\"",
        "data-testid=\"git-workflow-error-state\"",
        "data-testid=\"git-workflow-suggested-branch\"",
        "data-testid=\"git-workflow-suggested-branch-name\"",
        "data-testid=\"git-workflow-copy-branch-name\"",
        "data-testid=\"git-workflow-branch-reason\"",
        "data-testid=\"git-workflow-suggested-files\"",
        "data-testid=\"git-workflow-suggested-files-count\"",
        "data-testid=\"git-workflow-suggested-files-empty\"",
        "data-testid=\"git-workflow-suggested-commit-title\"",
        "data-testid=\"git-workflow-suggested-commit-body\"",
        "data-testid=\"git-workflow-copy-commit-title\"",
        "data-testid=\"git-workflow-copy-commit-message\"",
        "data-testid=\"git-current-branch\"",
        "data-testid=\"git-default-branch\"",
        "data-testid=\"git-protected-branch-status\"",
        "data-testid=\"git-working-tree-status\"",
        "data-testid=\"git-detached-head-status\"",
    ] {
        assert!(
            html.contains(selector),
            "missing Git Workflow selector {selector}"
        );
    }

    assert!(js.contains("const suggestedBranchName = writeSucceededOnWorkingBranch ? branch : branchCommand.replace"));
    assert!(js.contains(
        "byId(\"git-workflow-suggested-branch-name\").textContent = suggestedBranchName;"
    ));
    assert!(!js.contains("\"Suggested branch: \" + suggestedBranchName"));
    assert!(!js.contains("\"git switch -c \" + suggestedBranchName"));
    assert!(js.contains("byId(\"git-workflow-suggested-commit-title\").textContent = nextTitle;"));
    assert!(js.contains("byId(\"git-workflow-suggested-commit-body\").textContent = nextBody;"));
    assert!(js.contains("byId(\"git-workflow-suggested-commit-title\").textContent = title;"));
    assert!(js.contains("byId(\"git-workflow-suggested-commit-body\").textContent = body;"));
    assert!(!js.contains("\"Commit title: \" + nextTitle"));
    assert!(!js.contains("\"Commit body: \" + nextBody"));
    assert!(js.contains("code.textContent = path;"));
    assert!(
        js.contains("item.setAttribute(\"data-testid\", gitWorkflowSuggestedFileTestId(path));")
    );
    assert!(js
        .contains("code.setAttribute(\"data-testid\", gitWorkflowSuggestedFilePathTestId(path));"));
}

#[test]
fn git_workflow_suggested_file_selectors_reuse_repository_path_slugging() {
    let js = ui_js();

    assert!(
        js.contains("return \"git-workflow-suggested-file-\" + repositoryObjectPathSlug(path);")
    );
    assert!(js.contains(
        "return \"git-workflow-suggested-file-path-\" + repositoryObjectPathSlug(path);"
    ));
    assert!(js.contains("return \"repository-file-row-\" + repositoryObjectPathSlug(path);"));

    let representative_paths = [
        "database/objects/tables/public.country.sql",
        "database/objects/functions/public.calculate_total.sql",
        "database/objects/constraints/foreign-keys/public.city.country_fk.sql",
        "database/objects/grants/tables/public.country.app_reader.sql",
        "database/objects/rls-policies/public.customer.tenant_policy.sql",
        "database/objects/tables/Public.Mixed-Case_Name.sql",
    ];
    let mut slugs = std::collections::BTreeSet::new();
    for path in representative_paths {
        let slug = format!(
            "{}-{}",
            selector_readable_slug(path),
            stable_selector_hash(path)
        );
        assert!(
            slugs.insert(slug.clone()),
            "duplicate slug for {path}: {slug}"
        );
        assert!(
            slug.starts_with("database-objects-"),
            "repository object path should keep readable prefix: {slug}"
        );
    }

    let normalized_a = selector_readable_slug("database/objects/tables/public.a-b.sql");
    let normalized_b = selector_readable_slug("database/objects/tables/public.a_b.sql");
    assert_eq!(normalized_a, normalized_b);
    assert_ne!(
        stable_selector_hash("database/objects/tables/public.a-b.sql"),
        stable_selector_hash("database/objects/tables/public.a_b.sql")
    );
}

#[test]
fn git_workflow_state_selector_logic_covers_expected_states() {
    let js = ui_js();

    assert!(js.contains("function setGitWorkflowState(stateName, message)"));
    assert!(js.contains("status.setAttribute(\"data-state\", stateValue);"));
    assert!(js.contains("setHidden(\"git-workflow-ready-state\", stateValue !== \"ready\")"));
    assert!(js.contains("setHidden(\"git-workflow-blocked-state\", stateValue !== \"blocked\")"));
    assert!(
        js.contains("setHidden(\"git-workflow-no-changes-state\", stateValue !== \"no-changes\")")
    );
    assert!(js.contains("setHidden(\"git-workflow-loading-state\", stateValue !== \"loading\")"));
    assert!(js.contains("setHidden(\"git-workflow-error-state\", stateValue !== \"error\")"));
    assert!(js.contains("if (blocked)"));
    assert!(js.contains("protectedBranch || errors.toLowerCase().indexOf(\"blocked\") >= 0"));
    assert!(js.contains("} else if (data && data.success === false)"));
    assert!(js.contains("} else if (suggestedPaths.length)"));
    assert!(js.contains("setGitWorkflowState(\"no-changes\", \"No DbState files are recommended for staging yet.\");"));
    assert!(js.contains("setGitWorkflowState(\"loading\", \"Generating semantic commit message from staged changes.\");"));
    assert!(js.contains("byId(\"git-handoff-detached-head\").textContent = branch === \"unknown\" ? \"yes\" : \"no\";"));
}

#[test]
fn cycle3_object_diff_demo_script_is_fixed_scope_and_secret_safe() {
    let script_path =
        PathBuf::from("demo/video-01/scripts/invoke-pagila-country-object-diff-change.ps1");
    let script = fs::read_to_string(&script_path).expect("read Cycle 3 demo script");

    for expected in [
        "[ValidateSet(\"Status\", \"Apply\", \"Reset\")]",
        "$ApprovedHosts = @(\"localhost\", \"127.0.0.1\")",
        "$ApprovedDatabase = \"pagila\"",
        "$ApprovedUsername = \"exitpass\"",
        "$ApprovedContainer = \"exitpass-postgres\"",
        "$ApprovedDockerPort = \"5432\"",
        "$ExecutionMode = if ($env:DBSTATE_DEMO_PG_EXECUTION_MODE)",
        "$TargetSchema = \"public\"",
        "$TargetTable = \"country\"",
        "$TargetColumn = \"iso_code\"",
        "ALTER TABLE public.country ADD COLUMN iso_code varchar(2);",
        "ALTER TABLE public.country DROP COLUMN iso_code;",
        "$null = Invoke-DemoPsql -Sql \"ALTER TABLE public.country ADD COLUMN iso_code varchar(2);\"",
        "$null = Invoke-DemoPsql -Sql \"ALTER TABLE public.country DROP COLUMN iso_code;\"",
        "DBSTATE_DEMO_PG_CONTAINER",
        "DBSTATE_DEMO_PG_PASSWORD",
        "PGPASSWORD",
        "--env\", \"PGPASSWORD\"",
        "--set=ON_ERROR_STOP=1",
        "--field-separator=|",
        "ConvertTo-Json -Compress",
        "columnExists",
        "maximumLength",
        "nullable",
        "DockerUnavailable",
        "ContainerMissing",
        "ContainerStopped",
        "ContainerPsqlUnavailable",
        "UnsafeContainer",
        "UnexpectedPort",
        "UnexpectedUsername",
        "AlreadyApplied",
        "AlreadyReset",
    ] {
        assert!(
            script.contains(expected),
            "missing script contract text {expected}"
        );
    }

    for forbidden in [
        "POSTGRES_PASSWORD=postgres",
        "postgres://postgres:postgres@",
        "param(\n    [string]$Sql",
        "2>&1",
        "--env PGPASSWORD=",
        "git add",
        "git commit",
        "git push",
        "git pull",
        "git fetch",
        "git branch",
        "git switch",
        "dbstate.exe",
        "database/objects",
        "database/releases",
    ] {
        assert!(
            !script.contains(forbidden),
            "script contains forbidden text {forbidden}"
        );
    }
}

#[test]
fn cycle3_object_diff_demo_script_has_docker_process_and_parser_contract() {
    let script_path =
        PathBuf::from("demo/video-01/scripts/invoke-pagila-country-object-diff-change.ps1");
    let script = fs::read_to_string(&script_path).expect("read Cycle 3 demo script");

    for expected in [
        "function Invoke-ProcessCaptured",
        "Stdout = [System.IO.File]::ReadAllText($stdoutFile)",
        "Stderr = [System.IO.File]::ReadAllText($stderrFile)",
        "& $FilePath @Arguments > $stdoutFile 2> $stderrFile",
        "@(\"exec\", \"-i\", \"--env\", \"PGPASSWORD\", $Container, \"psql\") + $psqlArguments",
        "-FilePath \"docker\" -Arguments @(\"version\", \"--format\", \"{{.Server.Version}}\")",
        "-Arguments @(\"inspect\", \"--format\", \"{{.Name}}|{{.State.Running}}\", $Container)",
        "Invoke-ProcessCaptured -FilePath \"docker\" -Arguments @(\"exec\", $Container, \"psql\", \"--version\")",
        "function Test-TableExists",
        "to_regclass('public.country')",
        "function ConvertTo-ColumnState",
        "$parts = $line -split \"\\|\"",
        "if ($parts.Count -ne 4)",
        "if ($parts[0] -ne \"true\" -and $parts[0] -ne \"false\")",
        "if ($parts[1] -ne \"\" -or $parts[2] -ne \"\" -or $parts[3] -ne \"\")",
        "[int]::TryParse($parts[2], [ref]$parsedLength)",
        "$parts[3] -eq \"YES\"",
        "$parts[3] -eq \"NO\"",
        "Absent column state included unexpected metadata.",
        "Present column state was missing metadata.",
        "Column maximum length was not a valid integer.",
        "Column nullability was not a valid PostgreSQL value.",
    ] {
        assert!(
            script.contains(expected),
            "missing parser/process contract text {expected}"
        );
    }

    assert!(!script.contains("Invoke-Expression"));
    assert!(!script.contains("Out-String"));
    assert!(!script.contains("New-TemporaryFile"));
}

#[test]
fn cycle3_object_diff_demo_script_documents_expected_metadata_rows() {
    let docs = fs::read_to_string("docs/postgresql-v0.1/66-video-01-object-diff-demo-script.md")
        .expect("read Cycle 3 script docs");

    for expected in [
        "Docker is the canonical Video 1 PostgreSQL execution environment.",
        "container: `exitpass-postgres`",
        "container port: `5432`",
        "host port: `5433`",
        "database: `pagila`",
        "username: `exitpass`",
        "`DBSTATE_DEMO_PG_EXECUTION_MODE`: `docker`",
        "`DBSTATE_DEMO_PG_CONTAINER`: `exitpass-postgres`",
        "`DBSTATE_DEMO_PG_PASSWORD` has no default",
        "the password is forwarded by inherited environment variable name with `--env PGPASSWORD`.",
        "columnExists|dataType|maximumLength|nullable",
        "false|||",
        "true|character varying|2|YES",
        "Status, Apply, Status, repeated Apply, Reset, Status, repeated Reset",
        "The final validation state must have `public.country.iso_code` absent.",
        "DbState does not execute or apply this change.",
    ] {
        assert!(
            docs.contains(expected),
            "missing documentation text {expected}"
        );
    }

    for forbidden in [
        "POSTGRES_PASSWORD=postgres",
        "postgres://postgres:postgres@",
        "D:\\SourceCodes\\DbState",
        "D:\\DbState\\pagila",
    ] {
        assert!(
            !docs.contains(forbidden),
            "docs contain forbidden tester-facing text {forbidden}"
        );
    }
}

fn selector_readable_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug.chars().take(72).collect()
}

fn stable_selector_hash(value: &str) -> String {
    let mut hash = 2166136261u32;
    for unit in value.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")[..8].to_string()
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

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn js_bound_click_actions(js: &str) -> Vec<String> {
    let marker = "document.querySelector(\"[data-action='";
    let mut actions = Vec::new();
    let mut remaining = js;
    while let Some(start) = remaining.find(marker) {
        let after_marker = &remaining[start + marker.len()..];
        if let Some(end) = after_marker.find("']\")") {
            let after_selector = &after_marker[end + 4..];
            if after_selector.starts_with(".addEventListener") {
                actions.push(after_marker[..end].to_string());
            }
            remaining = after_selector;
        } else {
            break;
        }
    }
    actions
}

fn js_handler_for_action<'a>(js: &'a str, action: &str) -> &'a str {
    let marker = format!("document.querySelector(\"[data-action='{action}']\").addEventListener");
    let start = js.find(&marker).unwrap_or_else(|| {
        panic!("missing handler marker {marker}");
    });
    let remaining = &js[start..];
    let next_handler = remaining
        .get(marker.len()..)
        .and_then(|tail| {
            tail.find("\n  document.querySelector(")
                .map(|index| marker.len() + index)
        })
        .unwrap_or(remaining.len());
    &remaining[..next_handler]
}

#[test]
fn shell_click_handlers_have_runtime_dom_targets_and_prevent_default() {
    let html = ui_html();
    let js = ui_js();
    let actions = js_bound_click_actions(js);

    assert!(
        actions.contains(&"modal-close".to_string()),
        "modal close handler must be bound at runtime"
    );

    for action in &actions {
        assert!(
            html.contains(&format!("data-action=\"{action}\"")),
            "startup click handler has no matching DOM action {action}"
        );
    }

    for action in [
        "health",
        "check-status",
        "init-plan",
        "profiles-refresh",
        "profile-save",
        "profile-delete",
        "connection-test",
        "modal-close",
    ] {
        let handler = js_handler_for_action(js, action);
        assert!(
            handler.contains("event.preventDefault();"),
            "shell handler {action} must prevent default click behavior"
        );
    }

    for expected in [
        "id=\"app-modal\"",
        "id=\"app-modal-title\"",
        "id=\"app-modal-body\"",
        "data-action=\"modal-close\"",
    ] {
        assert!(
            html.contains(expected),
            "missing modal DOM target {expected}"
        );
    }
}

#[test]
fn shell_workspace_controls_have_independent_endpoint_handlers() {
    let html = ui_html();
    let js = ui_js();

    for (test_id, action, label) in [
        ("workspace-health", "health", "Health"),
        ("workspace-check-status", "check-status", "Check Status"),
        ("workspace-init-plan", "init-plan", "Init Plan"),
    ] {
        assert!(
            html.contains(&format!(
                "data-action=\"{action}\" data-testid=\"{test_id}\""
            )),
            "missing shell control action/test id {action}/{test_id}"
        );
        assert!(html.contains(label), "missing shell control label {label}");
        assert_eq!(
            count_occurrences(html, &format!("data-action=\"{action}\"")),
            1,
            "duplicate shell action {action}"
        );
        assert_eq!(
            count_occurrences(html, &format!("data-testid=\"{test_id}\"")),
            1,
            "duplicate shell test id {test_id}"
        );
    }

    for expected in [
        "\"health\": approvedEndpoints.health",
        "\"check-status\": approvedEndpoints.workspaceValidate",
        "\"workspace-status\": approvedEndpoints.workspaceValidate",
        "\"repo-status\": approvedEndpoints.repoStatus",
        "\"init-plan\": approvedEndpoints.initPlan",
        "\"connection-test\": approvedEndpoints.connectionTest",
    ] {
        assert!(
            js.contains(expected),
            "missing shell action endpoint dispatch {expected}"
        );
    }

    let health = js_handler_for_action(js, "health");
    assert!(health.contains("run(\"Health\", shellActionEndpointByAction[\"health\"]"));
    assert!(!health.contains("approvedEndpoints.repoStatus"));
    assert!(!health.contains("approvedEndpoints.workspaceValidate"));
    assert!(!health.contains("approvedEndpoints.initPlan"));

    let check_status = js_handler_for_action(js, "check-status");
    assert!(check_status.contains("runCheckStatus();"));
    assert!(js.contains("requestJson(shellActionEndpointByAction[\"workspace-status\"]"));
    assert!(js.contains("requestJson(shellActionEndpointByAction[\"repo-status\"]"));
    assert!(js.contains("updateStatus(\"Check Status\", data)"));
    assert!(!html.contains("Check Workspace"));
    assert!(!html.contains("Repo Status"));

    let init_plan = js_handler_for_action(js, "init-plan");
    assert!(init_plan.contains("guardGitWorkspaceBefore(\"init-plan\")"));
    assert!(init_plan.contains("run(\"Init plan\", shellActionEndpointByAction[\"init-plan\"]"));
    assert!(init_plan.contains("attachWorkspacePath({ dryRun: true })"));
    assert!(!init_plan.contains("approvedEndpoints.repoStatus"));
}

#[test]
fn shell_connection_controls_have_independent_profile_and_session_wiring() {
    let html = ui_html();
    let js = ui_js();

    for expected in [
        "id=\"connection-mode\" data-testid=\"connection-mode\"",
        "<option value=\"sessionUrl\">Use session URL</option>",
        "<option value=\"profile\">Use saved profile</option>",
        "id=\"session-url-panel\" data-testid=\"target-connection-input\"",
        "id=\"postgres-url\" data-testid=\"source-connection-input\" type=\"password\"",
        "id=\"profile-panel\" hidden",
        "id=\"profile-select\"",
        "id=\"profile-password\" type=\"password\"",
        "data-action=\"profiles-refresh\"",
        "data-action=\"profile-save\"",
        "data-action=\"profile-delete\"",
        "data-action=\"connection-test\" data-testid=\"source-target-run\"",
        "Passwords, tokens, and full URLs are never saved.",
    ] {
        assert!(
            html.contains(expected),
            "missing connection shell UI {expected}"
        );
    }

    for unique in [
        "<select id=\"connection-mode\"",
        "<input id=\"postgres-url\"",
        "<select id=\"profile-select\"",
        "<input id=\"profile-password\"",
        "data-action=\"profiles-refresh\"",
        "data-action=\"profile-save\"",
        "data-action=\"profile-delete\"",
        "data-action=\"connection-test\"",
    ] {
        assert_eq!(
            count_occurrences(html, unique),
            1,
            "duplicate connection shell selector {unique}"
        );
    }

    assert!(js.contains("document.getElementById(\"connection-mode\").addEventListener(\"change\""));
    assert!(js.contains("selectedConnectionMode() === \"profile\""));
    assert!(js.contains("run(\"Connection profiles\", approvedEndpoints.profiles"));
    assert!(js.contains("document.getElementById(\"profile-select\").addEventListener(\"change\", updateSelectedProfileDetails)"));

    let refresh = js_handler_for_action(js, "profiles-refresh");
    assert!(refresh.contains("run(\"Connection profiles\", approvedEndpoints.profiles"));

    let save = js_handler_for_action(js, "profile-save");
    assert!(save.contains("profileRequestBody()"));
    assert!(save.contains("approvedEndpoints.profiles"));
    assert!(save.contains("method: method"));

    let delete = js_handler_for_action(js, "profile-delete");
    assert!(delete.contains("profilePath(profileName)"));
    assert!(delete.contains("method: \"DELETE\""));

    let connection_test = js_handler_for_action(js, "connection-test");
    assert!(connection_test
        .contains("run(\"Connection test\", shellActionEndpointByAction[\"connection-test\"]"));
    assert!(connection_test.contains("attachConnection({})"));
    assert!(!connection_test.contains("approvedEndpoints.profiles"));

    assert!(js.contains("if (mode === \"sessionUrl\")"));
    assert!(js.contains("body.postgresUrl = url"));
    assert!(js.contains("body.connection = { profileName: profileName }"));
    assert!(js.contains("body.connection.password = password"));
    assert!(!js.contains("localStorage"));
    assert!(!js.contains("sessionStorage"));
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

    assert!(html
        .contains("<label id=\"object-type-filter-label\" for=\"object-type-filter\">Object type"));
    assert!(html.contains("<option value=\"all\">All</option>"));
    assert!(html.contains("<option value=\"schema\">Schema</option>"));
    assert!(html.contains("<option value=\"table\">Table</option>"));
    assert!(!html.contains("<option value=\"column\">Column</option>"));
    assert!(!html.contains("<option value=\"referenceData\">Reference data</option>"));
    assert!(html.contains("<select id=\"compare-schema\">"));
    assert!(html.contains("<select id=\"compare-table\">"));
    assert!(html.contains("<select id=\"data-table\">"));
    assert!(html.contains("Run Inspect first to populate schema and table lists."));
    assert!(html.contains("Reference-data compare is read-only."));
    assert!(html.contains("database/reference-data/dbstate.reference-data.yml"));
    assert!(html.contains("database/reference-data/tables/"));
    assert!(html.contains("keyColumns:"));
    assert!(html.contains("ignoredColumns:"));
    assert!(html.contains("reference-data-configured-tables"));
    assert!(html.contains("reference-data-selected-count"));
    assert!(js.contains("Reference-Data Row Data Diff"));
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
        "database/objects/domains/",
        "database/objects/aggregates/",
        "database/objects/sequences/",
        "database/objects/indexes/",
        "database/objects/views/",
        "database/objects/materialized-views/",
        "database/objects/constraints/",
        "database/objects/functions/",
        "database/objects/triggers/",
        "database/objects/grants/",
        "database/objects/rls-policies/",
        "objectType: \"schema\"",
        "objectType: \"table\"",
        "objectType: \"extension\"",
        "objectType: \"enum\"",
        "objectType: \"domain\"",
        "objectType: \"aggregate\"",
        "objectType: \"sequence\"",
        "objectType: \"index\"",
        "objectType: \"view\"",
        "objectType: \"materializedView\"",
        "objectType: \"constraint\"",
        "objectType: \"function\"",
        "objectType: \"trigger\"",
        "objectType: \"grant\"",
        "objectType: \"rlsPolicy\"",
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
        "data.domains",
        "data.aggregates",
        "data.sequences",
        "data.indexes",
        "data.views",
        "data.materializedViews",
        "data.constraints",
        "data.functions",
        "data.triggers",
        "data.grants",
        "data.rlsPolicies",
        "functionIdentitySlug",
        "updateCompareOptionLists",
        "updateTableOptions",
        "updateReferenceDataOptions",
        "updateReferenceDataConfiguredTables",
        "renderReferenceDataConfiguredTables",
        "referenceDataSelectedTables",
        "selectedTables",
        "referenceDataStatus",
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
    assert!(!html.contains("functions future"));
    assert!(!html.contains("triggers future"));
    assert!(!html.contains("materialized views future"));
    assert!(!html.contains("grants future"));
    assert!(!html.contains("RLS policies future"));
    assert!(js.contains("label === \"Inspect\""));
    assert!(js.contains("label === \"Reference-data compare\""));
    assert!(js.contains("label === \"Reference-data status\""));
    assert!(js.contains("appendOption(select, \"referenceData\", \"Reference data\")"));
    assert!(html.contains("Run Reference Data Compare"));
    assert!(!js.contains("Reference-Data Compare Is Out of Scope"));
    assert!(!js.contains("service:response"));
    assert!(!js.contains("objectType: \"service\""));
}

#[test]
fn slice34_reference_data_compare_ui_is_enabled_and_read_only() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "Reference-Data Compare",
        "reference-data-compare-panel",
        "reference-data-status",
        "configured reference-data tables",
        "Key columns",
        "Ignored columns",
        "Masked columns",
        "reference-data-table-checkbox",
        "reference-data-selected-count",
        "Select all",
        "Clear selection",
        "Reference-Data Row Data Diff",
        "database/reference-data/dbstate.reference-data.yml",
        "database/reference-data/tables/public.country.yml",
        "Canonical table file format",
        "Simplified supported format",
        "derives row keys from registry",
        "table: public.country",
        "key:",
        "values:",
        "country_id: 1",
        "country: Afghanistan",
        "keyColumns:",
        "ignoredColumns:",
        "maskedColumns: []",
        "DbState does not insert, update, delete, merge, or apply data changes",
        "Only tables listed in",
        "Masked columns remain masked",
        "[masked]",
        "ignored for comparison",
    ] {
        assert!(
            combined.contains(expected),
            "missing Slice 34 UI contract text {expected}"
        );
    }

    assert!(combined.contains("/api/v1/reference-data/status"));
    assert!(combined.contains("selectedTables"));
    assert!(combined.contains("renderReferenceDataConfiguredTables"));
    assert!(combined.contains("renderReferenceDataTableDataDiff"));
    assert!(combined.contains("showReferenceDataRowDataDiffModal"));
    assert!(combined.contains("state.referenceDataSelectedTables"));
    assert!(!combined.contains("Reference-Data Compare Is Out of Scope"));
    assert!(!combined.contains("reference-data compare is out-of-scope"));
    assert!(
        !combined.contains("data-action=\"data-compare\" data-standard-operation-action disabled")
    );
    assert!(!combined.contains("Sync to Database"));
    assert!(!combined.contains("Apply data changes"));
}

#[test]
fn slice34b_reference_data_repo_to_database_is_table_first_data_diff() {
    let html = ui_html();
    let css = ui_css();
    let js = ui_js();
    let combined = format!("{html}\n{css}\n{js}");

    for expected in [
        "id=\"tab-object-diff\"",
        "id=\"object-type-filter-label\"",
        "id=\"results-head\"",
        "id=\"data-diff-view\" data-testid=\"reference-data-data-diff\"",
        "data-testid=\"reference-data-data-diff-rows\"",
        "<tr><th>Key</th><th>Status</th><th>Changed Columns</th><th>Warnings</th></tr>",
    ] {
        assert!(
            html.contains(expected),
            "missing Slice 34B HTML contract {expected}"
        );
    }

    for expected in [
        "objectDiffTab.textContent = referenceDataMode ? \"5. Data Diff\" : \"5. Object Diff\"",
        "objectTypeLabel.hidden = referenceDataMode",
        "renderReferenceDataResultsHeader",
        "<tr><th>Table</th><th>Total Rows</th><th>Similar</th><th>Different</th><th>Repository Only</th><th>Database Only</th><th>Warnings</th></tr>",
        "data-action\", \"open-reference-data-table-diff\"",
        "data-testid\", \"reference-data-table-summary-row\"",
        "status-insync",
        "status-repodifferent",
        "status-repoonly",
        "status-databaseonly",
        "renderReferenceDataTableDataDiff",
        "data-action\", \"open-reference-data-row-diff-modal\"",
        "data-testid\", \"reference-data-row-data-diff-row\"",
        "showReferenceDataRowDataDiffModal",
        "Reference-Data Row Data Diff",
        "data-testid\", \"reference-data-row-data-diff-modal\"",
        "Source: \" + direction.sourceLabel + \". Target: \" + direction.targetLabel",
        "Column</th><th>Source value</th><th>Target value</th><th>Status",
        "Same",
        "Source only",
        "Target only",
        "Different",
        "Ignored",
        "[masked]",
        "repositoryValues",
        "databaseValues",
        "keyValues",
        "changedColumns",
        "ignoredColumns",
        "maskedColumns",
    ] {
        assert!(js.contains(expected), "missing Slice 34B JS contract {expected}");
    }

    for expected in [
        ".summary-count",
        ".source-only-value",
        ".target-only-value",
        ".different-value",
        ".ignored-value",
        ".masked-value",
        "#object-type-filter-label[hidden]",
        "#data-diff-view[hidden]",
        ".object-diff-tabs[hidden]",
        "#ddl-comparison-status[hidden]",
        "#ddl-diff-view[hidden]",
        "#related-objects-view[hidden]",
    ] {
        assert!(
            css.contains(expected),
            "missing Slice 34B CSS contract {expected}"
        );
    }

    let data_diff_section = html
        .split("id=\"data-diff-view\"")
        .nth(1)
        .expect("data diff section exists")
        .split("<div class=\"object-diff-tabs\"")
        .next()
        .expect("data diff section before object diff tabs");
    for forbidden in [
        "Full Context DDL",
        "Object Only DDL",
        "Related Objects",
        "Raw Details",
        "DDL unavailable",
        "Source DDL",
        "Target DDL",
    ] {
        assert!(
            !data_diff_section.contains(forbidden),
            "reference-data Data Diff section exposes DDL chrome {forbidden}"
        );
    }

    for forbidden in [
        "Edit Reference Data Row",
        "Delete Reference Data Row",
        "Apply Reference Data",
        "Sync to Database",
        "Execute Reference Data",
    ] {
        assert!(
            !combined.contains(forbidden),
            "Slice 34B must not expose forbidden reference-data action {forbidden}"
        );
    }
}

#[test]
fn slice34b_schema_object_diff_chrome_is_preserved() {
    let html = ui_html();
    let js = ui_js();

    for expected in [
        "5. Object Diff",
        "Full Context DDL",
        "Object Only DDL",
        "Related Objects",
        "Raw Details",
        "Source DDL - Full Context DDL",
        "Target DDL - Full Context DDL",
        "approvedEndpoints.objectDdl",
        "loadSelectedObjectDdl",
        "renderLoadedObjectDiff",
    ] {
        assert!(
            format!("{html}\n{js}").contains(expected),
            "schema Object Diff contract missing {expected}"
        );
    }

    assert!(js.contains(
        "objectDiffTab.textContent = referenceDataMode ? \"5. Data Diff\" : \"5. Object Diff\""
    ));
    assert!(js.contains("dataDiffView.hidden = !referenceDataMode"));
    assert!(js.contains("element.hidden = referenceDataMode"));
}

#[test]
fn slice34e_reference_data_database_to_repository_uses_data_diff_and_registry_status() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "Reference Data Compare: Database to Repository",
        "Load database tables",
        "Run Reference Data Compare",
        "Preview Reference YAML",
        "Write Reference Data Files",
        "Registry status",
        "keyColumns",
        "ignoredColumns",
        "maskedColumns",
        "In registry",
        "Not configured",
        "registryStatus",
        "configuredTables",
        "registryConfig",
        "versionedColumns",
    ] {
        assert!(
            combined.contains(expected),
            "missing Database-to-Repository reference-data UI contract {expected}"
        );
    }

    for expected in [
        "const referenceDataWorkflow = isReferenceDataWorkflowMode(mode)",
        "button.hidden = !referenceDataWorkflow",
        "button.disabled = !referenceDataWorkflow",
        "if (isReferenceDataWorkflowMode(currentWorkflowMode()))",
        "renderReferenceDataTableSummaryResults();",
        "return isReferenceDataDatabaseToRepositoryMode(currentWorkflowMode())",
        "? \"referenceDataDatabaseToRepository\"",
        "sourceLabel: \"PostgreSQL database\"",
        "targetLabel: \"Repository reference-data\"",
        "sourceValues = direction.sourceDdlSide === \"database\" ? rowRaw.databaseValues",
        "targetValues = direction.targetDdlSide === \"repository\" ? rowRaw.repositoryValues",
        "databaseToRepository && rowRaw.classification === \"databaseOnly\"",
        "databaseToRepository && rowRaw.classification === \"repoOnly\"",
        "objectDiffTab.textContent = referenceDataMode ? \"5. Data Diff\" : \"5. Object Diff\"",
        "document.querySelectorAll(\".object-diff-tabs\").forEach",
        "rawDetails.parentElement.hidden = referenceDataMode",
    ] {
        assert!(
            js.contains(expected),
            "missing Database-to-Repository Data Diff JS contract {expected}"
        );
    }

    for expected in [
        "Column",
        "Source value",
        "Target value",
        "Status",
        "Key",
        "Changed Columns",
        "Warnings",
    ] {
        assert!(
            combined.contains(expected),
            "reference-data Data Diff missing {expected}"
        );
    }
}

#[test]
fn slice34c_reference_data_review_script_ui_contract_is_review_only() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "data-testid=\"reference-data-review-script-panel\"",
        "data-testid=\"reference-data-review-script-status\"",
        "Preview Review-Only Data Script",
        "Generate Review-Only Data Script",
        "GENERATE REVIEW DATA SCRIPT",
        "DbState does not execute this SQL",
        "DbState does not generate DELETE for database-only rows in Private Beta",
        "Review-only data script preview completed. No files were written.",
        "Review-only data script preview failed. No files were written.",
        "Review-only data script artifacts generated successfully.",
        "Review-only data script generation failed. No files were written.",
        "Zero artifacts were created; check the errors and warnings below.",
        "Database-only rows require manual review",
        "potentially affected foreign keys",
        "Script generation includes non-in-sync rows for selected tables",
        "referenceDataReviewScriptPreview: \"/api/v1/reference-data/review-script/preview\"",
        "referenceDataReviewScriptWrite: \"/api/v1/reference-data/review-script/write\"",
        "confirmReferenceDataReviewScript",
        "renderReferenceDataReviewScriptResult",
        "selectedReferenceDataReviewTables",
        "reference-data-review-artifact-preview",
        "reference-data-review-script-preview-meta",
        "previewReferenceDataReviewArtifact",
        "artifactPath: artifactPath",
        "artifactType",
        "truncated",
    ] {
        assert!(
            combined.contains(expected),
            "missing Slice 34C UI contract {expected}"
        );
    }

    for forbidden in [
        "data-action=\"reference-data-apply\"",
        "data-action=\"reference-data-execute\"",
        "data-action=\"reference-data-sync-to-database\"",
        "data-action=\"reference-data-edit-row\"",
        "data-action=\"reference-data-delete-row\"",
        "Apply Reference Data",
        "Execute Reference Data",
        "Sync to Database",
        "Edit Reference Data Row",
        "Delete Reference Data Row",
    ] {
        assert!(
            !combined.contains(forbidden),
            "Slice 34C UI exposes forbidden control {forbidden}"
        );
    }
}

#[test]
fn slice34d_release_plan_is_hidden_for_reference_data_workflows() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    assert!(html.contains("data-testid=\"tab-release-plan\""));
    assert!(html.contains("id=\"step-release-plan\""));
    assert!(js.contains("function releasePlanAppliesToWorkflow(mode)"));
    assert!(js.contains("return isSchemaRepositoryToDatabaseMode(mode);"));
    assert!(js.contains("releasePlanTab.hidden = !releasePlanApplies"));
    assert!(js.contains("releasePlanPanel.hidden = !releasePlanApplies"));
    assert!(js.contains("step === \"release-plan\" && !releasePlanAppliesToWorkflow"));
    assert!(js.contains("Reference-data compare is read-only and does not generate DML"));
    assert!(js.contains("This workflow writes selected YAML files under database/reference-data/"));
    assert!(combined.contains("Review-Only Data Script"));
}

#[test]
fn slice34a_reference_data_export_ui_contract_is_present() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "Schema Compare: Repository to Database",
        "Schema Compare: Database to Repository",
        "Reference Data Compare: Repository to Database",
        "Reference Data Compare: Database to Repository",
        "schemaRepoToDatabase",
        "schemaDatabaseToRepository",
        "referenceDataRepoToDatabase",
        "referenceDataDatabaseToRepository",
    ] {
        assert!(
            combined.contains(expected),
            "missing direction-explicit workflow mode {expected}"
        );
    }

    for expected in [
        "Reference Data: Database to Repository",
        "Load database tables",
        "reference-data-database-tables",
        "Selected key count",
        "Versioned columns",
        "Masked columns",
        "Rows",
        "Selected Table Detail",
        "key column",
        "versioned columns",
        "Ignored columns are derived",
        "masked columns",
        "Preview Reference YAML",
        "Write Reference Data Files",
        "WRITE REFERENCE DATA FILES",
        "reference-data-export-table-checkbox",
        "reference-data-export-columns",
        "reference-data-export-preview",
        "/api/v1/reference-data/database-tables",
        "/api/v1/reference-data/export/preview",
        "/api/v1/reference-data/export/write",
        "confirmReferenceDataWrite",
        "sourceKind: \"PostgreSQL database\"",
        "targetKind: \"Repository reference-data\"",
    ] {
        assert!(
            combined.contains(expected),
            "missing Slice 34A UI contract text {expected}"
        );
    }

    assert!(combined.contains("Refresh Database Inventory"));
    assert!(!combined.contains("PostgreSQL Inspect Only"));
    assert!(!combined.contains("<option value=\"inspect\">"));
    assert!(
        html.find("id=\"reference-data-panel\"").unwrap()
            < html.find("id=\"reference-data-export-panel\"").unwrap(),
        "reference-data export workflow should be separated after the compare panel"
    );
    assert!(
        html.find("id=\"reference-data-export-panel\"").unwrap()
            < html.find("id=\"repository-sync-controls\"").unwrap(),
        "reference-data export workflow should not be nested in schema repository sync controls"
    );
    assert!(!combined.contains("Sync to Database"));
    assert!(!combined.contains("Apply data changes"));
    assert!(!combined.contains("Execute Reference Data"));
    assert!(!combined.contains("Edit Reference Data Row"));
    assert!(!combined.contains("Delete Reference Data Row"));
}

#[test]
fn reference_data_export_write_requires_confirmation() {
    let dir = create_temp_dir("reference-data-export-confirmation");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let base_body = format!(
        r#"{{
  "repositoryPath": "{}",
  "postgresUrl": "postgres://user:secret@example.invalid/db",
  "tables": [
    {{
      "schema": "public",
      "name": "country",
      "keyColumns": ["country_id"],
      "versionedColumns": ["country"],
      "maskedColumns": []
    }}
  ]
}}"#,
        escape_json(&display_path(&dir))
    );

    let missing = service_response(
        "POST",
        "/api/v1/reference-data/export/write",
        &base_body,
        &dir,
    );
    assert_eq!(missing.status_code, 400);
    assert!(missing.body.contains("WRITE REFERENCE DATA FILES"));
    assert!(!missing.body.contains("secret"));
    assert!(!missing.body.contains("postgres://"));
    assert!(!dir
        .join("database/reference-data/tables/public.country.yml")
        .exists());
}

#[test]
fn reference_data_database_tables_endpoint_redacts_connection_errors() {
    let dir = create_temp_dir("reference-data-database-tables-redaction");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let body = format!(
        r#"{{
  "repositoryPath": "{}",
  "postgresUrl": "postgres://user:secret@/db"
}}"#,
        escape_json(&display_path(&dir))
    );

    let response = service_response(
        "POST",
        "/api/v1/reference-data/database-tables",
        &body,
        &dir,
    );

    assert_eq!(response.status_code, 400);
    assert!(!response.body.contains("secret"));
    assert!(!response.body.contains("postgres://"));
}

#[test]
fn slice15_service_routes_and_write_confirmation_are_present() {
    let routes = service_route_definitions();
    assert!(routes.contains(&("POST", "/api/v1/postgres/object-ddl")));
    assert!(routes.contains(&("POST", "/api/v1/repository/object-files/list")));
    assert!(routes.contains(&("POST", "/api/v1/repository/object-files/preview")));
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
fn object_ddl_supports_aggregate_sql_files() {
    let dir = create_temp_dir("object-ddl-aggregate");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/objects/aggregates/dbstate_slice2.account_labels.text.sql"),
        render_aggregate_sql(&sample_inventory().aggregates[0]),
    )
    .expect("write aggregate ddl");
    commit_all(&dir, "add aggregate object");

    let body = r#"{ "scope": "all", "objectType": "aggregate", "schema": "dbstate_slice2", "objectName": "account_labels.text", "relativePath": "database/objects/aggregates/dbstate_slice2.account_labels.text.sql" }"#;
    let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("CREATE AGGREGATE"));
    assert!(response.body.contains("Database DDL is unavailable"));
    assert!(!response.body.contains("DROP AGGREGATE"));
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
    let constraint_path = dir
        .join("database")
        .join("objects")
        .join("constraints")
        .join("primary-keys")
        .join("core.accounts.accounts_pkey.sql");
    fs::write(
        &constraint_path,
        "ALTER TABLE \"core\".\"accounts\"\n    ADD CONSTRAINT \"accounts_pkey\" PRIMARY KEY (\"account_id\");\n",
    )
    .expect("write constraint ddl");
    let trigger_path = dir
        .join("database")
        .join("objects")
        .join("triggers")
        .join("core.accounts.accounts_audit_trigger.sql");
    fs::write(
        &trigger_path,
        "-- DbState PostgreSQL desired-state object\n-- Object type: trigger\n-- Object name: core.accounts.accounts_audit_trigger\n-- Trigger function: core.audit_accounts\n\nCREATE TRIGGER accounts_audit_trigger AFTER INSERT ON core.accounts FOR EACH ROW EXECUTE FUNCTION core.audit_accounts();\n",
    )
    .expect("write trigger ddl");
    commit_all(
        &dir,
        "complete structure with table index constraint and trigger",
    );

    let body = r#"{ "scope": "all", "objectType": "table", "schema": "core", "objectName": "accounts", "relativePath": "database/objects/tables/core.accounts.sql" }"#;
    let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("\"objectOnly\""));
    assert!(response.body.contains("\"fullContext\""));
    assert!(response.body.contains("\"relatedObjects\""));
    assert!(response.body.contains("CREATE TABLE"));
    assert!(response.body.contains("accounts_code_idx"));
    assert!(response.body.contains("accounts_pkey"));
    assert!(response.body.contains("accounts_audit_trigger"));
    assert!(response
        .body
        .contains("database/objects/indexes/core.accounts.accounts_code_idx.sql"));
    assert!(response
        .body
        .contains("database/objects/constraints/primary-keys/core.accounts.accounts_pkey.sql"));
    assert!(response.body.contains("\"group\":\"Indexes\""));
    assert!(response.body.contains("\"group\":\"Constraints\""));
    assert!(response.body.contains("\"group\":\"Triggers\""));
    assert!(response.body.contains("\"group\":\"Comments\""));
    assert!(response
        .body
        .contains("Object Only DDL is the normalized durable object"));
    assert!(!response.body.contains("postgres://"));

    let constraint_body = r#"{ "scope": "all", "objectType": "constraint", "schema": "core", "objectName": "accounts.accounts_pkey", "relativePath": "database/objects/constraints/primary-keys/core.accounts.accounts_pkey.sql" }"#;
    let constraint_response =
        service_response("POST", "/api/v1/postgres/object-ddl", constraint_body, &dir);

    assert_eq!(
        constraint_response.status_code, 200,
        "{}",
        constraint_response.body
    );
    assert!(constraint_response
        .body
        .contains("\"objectType\":\"constraint\""));
    assert!(constraint_response.body.contains("ADD CONSTRAINT"));
    assert!(constraint_response.body.contains("accounts_pkey"));

    let function_path = dir
        .join("database")
        .join("objects")
        .join("functions")
        .join("core.account_label.account_id_integer.sql");
    fs::write(
        &function_path,
        "-- DbState PostgreSQL desired-state object\n-- Object type: function\n-- Object name: core.account_label(account_id integer)\n-- Language: sql\n\nCREATE FUNCTION core.account_label(account_id integer)\n RETURNS text\n LANGUAGE sql\nAS $function$\n    SELECT 'account-' || account_id::text;\n$function$;\n",
    )
    .expect("write function ddl");
    commit_all(&dir, "add function object");

    let function_body = r#"{ "scope": "all", "objectType": "function", "schema": "core", "objectName": "account_label.account_id_integer", "relativePath": "database/objects/functions/core.account_label.account_id_integer.sql" }"#;
    let function_response =
        service_response("POST", "/api/v1/postgres/object-ddl", function_body, &dir);

    assert_eq!(
        function_response.status_code, 200,
        "{}",
        function_response.body
    );
    assert!(function_response
        .body
        .contains("\"objectType\":\"function\""));
    assert!(function_response
        .body
        .contains("CREATE FUNCTION core.account_label"));
    assert!(function_response.body.contains("\"group\":\"Schema\""));
    assert!(function_response.body.contains("\"group\":\"Language\""));
    assert!(function_response
        .body
        .contains("Function comment rendering is deferred"));
    assert!(!function_response.body.contains("DROP FUNCTION"));

    let trigger_body = r#"{ "scope": "all", "objectType": "trigger", "schema": "core", "objectName": "accounts.accounts_audit_trigger", "relativePath": "database/objects/triggers/core.accounts.accounts_audit_trigger.sql" }"#;
    let trigger_response =
        service_response("POST", "/api/v1/postgres/object-ddl", trigger_body, &dir);

    assert_eq!(
        trigger_response.status_code, 200,
        "{}",
        trigger_response.body
    );
    assert!(trigger_response.body.contains("\"objectType\":\"trigger\""));
    assert!(trigger_response
        .body
        .contains("CREATE TRIGGER accounts_audit_trigger"));
    assert!(trigger_response.body.contains("\"group\":\"Schema\""));
    assert!(trigger_response
        .body
        .contains("\"group\":\"Parent Relation\""));
    assert!(trigger_response
        .body
        .contains("\"group\":\"Trigger Function\""));
    assert!(trigger_response
        .body
        .contains("Trigger comment rendering is deferred"));
    assert!(!trigger_response.body.contains("DROP TRIGGER"));
    assert!(!trigger_response.body.contains("ALTER TRIGGER"));

    let materialized_view_path = dir
        .join("database")
        .join("objects")
        .join("materialized-views")
        .join("core.account_summary.sql");
    fs::write(
        &materialized_view_path,
        "-- DbState PostgreSQL desired-state object\n-- Object type: materializedView\n-- Object name: core.account_summary\n\nCREATE MATERIALIZED VIEW \"core\".\"account_summary\" AS\n SELECT account_id FROM core.accounts\nWITH NO DATA;\n",
    )
    .expect("write materialized view ddl");
    let materialized_view_index_path = dir
        .join("database")
        .join("objects")
        .join("indexes")
        .join("core.account_summary.account_summary_account_id_idx.sql");
    fs::write(
        &materialized_view_index_path,
        "CREATE INDEX \"account_summary_account_id_idx\" ON \"core\".\"account_summary\" (\"account_id\");\n",
    )
    .expect("write materialized view index ddl");
    commit_all(&dir, "add materialized view object");

    let materialized_view_body = r#"{ "scope": "all", "objectType": "materializedView", "schema": "core", "objectName": "account_summary", "relativePath": "database/objects/materialized-views/core.account_summary.sql" }"#;
    let materialized_view_response = service_response(
        "POST",
        "/api/v1/postgres/object-ddl",
        materialized_view_body,
        &dir,
    );

    assert_eq!(
        materialized_view_response.status_code, 200,
        "{}",
        materialized_view_response.body
    );
    assert!(materialized_view_response
        .body
        .contains("\"objectType\":\"materializedView\""));
    assert!(materialized_view_response
        .body
        .contains("CREATE MATERIALIZED VIEW"));
    assert!(materialized_view_response.body.contains("WITH NO DATA"));
    assert!(materialized_view_response
        .body
        .contains("account_summary_account_id_idx"));
    assert!(materialized_view_response
        .body
        .contains("\"group\":\"Indexes\""));
    assert!(materialized_view_response
        .body
        .contains("Materialized view comment rendering is deferred"));
    assert!(!materialized_view_response
        .body
        .contains("DROP MATERIALIZED VIEW"));
    assert!(!materialized_view_response
        .body
        .contains("REFRESH MATERIALIZED VIEW"));

    let grant_path = dir
        .join("database")
        .join("objects")
        .join("grants")
        .join("tables")
        .join("core.accounts.app_reader.sql");
    fs::write(
        &grant_path,
        "-- DbState PostgreSQL desired-state object\n-- Object type: grant\n-- Grant target kind: table\n-- Object name: core.accounts\n-- Grantee: app_reader\n\nGRANT SELECT ON TABLE \"core\".\"accounts\" TO \"app_reader\";\n",
    )
    .expect("write grant ddl");
    commit_all(&dir, "add grant object");

    let grant_body = r#"{ "scope": "all", "objectType": "grant", "schema": "core", "objectName": "table.core.accounts.app_reader", "relativePath": "database/objects/grants/tables/core.accounts.app_reader.sql" }"#;
    let grant_response = service_response("POST", "/api/v1/postgres/object-ddl", grant_body, &dir);

    assert_eq!(grant_response.status_code, 200, "{}", grant_response.body);
    assert!(grant_response.body.contains("\"objectType\":\"grant\""));
    assert!(grant_response.body.contains("GRANT SELECT ON TABLE"));
    assert!(grant_response.body.contains("\"group\":\"Schema\""));
    assert!(grant_response.body.contains("\"group\":\"Target Object\""));
    assert!(grant_response.body.contains("\"group\":\"Grantee Role\""));
    assert!(!grant_response.body.contains("REVOKE"));
}

#[test]
fn object_ddl_supports_partitioned_table_sql_files() {
    let dir = create_temp_dir("object-ddl-partitioned-table");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let inventory = partitioned_payment_inventory();
    fs::write(
        dir.join("database/objects/tables/public.payment.sql"),
        render_table_sql_with_partition(
            "public",
            "payment",
            &inventory.columns,
            inventory.tables[0].partition_key.as_deref(),
        ),
    )
    .expect("write partitioned table");
    commit_all(&dir, "partitioned table");

    let body = r#"{ "scope": "all", "objectType": "table", "schema": "public", "objectName": "payment", "relativePath": "database/objects/tables/public.payment.sql" }"#;
    let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("\"objectOnly\""));
    assert!(response.body.contains("CREATE TABLE"));
    assert!(response.body.contains("PARTITION BY RANGE (payment_date);"));
}

#[test]
fn object_ddl_returns_rls_policy_ddl_and_related_objects() {
    let dir = create_temp_dir("object-ddl-rls-policy");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    let policy_path = dir
        .join("database")
        .join("objects")
        .join("rls-policies")
        .join("core.accounts.accounts_tenant_policy.sql");
    fs::write(
        &policy_path,
        "-- DbState PostgreSQL desired-state object\n-- Object type: rlsPolicy\n-- Object name: core.accounts.accounts_tenant_policy\n-- Table RLS enabled observed: true\n-- Table RLS forced observed: false\n-- Table RLS state is informational only; DbState does not enable, disable, or force RLS.\n\nCREATE POLICY \"accounts_tenant_policy\"\nON \"core\".\"accounts\"\nAS PERMISSIVE\nFOR SELECT\nTO PUBLIC, \"app_reader\"\nUSING (tenant_id = current_setting('app.tenant_id')::uuid)\n;\n",
    )
    .expect("write RLS policy ddl");
    commit_all(&dir, "add RLS policy object");

    let body = r#"{ "scope": "all", "objectType": "rlsPolicy", "schema": "core", "objectName": "accounts.accounts_tenant_policy", "relativePath": "database/objects/rls-policies/core.accounts.accounts_tenant_policy.sql" }"#;
    let response = service_response("POST", "/api/v1/postgres/object-ddl", body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert!(response.body.contains("\"objectType\":\"rlsPolicy\""));
    assert!(response.body.contains("CREATE POLICY"));
    assert!(response.body.contains("\"group\":\"Schema\""));
    assert!(response.body.contains("\"group\":\"Target Table\""));
    assert!(response.body.contains("\"group\":\"Role\""));
    assert!(!response.body.contains("DROP POLICY"));
    assert!(!response.body.contains("ALTER POLICY"));
    assert!(!response.body.contains("ENABLE ROW LEVEL SECURITY"));
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
fn slice15_repository_sync_write_allows_unrelated_dirty_tree() {
    let dir = create_temp_dir("slice15-dirty-write");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(dir.join("dirty.txt"), "dirty").expect("dirty tree");

    let report =
        sync_postgres_with_inventory(&dir, &sample_inventory(), &ExportSelection::All, false);

    assert!(report.success, "{:?}", report.errors);
    assert!(!report.created_files.is_empty());
    assert!(report.updated_files.is_empty());
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("unrelated changes")));
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
            "Compare is not available in Schema Compare: Database to Repository mode. Use Preview Repository Sync.",
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
            "Repository reference-data",
            "PostgreSQL target",
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
    let direction_function_index = js.find("function directionForWorkflowMode(mode)").unwrap();
    let direction_index = js[direction_function_index..]
        .find("if (isSchemaDatabaseToRepositoryMode(mode))")
        .map(|offset| direction_function_index + offset)
        .unwrap();
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
fn database_project_initialization_automation_selector_contract_is_present() {
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    for expected in [
        "data-testid=\"workspace-initialize-project\"",
        "data-testid=\"workspace-initialization-panel\"",
        "data-testid=\"workspace-initialization-confirmation-input\"",
        "data-testid=\"workspace-initialization-start\"",
        "data-testid=\"workspace-initialization-status\"",
        "data-testid=\"workspace-initialization-available\"",
        "data-testid=\"workspace-initialization-blocked\"",
        "data-testid=\"workspace-initialization-loading\"",
        "data-testid=\"workspace-initialization-success\"",
        "data-testid=\"workspace-initialization-error\"",
        "data-testid=\"workspace-initialization-error-code\"",
        "data-testid=\"workspace-initialization-error-message\"",
        "data-testid=\"workspace-project-status\"",
        "INITIALIZE DBSTATE PROJECT",
    ] {
        assert!(
            combined.contains(expected),
            "missing initialization automation contract {expected}"
        );
    }

    assert!(js.contains("function setWorkspaceInitializationState(stateName, data)"));
    assert!(js.contains("status.setAttribute(\"data-state\", stateValue);"));
    assert!(js.contains(
        "setHidden(\"workspace-initialization-available\", stateValue !== \"available\")"
    ));
    assert!(
        js.contains("setHidden(\"workspace-initialization-blocked\", stateValue !== \"blocked\")")
    );
    assert!(
        js.contains("setHidden(\"workspace-initialization-loading\", stateValue !== \"loading\")")
    );
    assert!(
        js.contains("setHidden(\"workspace-initialization-success\", stateValue !== \"success\")")
    );
    assert!(js.contains("setHidden(\"workspace-initialization-error\", stateValue !== \"error\")"));
    assert!(js.contains("\"protectedBranch\""));
    assert!(js.contains("setWorkspaceInitializationState(\"loading\""));
    assert!(js.contains("setWorkspaceInitializationState(\"blocked\""));
    assert!(js.contains("setWorkspaceInitializationState(\"success\""));
    assert!(js.contains("setWorkspaceInitializationState(\"available\""));
    assert!(!combined.contains("workspace-initialization-error-code\">git switch"));
}

#[test]
fn repository_directory_selectors_reuse_shared_path_slugging() {
    let js = ui_js();
    assert!(js.contains("return \"repository-directory-row-\" + repositoryObjectPathSlug(path);"));
    assert!(js.contains("return \"repository-directory-path-\" + repositoryObjectPathSlug(path);"));
    assert!(js.contains("return \"repository-directory-name-\" + repositoryObjectPathSlug(path);"));
    assert!(js.contains("code.textContent = path;"));
    assert!(js.contains("name.textContent = textOrEmpty(entry.name || path.split(\"/\").pop());"));

    let representative_paths = [
        "database",
        "database/objects",
        "database/objects/tables",
        "database/objects/constraints/primary-keys",
        "database/objects/materialized-views",
        "database/reference-data",
        "database/releases",
        "database/objects/Mixed Case/path.with.dots",
    ];
    let mut slugs = BTreeSet::new();
    for path in representative_paths {
        let slug = format!(
            "{}-{}",
            selector_readable_slug(path),
            stable_selector_hash(path)
        );
        assert!(slugs.insert(slug.clone()), "duplicate path slug {slug}");
    }

    let normalized_a = selector_readable_slug("database/objects/demo/a-b");
    let normalized_b = selector_readable_slug("database\\objects\\demo\\a_b");
    assert_eq!(normalized_a, normalized_b);
    assert_ne!(
        stable_selector_hash("database/objects/demo/a-b"),
        stable_selector_hash("database\\objects\\demo\\a_b")
    );
}

#[test]
fn initialization_on_protected_branch_is_blocked_without_creating_project_files() {
    let dir = create_temp_dir("init-write-protected-dev");
    init_git_repo(&dir);
    set_git_head_branch(&dir, "dev");
    let body =
        r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;

    let response = service_response("POST", "/api/v1/init/write", body, &dir);

    assert_eq!(response.status_code, 409, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("\"success\":false"));
    assert!(response.body.contains("\"branch\":\"dev\""));
    assert!(response.body.contains("\"isProtectedBranch\":true"));
    assert!(response
        .body
        .contains("\"dbstateProjectStatus\":\"gitRepositoryWithoutDbStateStructure\""));
    assert!(response.body.contains("Write blocked on protected branch"));
    assert!(response
        .body
        .contains("git switch -c dbstate/project-init/"));
    for expected in EXPECTED_PATHS {
        assert!(
            !dir.join(expected.relative).exists(),
            "protected init created {}",
            expected.relative
        );
    }
}

#[test]
fn initialization_retry_on_working_branch_creates_only_project_structure() {
    let dir = create_temp_dir("init-write-retry-working-branch");
    init_git_repo(&dir);
    set_git_head_branch(&dir, "dbstate/project-init/demo");
    let body =
        r#"{ "confirmInitializeProject": true, "confirmationText": "INITIALIZE DBSTATE PROJECT" }"#;

    let response = service_response("POST", "/api/v1/init/write", body, &dir);

    assert_eq!(response.status_code, 200, "{}", response.body);
    assert_common_json_contract(&response.body);
    assert!(response.body.contains("\"success\":true"));
    assert!(response
        .body
        .contains("\"dbstateProjectStatus\":\"completeDbStateStructure\""));
    for expected in EXPECTED_PATHS {
        assert!(
            dir.join(expected.relative).exists(),
            "missing initialized path {}",
            expected.relative
        );
    }
    assert!(!dir
        .join("database/objects/tables/public.country.sql")
        .exists());
    assert!(!dir.join("database/releases/objects/0001.sql").exists());
    assert_eq!(
        fs::read_to_string(dir.join("database/reference-data/dbstate.reference-data.yml"))
            .expect("read default registry"),
        DEFAULT_REGISTRY
    );
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
    let html = ui_html();
    let js = ui_js();
    let combined = format!("{html}\n{js}");

    assert!(js.contains("repositoryPath"));
    assert!(js.contains("workspace-path"));
    assert!(js.contains("attachWorkspacePath"));
    assert!(!js.contains("localStorage"));
    assert!(!js.contains("sessionStorage"));
    assert!(!js.contains("clone"));
    assert!(!js.contains("git fetch"));
    assert!(!js.contains("git pull"));
    assert!(!js.contains("git push"));
    assert!(combined.contains("Suggested Manual Git Commands"));
    assert!(js.contains("git add "));
    assert!(js.contains("git commit -m"));
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
        "--all".to_string(),
        "--include".to_string(),
        "database/objects/tables/public.country.sql".to_string()
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "sync".to_string(),
        "postgres".to_string(),
        "--all".to_string(),
        "--exclude".to_string(),
        "database/objects/tables/public.country.sql".to_string()
    ])
    .is_err());
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

#[test]
fn project_status_reports_missing_domain_folder() {
    let dir = create_temp_dir("missing-domain-folder");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::remove_dir_all(dir.join("database/objects/domains")).expect("remove domains dir");

    let report = status_report(&dir, CommandKind::RepoStatus);

    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::PartialDbStateStructure
    );
    assert!(report
        .missing_paths
        .contains(&"database/objects/domains".to_string()));
}

#[test]
fn project_status_reports_missing_aggregate_folder() {
    let dir = create_temp_dir("missing-aggregate-folder");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    fs::remove_dir_all(dir.join("database/objects/aggregates")).expect("remove aggregates dir");

    let report = status_report(&dir, CommandKind::RepoStatus);

    assert_eq!(
        report.dbstate_project_status,
        DbStateProjectStatus::PartialDbStateStructure
    );
    assert!(report
        .missing_paths
        .contains(&"database/objects/aggregates".to_string()));
}

#[test]
fn slice37_ci_validate_requires_disposable_before_sql_execution() {
    let dir = create_temp_dir("slice37-requires-disposable");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let (exit_code, output) = run_cli_output(
        &dir,
        &[
            "ci",
            "validate",
            "--repository",
            ".",
            "--postgres-url",
            "postgres://user:sensitive-marker@example.invalid/db",
        ],
    );

    assert_eq!(exit_code, 2);
    assert!(output.contains("Database State CI can execute repository-defined SQL only"));
    assert!(!output.contains("sensitive-marker"));
    assert!(!output.contains("example.invalid"));
}

#[test]
fn slice37_ci_validate_json_contract_when_disposable_missing() {
    let dir = create_temp_dir("slice37-json-disposable");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let (exit_code, output) = run_cli_output(
        &dir,
        &[
            "ci",
            "validate",
            "--repository",
            ".",
            "--postgres-url",
            "postgres://user:secret@example.invalid/db",
            "--json",
        ],
    );

    assert_eq!(exit_code, 2);
    for field in [
        "\"command\"",
        "\"success\"",
        "\"repositoryPath\"",
        "\"validationTarget\"",
        "\"scope\"",
        "\"disposable\"",
        "\"projectStructure\"",
        "\"objectFiles\"",
        "\"referenceData\"",
        "\"releaseArtifacts\"",
        "\"comparison\"",
        "\"warnings\"",
        "\"errors\"",
        "\"reportPath\"",
    ] {
        assert!(output.contains(field), "missing JSON field {field}");
    }
    assert!(output.contains("\"command\":\"ci validate\""));
    assert!(output.contains("\"success\":false"));
    assert!(!output.contains("secret"));
}

#[test]
fn slice37_ci_validate_missing_repository_returns_clear_error() {
    let dir = create_temp_dir("slice37-missing-repository");
    let missing = dir.join("missing");

    let (exit_code, output) = run_cli_output(
        &dir,
        &[
            "ci",
            "validate",
            "--repository",
            missing.to_str().expect("path"),
            "--postgres-url",
            "postgres://user:secret@example.invalid/db",
            "--disposable",
        ],
    );

    assert_eq!(exit_code, 2);
    assert!(output.contains("Repository path is not inside a Git repository."));
    assert!(!output.contains("secret"));
}

#[test]
fn slice37_ci_validate_non_dbstate_repository_returns_clear_error() {
    let dir = create_temp_dir("slice37-non-dbstate");
    init_git_repo(&dir);
    fs::write(dir.join("README.md"), "# Not DbState\n").expect("write readme");
    commit_all(&dir, "empty repository");

    let (exit_code, output) = run_cli_output(
        &dir,
        &[
            "ci",
            "validate",
            "--repository",
            ".",
            "--postgres-url",
            "postgres://user:secret@example.invalid/db",
            "--disposable",
        ],
    );

    assert_eq!(exit_code, 2);
    assert!(output.contains("DbState PostgreSQL project structure is incomplete"));
    assert!(!output.contains("secret"));
}

#[test]
fn slice37_ci_validate_rejects_dirty_dbstate_managed_worktree() {
    let dir = create_temp_dir("slice37-dirty-managed");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    fs::write(
        dir.join("database/objects/tables/public.customer.sql"),
        "CREATE TABLE \"public\".\"customer\" (\n    \"customer_id\" integer NOT NULL\n);\n",
    )
    .expect("write dirty managed file");

    let (exit_code, output) = run_cli_output(
        &dir,
        &[
            "ci",
            "validate",
            "--repository",
            ".",
            "--postgres-url",
            "postgres://user:secret@example.invalid/db",
            "--disposable",
        ],
    );

    assert_eq!(exit_code, 2);
    assert!(output.contains("Database State CI validates committed branch state"));
    assert!(output.contains("database/objects/tables/public.customer.sql"));
    assert!(!output.contains("secret"));
}

#[test]
fn slice37_ci_validate_rejects_report_paths_under_database() {
    let dir = create_temp_dir("slice37-report-database");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");

    let (exit_code, output) = run_cli_output(
        &dir,
        &[
            "ci",
            "validate",
            "--repository",
            ".",
            "--postgres-url",
            "postgres://user:secret@example.invalid/db",
            "--disposable",
            "--report",
            "database/ci-report.md",
        ],
    );

    assert_eq!(exit_code, 2);
    assert!(output.contains("CI report path must not be under database/."));
    assert!(!dir.join("database/ci-report.md").exists());
}

#[test]
fn slice37_ci_validate_writes_markdown_report_for_safe_failure() {
    let dir = create_temp_dir("slice37-report-safe-failure");
    init_git_repo(&dir);
    create_complete_structure(&dir);
    commit_all(&dir, "complete structure");
    let report_path = "target/dbstate-ci/database-state-ci-report.md";

    let (exit_code, output) = run_cli_output(
        &dir,
        &[
            "ci",
            "validate",
            "--repository",
            ".",
            "--postgres-url",
            "postgres://user:secret@127.0.0.1:1/dbstate_ci",
            "--disposable",
            "--report",
            report_path,
        ],
    );

    assert_eq!(exit_code, 2);
    assert!(output.contains("Report path:"));
    let report = fs::read_to_string(dir.join(report_path)).expect("read report");
    assert!(report.contains("# Database State CI Report"));
    assert!(report.contains("## Safety statement"));
    assert!(!report.contains("secret"));
    assert!(!report.contains("postgres://"));
}

#[test]
fn slice37_disposable_database_precheck_rejects_user_objects() {
    let inventory = sample_inventory();
    let errors = validate_disposable_database(&inventory).expect_err("non-empty database");

    assert!(errors
        .iter()
        .any(|error| error.contains("not empty/user-clean")));
    assert!(errors
        .iter()
        .any(|error| error.contains("Detected user object count")));
}

#[test]
fn slice37_object_apply_order_is_deterministic() {
    let objects = vec![
        desired_object(
            RepositoryObjectType::Index,
            "database/objects/indexes/public.customer_email_idx.sql",
        ),
        desired_object(
            RepositoryObjectType::Table,
            "database/objects/tables/public.customer.sql",
        ),
        desired_object(
            RepositoryObjectType::Domain,
            "database/objects/domains/public.customer_code.sql",
        ),
        desired_object(
            RepositoryObjectType::Aggregate,
            "database/objects/aggregates/public.group_concat.text.sql",
        ),
        desired_object(
            RepositoryObjectType::Schema,
            "database/objects/schemas/public.sql",
        ),
        desired_object(
            RepositoryObjectType::Constraint,
            "database/objects/constraints/foreign-keys/public.orders_customer_id_fkey.sql",
        ),
        desired_object(
            RepositoryObjectType::Function,
            "database/objects/functions/public._group_concat.text_text.sql",
        ),
        desired_object(
            RepositoryObjectType::Constraint,
            "database/objects/constraints/primary-keys/public.customer_pkey.sql",
        ),
    ];

    let ordered = ci_apply_order_for_test(objects);

    assert_eq!(
        ordered,
        vec![
            "database/objects/schemas/public.sql",
            "database/objects/domains/public.customer_code.sql",
            "database/objects/tables/public.customer.sql",
            "database/objects/functions/public._group_concat.text_text.sql",
            "database/objects/aggregates/public.group_concat.text.sql",
            "database/objects/constraints/primary-keys/public.customer_pkey.sql",
            "database/objects/constraints/foreign-keys/public.orders_customer_id_fkey.sql",
            "database/objects/indexes/public.customer_email_idx.sql",
        ]
    );
}

#[test]
fn slice37_ci_orders_partitioned_parent_table_before_primary_key() {
    let objects = vec![
        desired_object(
            RepositoryObjectType::Constraint,
            "database/objects/constraints/primary-keys/public.payment.payment_pkey.sql",
        ),
        desired_object(
            RepositoryObjectType::Table,
            "database/objects/tables/public.payment.sql",
        ),
    ];

    let ordered = ci_apply_order_for_test(objects);

    assert_eq!(
        ordered,
        vec![
            "database/objects/tables/public.payment.sql",
            "database/objects/constraints/primary-keys/public.payment.payment_pkey.sql",
        ]
    );
}

#[test]
fn slice37_ci_orders_materialized_views_before_indexes() {
    let objects = vec![
        desired_object(
            RepositoryObjectType::Index,
            "database/objects/indexes/public.customer.customer_email_idx.sql",
        ),
        desired_object(
            RepositoryObjectType::Index,
            "database/objects/indexes/public.rental_by_category.rental_category.sql",
        ),
        desired_object(
            RepositoryObjectType::MaterializedView,
            "database/objects/materialized-views/public.rental_by_category.sql",
        ),
        desired_object(
            RepositoryObjectType::Table,
            "database/objects/tables/public.customer.sql",
        ),
    ];

    let ordered = ci_apply_order_for_test(objects);

    assert_eq!(
        ordered,
        vec![
            "database/objects/tables/public.customer.sql",
            "database/objects/materialized-views/public.rental_by_category.sql",
            "database/objects/indexes/public.customer.customer_email_idx.sql",
            "database/objects/indexes/public.rental_by_category.rental_category.sql",
        ]
    );
}

#[test]
fn slice37_ci_orders_functions_before_aggregates_before_views() {
    let objects = vec![
        desired_object(
            RepositoryObjectType::View,
            "database/objects/views/public.actor_info.sql",
        ),
        desired_object(
            RepositoryObjectType::Aggregate,
            "database/objects/aggregates/public.group_concat.text.sql",
        ),
        desired_object(
            RepositoryObjectType::Function,
            "database/objects/functions/public._group_concat.text_text.sql",
        ),
    ];

    let ordered = ci_apply_order_for_test(objects);

    assert_eq!(
        ordered,
        vec![
            "database/objects/functions/public._group_concat.text_text.sql",
            "database/objects/aggregates/public.group_concat.text.sql",
            "database/objects/views/public.actor_info.sql",
        ]
    );
}

#[test]
fn slice37_function_dependency_retry_applies_later_dependency_then_retries() {
    let function_a = desired_object(
        RepositoryObjectType::Function,
        "database/objects/functions/public.film_in_stock.p_film_id_integer_p_store_id_integer_out_p_film_count_integer.sql",
    );
    let function_b = desired_object(
        RepositoryObjectType::Function,
        "database/objects/functions/public.inventory_in_stock.p_inventory_id_integer.sql",
    );
    let mut outcomes = BTreeMap::new();
    outcomes.insert(
        function_a.relative_path.clone(),
        vec![
            Err(CiSqlExecutionError {
                message: "SQL execution failed for database/objects/functions/public.film_in_stock.p_film_id_integer_p_store_id_integer_out_p_film_count_integer.sql (function): function inventory_in_stock(integer) does not exist\nHint: No function matches the given name and argument types.".to_string(),
                dependency_style: true,
            }),
            Ok(()),
        ],
    );

    let report = ci_apply_function_retry_for_test(vec![function_a, function_b], outcomes);

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.object_files.applied, 2);
    assert_eq!(report.object_files.failed, 0);
    assert!(report.warnings.iter().any(
        |warning| warning.contains("Deferred database/objects/functions/public.film_in_stock")
    ));
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("Retrying 1 deferred PostgreSQL function file")));
}

#[test]
fn slice37_function_retry_fails_unrecoverable_syntax_error_without_looping() {
    let function = desired_object(
        RepositoryObjectType::Function,
        "database/objects/functions/public.bad_function.no_args.sql",
    );
    let mut outcomes = BTreeMap::new();
    outcomes.insert(
        function.relative_path.clone(),
        vec![Err(CiSqlExecutionError {
            message: "SQL execution failed for database/objects/functions/public.bad_function.no_args.sql (function): syntax error at or near \"BROKEN\"".to_string(),
            dependency_style: false,
        })],
    );

    let report = ci_apply_function_retry_for_test(vec![function], outcomes);

    assert!(!report.success);
    assert_eq!(report.object_files.applied, 0);
    assert_eq!(report.object_files.failed, 1);
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("syntax error at or near")));
    assert!(!report
        .warnings
        .iter()
        .any(|warning| warning.contains("Deferred")));
}

#[test]
fn slice37_sql_execution_error_includes_safe_postgresql_diagnostics() {
    let object = desired_object(
        RepositoryObjectType::Function,
        "database/objects/functions/public.film_in_stock.p_film_id_integer_p_store_id_integer_out_p_film_count_integer.sql",
    );
    let message = format_ci_sql_execution_message(
        &object,
        "function inventory_in_stock(integer) does not exist",
        Some("Referenced function was not available yet."),
        Some("No function matches the given name and argument types. You might need to add explicit type casts."),
        Some("Original(212)"),
        "postgres://postgres:super-secret@127.0.0.1:55432/dbstate_ci_validation",
    );

    assert!(message.contains("database/objects/functions/public.film_in_stock"));
    assert!(message.contains("(function): function inventory_in_stock(integer) does not exist"));
    assert!(message.contains("Detail: Referenced function was not available yet."));
    assert!(message.contains("Hint: No function matches the given name and argument types."));
    assert!(message.contains("Position: Original(212)"));
    assert!(!message.contains("postgres://"));
    assert!(!message.contains("super-secret"));
}

#[test]
fn slice37_ci_skips_existing_bootstrap_public_schema() {
    let object = desired_object(
        RepositoryObjectType::Schema,
        "database/objects/schemas/public.sql",
    );

    assert!(ci_skips_bootstrap_public_schema_for_test(&object));
}

#[test]
fn slice37_release_artifacts_are_counted_but_not_executed() {
    let dir = create_temp_dir("slice37-release-summary");
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/releases/objects/0001_review.sql"),
        "-- REVIEW ONLY.\nSELECT 1;\n",
    )
    .expect("write object artifact");
    fs::write(
        dir.join("database/releases/reference-data/0001_ref.reference-data.sql"),
        "-- REVIEW ONLY.\nINSERT INTO ignored VALUES (1);\n",
    )
    .expect("write reference artifact");

    let summary = ci_release_summary_for_test(&dir);

    assert_eq!(summary.objects, 1);
    assert_eq!(summary.reference_data, 1);
    assert!(!summary.executed);
    assert!(summary
        .warnings
        .iter()
        .any(|warning| warning.contains("not executed")));
}

#[test]
fn slice37_reference_data_yaml_is_validated_without_dml_execution() {
    let dir = create_temp_dir("slice37-reference-data-validation");
    create_complete_structure(&dir);
    fs::write(
        dir.join("database/reference-data/dbstate.reference-data.yml"),
        valid_reference_registry_yaml(),
    )
    .expect("write registry");
    fs::create_dir_all(dir.join("database/reference-data/tables")).expect("create tables");
    fs::write(
        dir.join("database/reference-data/tables/dbstate_ref.payment_methods.yml"),
        valid_reference_table_yaml(),
    )
    .expect("write table yaml");

    let (summary, errors) = ci_reference_data_summary_for_test(&dir);

    assert!(errors.is_empty(), "{errors:?}");
    assert!(summary.registry_valid);
    assert_eq!(summary.table_files, 1);
}

#[test]
fn slice37_ci_json_includes_compare_back_drift_details() {
    let report = ci_report_with_compare_back_details_for_test(
        vec!["database/objects/tables/public.repo_only.sql"],
        vec!["database/objects/aggregates/public.group_concat.text.sql"],
        vec!["database/objects/tables/public.actor.sql"],
    );
    let json = report.to_json();

    assert!(json.contains("\"repositoryOnly\":1"));
    assert!(json.contains("\"databaseOnly\":1"));
    assert!(json.contains("\"different\":1"));
    assert!(json.contains("\"unexpectedDrift\":true"));
    assert!(json.contains("\"details\""));
    assert!(json.contains("\"status\":\"repositoryOnly\""));
    assert!(json.contains("\"objectType\":\"table\""));
    assert!(json.contains("\"identifier\":\"public.repo_only\""));
    assert!(json.contains("\"repositoryPath\":\"database/objects/tables/public.repo_only.sql\""));
    assert!(json.contains("\"status\":\"databaseOnly\""));
    assert!(json.contains("\"objectType\":\"aggregate\""));
    assert!(json.contains("\"identifier\":\"public.group_concat(text)\""));
    assert!(json.contains("\"databaseIdentifier\":\"public.group_concat(text)\""));
    assert!(json.contains("\"status\":\"different\""));
    assert!(json.contains("\"identifier\":\"public.actor\""));
    assert!(json.contains("\"repositoryPath\":\"database/objects/tables/public.actor.sql\""));
    assert!(json.contains("\"databaseIdentifier\":\"public.actor\""));
}

#[test]
fn slice37_ci_text_includes_compare_back_drift_details_only_when_needed() {
    let report = ci_report_with_compare_back_details_for_test(
        Vec::new(),
        vec!["database/objects/aggregates/public.group_concat.text.sql"],
        vec!["database/objects/tables/public.actor.sql"],
    );
    let text = report.to_text();

    assert!(text.contains("Compare-back drift details:"));
    assert!(text.contains("repositoryOnly:"));
    assert!(text.contains("    None"));
    assert!(text.contains("- aggregate public.group_concat(text)"));
    assert!(text.contains("database: public.group_concat(text)"));
    assert!(text.contains("- table public.actor"));
    assert!(text.contains("repository: database/objects/tables/public.actor.sql"));

    let clean = ci_report_with_compare_back_details_for_test(Vec::new(), Vec::new(), Vec::new());
    assert!(!clean.to_text().contains("Compare-back drift details:"));
}

#[test]
fn slice37_ci_compare_back_ignores_bootstrap_public_schema_grant_noise() {
    let report = ci_report_with_compare_back_details_for_test(
        Vec::new(),
        vec!["database/objects/grants/schemas/public.pg_database_owner.sql"],
        vec![
            "database/objects/grants/schemas/public.postgres.sql",
            "database/objects/grants/schemas/public.public.sql",
        ],
    );
    let json = report.to_json();
    let text = report.to_text();

    assert_eq!(report.comparison.repository_only, 0);
    assert_eq!(report.comparison.database_only, 0);
    assert_eq!(report.comparison.different, 0);
    assert!(!report.comparison.unexpected_drift);
    assert!(report.warnings.iter().any(|warning| warning.contains(
        "Ignored PostgreSQL bootstrap public schema grant noise during CI compare-back"
    )));
    assert!(json.contains("\"databaseOnly\":0"));
    assert!(json.contains("\"different\":0"));
    assert!(!text.contains("Compare-back drift details:"));
    assert!(!json.contains("schema.public.pg_database_owner"));
    assert!(!json.contains("schema.public.postgres"));
    assert!(!json.contains("schema.public.public"));
}

#[test]
fn slice37_ci_compare_back_still_reports_real_grant_drift() {
    let report = ci_report_with_compare_back_details_for_test(
        Vec::new(),
        vec![
            "database/objects/grants/schemas/app.pg_database_owner.sql",
            "database/objects/grants/tables/public.actor.public.sql",
            "database/objects/grants/sequences/public.actor_id_seq.public.sql",
            "database/objects/grants/functions/public.lookup_actor.no_args.public.sql",
            "database/objects/grants/materialized-views/public.rental_by_category.public.sql",
        ],
        vec!["database/objects/grants/schemas/app.postgres.sql"],
    );
    let json = report.to_json();

    assert_eq!(report.comparison.database_only, 5);
    assert_eq!(report.comparison.different, 1);
    assert!(report.comparison.unexpected_drift);
    assert!(json.contains("\"databaseOnly\":5"));
    assert!(json.contains("\"different\":1"));
    assert!(json.contains("\"identifier\":\"schema.app.pg_database_owner\""));
    assert!(json.contains("\"identifier\":\"table.public.actor.public\""));
    assert!(json.contains("\"identifier\":\"sequence.public.actor_id_seq.public\""));
    assert!(json.contains("\"identifier\":\"function.public.lookup_actor.no_args.public\""));
    assert!(json.contains("\"identifier\":\"materializedView.public.rental_by_category.public\""));
    assert!(json.contains("\"identifier\":\"schema.app.postgres\""));
}

#[test]
fn slice37_ci_markdown_report_includes_grouped_compare_back_drift_details() {
    let report = ci_report_with_compare_back_details_for_test(
        vec!["database/objects/tables/public.repo_only.sql"],
        Vec::new(),
        vec!["database/objects/tables/public.actor.sql"],
    );
    let markdown = report.to_markdown();

    assert!(markdown.contains("## Compare-back drift details"));
    assert!(markdown.contains("### Repository only"));
    assert!(markdown.contains("`table` `public.repo_only`"));
    assert!(markdown.contains("### Database only"));
    assert!(markdown.contains("None."));
    assert!(markdown.contains("### Different"));
    assert!(markdown.contains("`table` `public.actor`"));
}

#[test]
fn slice37_ci_compare_back_drift_details_do_not_include_sql_or_secrets() {
    let report = ci_report_with_compare_back_details_for_test(
        vec!["database/objects/tables/public.customer.sql"],
        Vec::new(),
        Vec::new(),
    );
    let combined = format!(
        "{}\n{}\n{}",
        report.to_text(),
        report.to_json(),
        report.to_markdown()
    );

    assert!(!combined.contains("CREATE TABLE"));
    assert!(!combined.contains("SELECT "));
    assert!(!combined.contains("postgres://"));
    assert!(!combined.contains("password"));
    assert!(!combined.contains("secret"));
}

#[test]
fn parser_accepts_ci_validate_and_rejects_legacy_url_for_ci() {
    assert!(ParsedArgs::parse(&[
        "ci".to_string(),
        "validate".to_string(),
        "--repository".to_string(),
        ".".to_string(),
        "--postgres-url".to_string(),
        "postgres://user:secret@example.invalid/db".to_string(),
        "--disposable".to_string(),
        "--report".to_string(),
        "target/dbstate-ci/report.md".to_string(),
    ])
    .is_ok());
    assert!(ParsedArgs::parse(&[
        "ci".to_string(),
        "validate".to_string(),
        "--repository".to_string(),
        ".".to_string(),
        "--url".to_string(),
        "postgres://user:secret@example.invalid/db".to_string(),
        "--disposable".to_string(),
    ])
    .is_err());
}

fn desired_object(object_type: RepositoryObjectType, relative_path: &str) -> DesiredStateObject {
    DesiredStateObject {
        object_type,
        schema_name: "public".to_string(),
        table_name: Some("customer".to_string()),
        object_name: "customer".to_string(),
        parent_name: None,
        relative_path: relative_path.to_string(),
        content: "-- desired state\n".to_string(),
    }
}
