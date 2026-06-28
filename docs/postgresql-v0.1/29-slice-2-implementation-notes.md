# Slice 2 Implementation Notes

DbState PostgreSQL v0.1 Slice 2 adds read-only PostgreSQL catalog inspection for schemas, tables, and columns.

It does not implement persistent connection profiles, credential storage, object export, schema compare, data compare, synchronization planning, SQL synchronization script generation, browser UI, Docker product runtime, MCP, AI integration, or Deployment Rehearsal.

No command applies SQL to a database.

## Commands

Prefer the environment variable so the connection URL is not written into shell history:

```powershell
$env:DBSTATE_POSTGRES_URL = "<session-only-postgres-url>"
cargo run -- inspect postgres
```

JSON output:

```powershell
cargo run -- inspect postgres --format json
```

The command also accepts a session-only URL argument:

```powershell
cargo run -- inspect postgres --url "<session-only-postgres-url>"
```

If both `--url` and `DBSTATE_POSTGRES_URL` are set, `--url` takes precedence.

## Credential Safety

- The connection URL is session-only.
- The URL is not persisted.
- The URL is not written to repository files.
- The URL is not included in text output, JSON output, warnings, or errors.
- Persistent connection profiles are not implemented.

## Inspection Scope

Slice 2 inspects only:

- Schemas.
- Tables.
- Columns.

Internal schemas are excluded by default:

- `pg_catalog`.
- `information_schema`.
- `pg_toast*`.
- Other `pg_*` schemas.

Deferred object types are reported as deferred in JSON output. Extensions, enums, sequences, constraints, indexes, views, materialized views, functions, triggers, grants, and RLS policies are not modeled deeply in this slice.

## Read-Only Behavior

The product command uses read-only catalog queries. It does not execute DDL, DML, migration SQL, synchronization SQL, or generated SQL.

The inspection command does not create or update DbState project files.

## Tests

Normal tests:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Normal `cargo test` does not require a PostgreSQL database. The PostgreSQL integration test exits early unless `DBSTATE_TEST_POSTGRES_URL` is set.

## Local PostgreSQL Fixture Path

The integration fixture SQL is:

```text
tests/fixtures/postgresql/slice2-basic.sql
```

Use only a disposable local PostgreSQL test database. Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database.

Example local fixture flow:

```powershell
createdb dbstate_slice2_test
psql -d dbstate_slice2_test -f tests/fixtures/postgresql/slice2-basic.sql
$env:DBSTATE_TEST_POSTGRES_URL = "<session-only-postgres-url-for-disposable-test-db>"
cargo test --test postgres_integration
```

The fixture setup SQL creates test-only schemas and tables. Product code remains read-only.

## Current Limitations

- No object export exists yet.
- No compare workflow exists yet.
- No generated synchronization script workflow exists yet.
- No reference-data row compare exists yet.
- No browser UI exists yet.
- No Docker product runtime exists yet.
- No MCP or AI integration exists yet.
