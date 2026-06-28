use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use dbstate::{
    export_postgres_with_inventory, inspect_postgres, inspect_postgres_command, render_schema_sql,
    render_table_sql, sync_postgres_with_inventory, ExportSelection,
};
use postgres::{Client, NoTls};

const FIXTURE_SQL: &str = include_str!("fixtures/postgresql/slice2-basic.sql");

#[test]
fn local_postgres_fixture_inspection_is_read_only_and_redacted() {
    let Some(url) = std::env::var("DBSTATE_TEST_POSTGRES_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        eprintln!("skipping PostgreSQL integration test: DBSTATE_TEST_POSTGRES_URL is not set");
        return;
    };

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

    assert_eq!(inventory.schemas[0].name, "dbstate_slice2");
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

    let report = inspect_postgres_command(Some(url.clone()), None);
    let json = report.to_json();
    let text = report.to_text();
    assert!(report.success);
    assert!(report
        .deferred_object_types
        .contains(&"extensions".to_string()));
    assert!(!json.contains(&url));
    assert!(!text.contains(&url));
    assert!(!json.contains("slice2-sensitive-marker"));
    assert!(!text.contains("slice2-sensitive-marker"));
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
    assert!(!schema_file.contains(&url));
    assert!(!table_file.contains(&url));
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
                    column.schema_name == "dbstate_slice2"
                        && column.table_name == "sample_accounts"
                })
                .cloned()
                .collect::<Vec<_>>()
        )
    );
    assert!(!render_schema_sql("dbstate_slice2").contains(&url));
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
