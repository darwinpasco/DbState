use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use dbstate::{
    compare_postgres_with_inventory, data_compare_postgres_with_connection,
    export_postgres_with_inventory, inspect_postgres, inspect_postgres_command,
    plan_postgres_with_inventory, release_postgres_with_inventory, render_schema_sql,
    render_table_sql, sync_postgres_with_inventory, ExportSelection, PlanSelection,
    ReferenceDataSelection,
};
use postgres::{Client, NoTls};

const FIXTURE_SQL: &str = include_str!("fixtures/postgresql/slice2-basic.sql");
const REFERENCE_DATA_FIXTURE_SQL: &str =
    include_str!("fixtures/postgresql/slice8-reference-data.sql");
const SLICE16_OBJECT_COVERAGE_SQL: &str =
    include_str!("fixtures/postgresql/slice16-object-coverage.sql");

static POSTGRES_FIXTURE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_postgres_fixture() -> MutexGuard<'static, ()> {
    POSTGRES_FIXTURE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn local_postgres_fixture_inspection_is_read_only_and_redacted() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();

    assert!(
        !url.to_ascii_lowercase().contains("prod"),
        "DBSTATE_TEST_POSTGRES_URL must not point at production-looking databases"
    );
    assert!(
        !url.to_ascii_lowercase().contains("uat"),
        "DBSTATE_TEST_POSTGRES_URL must not point at UAT-looking databases"
    );
    assert!(
        !url.to_ascii_lowercase().contains("staging"),
        "DBSTATE_TEST_POSTGRES_URL must not point at staging-looking databases"
    );

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(FIXTURE_SQL)
        .expect("apply test-only fixture SQL");

    let inventory = inspect_postgres(&url).expect("inspect local disposable PostgreSQL fixture");

    assert!(inventory
        .schemas
        .iter()
        .any(|schema| schema.name == "dbstate_slice2"));
    assert!(inventory.tables.iter().any(
        |table| table.schema_name == "dbstate_slice2" && table.table_name == "sample_accounts"
    ));
    assert!(inventory
        .schemas
        .windows(2)
        .all(|pair| pair[0].name.as_str() <= pair[1].name.as_str()));
    assert!(inventory.tables.windows(2).all(|pair| {
        (&pair[0].schema_name, &pair[0].table_name) <= (&pair[1].schema_name, &pair[1].table_name)
    }));
    assert!(inventory.columns.windows(2).all(|pair| {
        (
            &pair[0].schema_name,
            &pair[0].table_name,
            pair[0].ordinal_position,
        ) <= (
            &pair[1].schema_name,
            &pair[1].table_name,
            pair[1].ordinal_position,
        )
    }));
    assert!(!inventory
        .schemas
        .iter()
        .any(|schema| schema.name == "pg_catalog"));
    assert!(!inventory
        .schemas
        .iter()
        .any(|schema| schema.name == "information_schema"));
    assert!(!inventory
        .schemas
        .iter()
        .any(|schema| schema.name.starts_with("pg_toast")));
    assert!(inventory.grants.iter().any(|grant| {
        grant.target_kind == "schema"
            && grant.schema_name == "dbstate_slice2"
            && grant.grantee == "PUBLIC"
            && grant.privileges == vec!["USAGE".to_string()]
    }));
    assert!(inventory.grants.iter().any(|grant| {
        grant.target_kind == "table"
            && grant.schema_name == "dbstate_slice2"
            && grant.object_name.as_deref() == Some("sample_accounts")
            && grant.grantee == "PUBLIC"
            && grant.privileges == vec!["SELECT".to_string()]
    }));
    assert!(inventory.rls_policies.iter().any(|policy| {
        policy.schema_name == "dbstate_slice2"
            && policy.table_name == "sample_accounts"
            && policy.policy_name == "sample_accounts_public_read"
            && policy.command == "SELECT"
            && policy.policy_kind == "PERMISSIVE"
            && policy.roles == vec!["PUBLIC".to_string()]
            && policy.using_expression.as_deref() == Some("true")
            && policy.table_rls_enabled == Some(true)
    }));

    let report = inspect_postgres_command(Some(url.clone()), None);
    let json = report.to_json();
    let text = report.to_text();
    assert!(report.success);
    assert!(!report
        .deferred_object_types
        .contains(&"extensions".to_string()));
    assert!(!json.contains(&url));
    assert!(!text.contains(&url));
    assert!(!json.contains("slice2-sensitive-marker"));
    assert!(!text.contains("slice2-sensitive-marker"));
}

#[test]
fn local_postgres_slice16_fixture_inspection_does_not_panic() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();
    assert_safe_test_url(&url);

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(SLICE16_OBJECT_COVERAGE_SQL)
        .expect("apply test-only Slice 16 fixture SQL");

    let inventory =
        inspect_postgres(&url).expect("inspect local disposable Slice 16 PostgreSQL fixture");

    assert!(inventory
        .schemas
        .iter()
        .any(|schema| schema.name == "dbstate_slice16"));
    assert!(inventory
        .extensions
        .iter()
        .any(|extension| extension.extension_name == "pgcrypto"));
    assert!(inventory.enums.iter().any(|enum_info| {
        enum_info.schema_name == "dbstate_slice16"
            && enum_info.enum_name == "account_status"
            && enum_info.labels == vec!["active".to_string(), "closed".to_string()]
    }));
    assert!(inventory.sequences.iter().any(|sequence| {
        sequence.schema_name == "dbstate_slice16"
            && sequence.sequence_name == "account_number_seq"
            && sequence.data_type.as_deref() == Some("bigint")
    }));
    assert!(inventory.indexes.iter().any(|index| {
        index.schema_name == "dbstate_slice16"
            && index.table_name == "sample_accounts"
            && index.index_name == "sample_accounts_code_idx"
    }));
    assert!(inventory.views.iter().any(|view| {
        view.schema_name == "dbstate_slice16" && view.view_name == "active_accounts"
    }));

    let report = inspect_postgres_command(Some(url.clone()), None);
    let json = report.to_json();

    assert!(report.success, "{:?}", report.errors);
    assert!(json.contains("\"extensions\""));
    assert!(json.contains("\"sequences\""));
    assert!(json.contains("\"indexes\""));
    assert!(json.contains("\"views\""));
    assert!(!json.contains(&url));
    assert!(!json.contains("dbstate_test_only"));
}

#[test]
fn inspect_command_does_not_create_project_files() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();

    let temp = std::env::temp_dir().join(format!(
        "dbstate-slice2-inspect-no-files-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp).expect("create temp dir");
    Command::new("git")
        .arg("init")
        .current_dir(&temp)
        .output()
        .expect("run git init");

    let before = temp.join("database").exists();
    let report = inspect_postgres_command(Some(url), None);
    let after = temp.join("database").exists();

    assert!(report.success);
    assert_eq!(before, after);
}

#[test]
fn local_postgres_fixture_export_plans_and_writes_expected_files() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();
    assert_safe_test_url(&url);

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(FIXTURE_SQL)
        .expect("apply test-only fixture SQL");

    let inventory = inspect_postgres(&url).expect("inspect local disposable PostgreSQL fixture");
    let repo = disposable_git_repo();

    let dry_run = export_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, true);
    assert!(dry_run.success);
    assert!(dry_run
        .planned_files
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(dry_run.planned_files.contains(
        &"database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql".to_string()
    ));
    assert!(!repo
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());

    let export = export_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, false);
    assert!(export.success);
    let schema_file =
        std::fs::read_to_string(repo.join("database/objects/schemas/dbstate_slice2.sql"))
            .expect("read schema file");
    let table_file = std::fs::read_to_string(
        repo.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
    )
    .expect("read table file");

    assert!(schema_file.contains("CREATE SCHEMA \"dbstate_slice2\";"));
    assert!(table_file.contains("CREATE TABLE \"dbstate_slice2\".\"sample_accounts\""));
    let grant_file = std::fs::read_to_string(
        repo.join("database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql"),
    )
    .expect("read grant file");
    assert!(grant_file
        .contains("GRANT SELECT ON TABLE \"dbstate_slice2\".\"sample_accounts\" TO PUBLIC;"));
    assert!(!schema_file.contains(&url));
    assert!(!table_file.contains(&url));
    assert!(!grant_file.contains(&url));
}

#[test]
fn local_postgres_fixture_sync_plans_creates_updates_and_unchanged_files() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();
    assert_safe_test_url(&url);

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(FIXTURE_SQL)
        .expect("apply test-only fixture SQL");

    let inventory = inspect_postgres(&url).expect("inspect local disposable PostgreSQL fixture");
    let repo = disposable_git_repo();

    let dry_run = sync_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, true);
    assert!(dry_run.success);
    assert!(dry_run
        .planned_creates
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(!repo
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());

    let create = sync_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, false);
    assert!(create.success);
    assert!(create
        .created_files
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(create
        .created_files
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    assert!(create.created_files.contains(
        &"database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql".to_string()
    ));

    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "synced files",
        ],
    );

    let unchanged = sync_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, true);
    assert!(unchanged.success);
    assert!(unchanged
        .unchanged_files
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));

    let table_path = repo.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    std::fs::write(&table_path, "-- changed locally\n").expect("write changed table");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "local table change",
        ],
    );

    let update = sync_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        false,
    );
    assert!(update.success);
    assert!(update
        .updated_files
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    assert_eq!(
        std::fs::read_to_string(table_path).expect("read updated table"),
        render_table_sql(
            "dbstate_slice2",
            "sample_accounts",
            &inventory
                .columns
                .iter()
                .filter(|column| {
                    column.schema_name == "dbstate_slice2" && column.table_name == "sample_accounts"
                })
                .cloned()
                .collect::<Vec<_>>()
        )
    );
    assert!(!render_schema_sql("dbstate_slice2").contains(&url));
}

#[test]
fn local_postgres_fixture_compare_classifies_supported_objects_read_only() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();
    assert_safe_test_url(&url);

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(FIXTURE_SQL)
        .expect("apply test-only fixture SQL");

    let inventory = inspect_postgres(&url).expect("inspect local disposable PostgreSQL fixture");
    let repo = disposable_git_repo();

    let empty_compare = compare_postgres_with_inventory(&repo, &inventory, &ExportSelection::All);
    assert!(empty_compare.success);
    assert!(empty_compare
        .database_only
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(!repo
        .join("database/objects/schemas/dbstate_slice2.sql")
        .exists());

    let sync = sync_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, false);
    assert!(sync.success);
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "synced files",
        ],
    );

    let in_sync = compare_postgres_with_inventory(&repo, &inventory, &ExportSelection::All);
    assert!(in_sync.success);
    assert!(in_sync
        .in_sync
        .contains(&"database/objects/schemas/dbstate_slice2.sql".to_string()));
    assert!(in_sync
        .in_sync
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));
    assert!(in_sync.in_sync.contains(
        &"database/objects/grants/tables/dbstate_slice2.sample_accounts.public.sql".to_string()
    ));
    assert!(!in_sync
        .deferred_object_types
        .contains(&"indexes".to_string()));

    let table_path = repo.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    std::fs::write(&table_path, "-- local drift\n").expect("write local drift");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "local drift",
        ],
    );

    let different = compare_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
    );
    assert!(different.success);
    assert!(different
        .repo_different
        .contains(&"database/objects/tables/dbstate_slice2.sample_accounts.sql".to_string()));

    std::fs::write(
        repo.join("database/objects/tables/dbstate_slice2.repo_only.sql"),
        "-- repo only table\n",
    )
    .expect("write repo-only file");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "repo-only file",
        ],
    );

    let repo_only = compare_postgres_with_inventory(&repo, &inventory, &ExportSelection::All);
    assert!(repo_only.success);
    assert!(repo_only
        .repo_only
        .contains(&"database/objects/tables/dbstate_slice2.repo_only.sql".to_string()));

    let json = repo_only.to_json();
    let text = repo_only.to_text();
    assert!(!json.contains(&url));
    assert!(!text.contains(&url));
    assert!(!json.contains("slice2-sensitive-marker"));
    assert!(!text.contains("slice2-sensitive-marker"));
}

#[test]
fn local_postgres_fixture_plan_generates_selected_items_and_dependency_warnings() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();
    assert_safe_test_url(&url);

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(FIXTURE_SQL)
        .expect("apply test-only fixture SQL");

    let inventory = inspect_postgres(&url).expect("inspect local disposable PostgreSQL fixture");
    let repo = disposable_git_repo();

    let sync = sync_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, false);
    assert!(sync.success);
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "synced files",
        ],
    );

    let in_sync_plan = plan_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );
    assert!(in_sync_plan.success);
    assert!(in_sync_plan.plan_items.is_empty());
    assert!(in_sync_plan.blocked_items.is_empty());

    let table_path = repo.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    std::fs::write(&table_path, "-- local drift\n").expect("write local drift");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "local drift",
        ],
    );

    let table_only_selection = PlanSelection::from_options(
        vec!["table:dbstate_slice2.sample_accounts".to_string()],
        Vec::new(),
    )
    .expect("plan selection");
    let table_plan = plan_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::All,
        &table_only_selection,
    );
    assert!(table_plan.success);
    assert_eq!(table_plan.plan_items.len(), 1);
    assert_eq!(table_plan.plan_items[0].plan_intent, "updateDatabaseLater");

    std::fs::write(
        repo.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "repo-only schema",
        ],
    );

    let repo_only_plan = plan_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::All,
        &PlanSelection::include_all(),
    );
    assert!(repo_only_plan.success);
    assert!(repo_only_plan.plan_items.iter().any(|item| {
        item.object_ref == "schema:local_only" && item.plan_intent == "createInDatabaseLater"
    }));

    std::fs::remove_file(repo.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"))
        .expect("remove table file");
    std::fs::remove_file(repo.join("database/objects/schemas/dbstate_slice2.sql"))
        .expect("remove schema file");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "remove dbstate_slice2 files",
        ],
    );

    let missing_schema_plan = plan_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
    );
    assert!(missing_schema_plan.success);
    assert!(missing_schema_plan
        .blocked_items
        .iter()
        .any(|item| { item.object_ref == "table:dbstate_slice2.sample_accounts" }));
    assert!(missing_schema_plan
        .dependency_warnings
        .iter()
        .any(|warning| warning.warning_type == "missingDependency"));

    let json = missing_schema_plan.to_json();
    let text = missing_schema_plan.to_text();
    assert!(!json.contains(&url));
    assert!(!text.contains(&url));
    assert!(!json.contains("slice2-sensitive-marker"));
    assert!(!text.contains("slice2-sensitive-marker"));
    assert!(!repo.join("database/releases").join("slice6.sql").exists());
}

#[test]
fn local_postgres_fixture_release_generates_artifacts_without_database_apply() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();
    assert_safe_test_url(&url);

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(FIXTURE_SQL)
        .expect("apply test-only fixture SQL");

    let inventory = inspect_postgres(&url).expect("inspect local disposable PostgreSQL fixture");
    let repo = disposable_git_repo();

    let sync = sync_postgres_with_inventory(&repo, &inventory, &ExportSelection::All, false);
    assert!(sync.success);
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "synced files",
        ],
    );

    let table_path = repo.join("database/objects/tables/dbstate_slice2.sample_accounts.sql");
    let original_table_content = std::fs::read_to_string(&table_path).expect("read table");
    std::fs::write(&table_path, "-- local drift for release\n").expect("write local drift");
    std::fs::write(
        repo.join("database/objects/schemas/local_only.sql"),
        render_schema_sql("local_only"),
    )
    .expect("write repo-only schema");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "release candidate files",
        ],
    );
    let drifted_table_content = std::fs::read_to_string(&table_path).expect("read drifted table");
    assert_ne!(original_table_content, drifted_table_content);

    let dry_run = release_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::All,
        &PlanSelection::include_all(),
        "slice7_fixture",
        true,
    );
    assert!(dry_run.success, "{:?}", dry_run.errors);
    assert!(dry_run
        .planned_artifacts
        .contains(&"database/releases/0001_slice7_fixture.sql".to_string()));
    assert!(!repo
        .join("database/releases/0001_slice7_fixture.sql")
        .exists());

    let release = release_postgres_with_inventory(
        &repo,
        &inventory,
        &ExportSelection::All,
        &PlanSelection::include_all(),
        "slice7_fixture",
        false,
    );
    assert!(release.success, "{:?}", release.errors);
    assert!(release
        .created_artifacts
        .contains(&"database/releases/0001_slice7_fixture.sql".to_string()));
    assert!(release
        .created_artifacts
        .contains(&"database/releases/0001_slice7_fixture.summary.md".to_string()));
    assert!(release
        .created_artifacts
        .contains(&"database/releases/0001_slice7_fixture.risk.json".to_string()));

    assert_eq!(
        std::fs::read_to_string(&table_path).expect("read table after release"),
        drifted_table_content
    );

    let sql = std::fs::read_to_string(repo.join("database/releases/0001_slice7_fixture.sql"))
        .expect("read sql artifact");
    let summary =
        std::fs::read_to_string(repo.join("database/releases/0001_slice7_fixture.summary.md"))
            .expect("read summary artifact");
    let risk =
        std::fs::read_to_string(repo.join("database/releases/0001_slice7_fixture.risk.json"))
            .expect("read risk artifact");
    let json = release.to_json();
    let text = release.to_text();

    assert!(sql.contains("CREATE SCHEMA IF NOT EXISTS \"local_only\";"));
    assert!(sql.contains("REVIEW REQUIRED: object differs"));
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
    assert!(risk.contains("\"destructiveSqlGenerated\":false"));
    assert!(risk.contains("\"directApplyAvailable\":false"));
    assert!(risk.contains("\"generatedSqlExecutionSupported\":false"));

    for output in [&sql, &summary, &risk, &json, &text] {
        assert!(!output.contains(&url));
        assert!(!output.contains("dbstate_test_only"));
        assert!(!output.contains("slice2-sensitive-marker"));
    }

    let blocked_repo = disposable_git_repo();
    std::fs::write(
        blocked_repo.join("database/objects/tables/dbstate_slice2.sample_accounts.sql"),
        "-- local drift without schema\n",
    )
    .expect("write blocked table");
    run_git(&blocked_repo, &["add", "."]);
    run_git(
        &blocked_repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "blocked release candidate",
        ],
    );
    let blocked = release_postgres_with_inventory(
        &blocked_repo,
        &inventory,
        &ExportSelection::Table {
            schema: "dbstate_slice2".to_string(),
            table: "sample_accounts".to_string(),
        },
        &PlanSelection::include_all(),
        "slice7_blocked",
        false,
    );
    assert!(!blocked.success);
    assert_eq!(blocked.risk_level, "blocked");
    assert!(blocked.created_artifacts.is_empty());
    assert!(blocked
        .blocked_items
        .iter()
        .any(|item| item.object_ref == "table:dbstate_slice2.sample_accounts"));
}

#[test]
fn local_postgres_fixture_reference_data_compare_is_read_only_and_masked() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };
    let _fixture_guard = lock_postgres_fixture();
    assert_safe_test_url(&url);

    let mut client =
        Client::connect(&url, NoTls).expect("connect to local disposable test database");
    client
        .batch_execute(REFERENCE_DATA_FIXTURE_SQL)
        .expect("apply test-only reference-data fixture SQL");

    let repo = disposable_git_repo();
    std::fs::write(
        repo.join("database/reference-data/dbstate.reference-data.yml"),
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
",
    )
    .expect("write reference registry");
    std::fs::write(
        repo.join("database/reference-data/tables/dbstate_ref.payment_methods.yml"),
        "table: dbstate_ref.payment_methods
key:
  - code
rows:
  - code: CASH
    name: Cash
    is_active: true
    sort_order: 10
    updated_at: ignored repo value
    secret_note: slice8-repo-secret-cash
  - code: QRPH
    name: QRPh Desired
    is_active: true
    sort_order: 20
    updated_at: ignored repo value
    secret_note: slice8-repo-secret-qrph
  - code: WIRE
    name: Wire
    is_active: false
    sort_order: 40
    updated_at: ignored repo value
    secret_note: slice8-repo-secret-wire
",
    )
    .expect("write reference table file");
    run_git(&repo, &["add", "."]);
    run_git(
        &repo,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "configured reference data",
        ],
    );

    let report = data_compare_postgres_with_connection(&repo, &url, &ReferenceDataSelection::All)
        .expect("reference data compare");
    assert!(report.success, "{:?}", report.errors);
    assert_eq!(report.counts.in_sync, 1);
    assert_eq!(report.counts.repo_different, 1);
    assert_eq!(report.counts.repo_only, 1);
    assert_eq!(report.counts.database_only, 1);
    assert!(report
        .repo_different
        .contains(&"dbstate_ref.payment_methods:code=QRPH".to_string()));
    assert!(report
        .repo_only
        .contains(&"dbstate_ref.payment_methods:code=WIRE".to_string()));
    assert!(report
        .database_only
        .contains(&"dbstate_ref.payment_methods:code=CARD".to_string()));

    let table_only = data_compare_postgres_with_connection(
        &repo,
        &url,
        &ReferenceDataSelection::Table("dbstate_ref.payment_methods".to_string()),
    )
    .expect("table reference data compare");
    assert!(table_only.success);
    assert_eq!(
        table_only.selected_tables,
        vec!["dbstate_ref.payment_methods".to_string()]
    );

    let unconfigured = data_compare_postgres_with_connection(
        &repo,
        &url,
        &ReferenceDataSelection::Table("dbstate_ref.unconfigured".to_string()),
    )
    .expect("unconfigured table report");
    assert!(!unconfigured.success);
    assert!(unconfigured
        .errors
        .iter()
        .any(|error| error.contains("not configured")));

    let json = report.to_json();
    let text = report.to_text();
    for output in [&json, &text] {
        assert!(!output.contains(&url));
        assert!(!output.contains("dbstate_test_only"));
        assert!(!output.contains("slice8-repo-secret"));
        assert!(!output.contains("slice8-db-secret"));
        assert!(!output.contains("ignored repo value"));
    }
    assert!(!repo
        .join("database/releases/0001_data_compare.sql")
        .exists());
}

fn assert_safe_test_url(url: &str) {
    assert!(
        !url.to_ascii_lowercase().contains("prod"),
        "DBSTATE_TEST_POSTGRES_URL must not point at production-looking databases"
    );
    assert!(
        !url.to_ascii_lowercase().contains("uat"),
        "DBSTATE_TEST_POSTGRES_URL must not point at UAT-looking databases"
    );
    assert!(
        !url.to_ascii_lowercase().contains("staging"),
        "DBSTATE_TEST_POSTGRES_URL must not point at staging-looking databases"
    );
}

fn disposable_git_repo() -> PathBuf {
    let temp = std::env::temp_dir().join(format!(
        "dbstate-slice3-export-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp).expect("create temp dir");
    run_git(&temp, &["init"]);
    create_complete_structure(&temp);
    run_git(&temp, &["add", "."]);
    run_git(
        &temp,
        &[
            "-c",
            "user.email=dbstate@example.invalid",
            "-c",
            "user.name=DbState Test",
            "commit",
            "-m",
            "complete structure",
        ],
    );
    temp
}

fn create_complete_structure(root: &Path) {
    for relative in [
        "database/objects/schemas",
        "database/objects/extensions",
        "database/objects/enums",
        "database/objects/sequences",
        "database/objects/tables",
        "database/objects/indexes",
        "database/objects/views",
        "database/objects/materialized-views",
        "database/objects/functions",
        "database/objects/triggers",
        "database/objects/grants",
        "database/objects/grants/schemas",
        "database/objects/grants/tables",
        "database/objects/grants/views",
        "database/objects/grants/materialized-views",
        "database/objects/grants/sequences",
        "database/objects/grants/functions",
        "database/reference-data/tables",
        "database/releases",
    ] {
        std::fs::create_dir_all(root.join(relative)).expect("create project directory");
    }
    std::fs::write(
        root.join("database/reference-data/dbstate.reference-data.yml"),
        "version: 1\ntables: []\n",
    )
    .expect("create registry");
}

fn run_git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
