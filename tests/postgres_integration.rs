use std::process::Command;

use dbstate::{inspect_postgres, inspect_postgres_command};
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
    assert!(!json.contains("slice2-secret"));
    assert!(!text.contains("slice2-secret"));
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
