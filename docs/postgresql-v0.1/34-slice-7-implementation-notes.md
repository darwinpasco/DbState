# Slice 7 Implementation Notes

Slice 7 adds release artifact generation for DbState PostgreSQL v0.1.

The command builds on the Slice 6 in-memory plan. It reads supported repository desired-state files, inspects PostgreSQL with read-only catalog queries, builds a selected plan, and writes review artifacts under `database/releases/` only when explicitly invoked without `--dry-run`.

DbState does not execute generated SQL. DbState does not apply changes to a database.

## Command

```powershell
cargo run -- release postgres --all --name slice7_test
```

Use `DBSTATE_POSTGRES_URL` for the session-only PostgreSQL connection string:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://user:password@localhost:5432/disposable_test_db"
cargo run -- release postgres --all --name slice7_test
```

`--url <postgres-url>` is also supported, but environment variable usage is preferred because command-line arguments can be retained in shell history.

The raw connection URL is not persisted and must not appear in text output, JSON output, errors, warnings, or generated artifacts.

## Selection

Slice 7 requires an explicit scope:

```powershell
cargo run -- release postgres --schema dbstate_slice2 --name schema_release
cargo run -- release postgres --table dbstate_slice2.sample_accounts --name table_release
cargo run -- release postgres --all --name full_supported_scope
```

Cherry-pick selection uses Slice 6 object references:

```powershell
cargo run -- release postgres --all --include schema:local_only --name selected_release
cargo run -- release postgres --all --exclude schema:dbstate_slice2 --name selected_release
```

## Dry Run

Dry-run mode reads the repository, inspects PostgreSQL read-only, builds the selected plan, and reports planned artifacts without writing files:

```powershell
cargo run -- release postgres --all --name slice7_test --dry-run
cargo run -- release postgres --all --name slice7_test --dry-run --format json
```

Dry-run can run when the working tree is dirty. Write mode requires a clean working tree.

## Generated Artifacts

Slice 7 originally wrote a three-file artifact bundle under `database/releases/`:

- `0001_<release-name>.sql`
- `0001_<release-name>.summary.md`
- `0001_<release-name>.risk.json`

If `0001_<release-name>.*` already exists, the next available sequence is used, such as `0002_<release-name>.*`.

Release names are normalized to lowercase and must use only letters, numbers, hyphen, or underscore. Path separators, path traversal, spaces, and unsafe characters are rejected.

Slice 17 hardens the current release bundle and adds `0001_<release-name>.manifest.json`, sectioned SQL review output, richer summary markdown, and stable risk JSON fields. See `47-slice-17-release-artifact-hardening.md` for the current artifact contract.

## SQL Generation

Generated SQL is a review artifact, not an operation DbState runs.

Slice 7 supports only:

- `CREATE SCHEMA IF NOT EXISTS` for repo-only schema objects.
- `CREATE TABLE IF NOT EXISTS` for repo-only simple ordinary/base table objects.
- Review-only comments for changed schema or table objects.

Changed simple table items do not produce `ALTER TABLE` statements in Slice 7. Destructive SQL is not generated.

Slice 7 does not generate:

- `DROP TABLE`
- `DROP SCHEMA`
- `DROP COLUMN`
- `ALTER TABLE DROP`
- `TRUNCATE`
- `DELETE`
- `UPDATE`
- row `INSERT`
- `GRANT`
- `REVOKE`
- `ALTER OWNER`

## Blocked Items

Release artifact generation is blocked when the selected Slice 6 plan contains blocked items.

The current dependency rule remains narrow:

- A table desired-state object depends on its schema desired-state object.

If a selected table is missing its required schema file, the plan item is blocked and release artifacts are not written.

## JSON Output

```powershell
cargo run -- release postgres --all --name slice7_test --format json
```

The JSON output includes release name, scope, selected objects, plan items, blocked items, dependency warnings, planned artifacts, created artifacts, risk level, warnings, errors, deferred object types, and working tree status.

The risk JSON artifact always includes:

- `destructiveSqlGenerated: false`
- `directApplyAvailable: false`
- `generatedSqlExecutionSupported: false`

Slice 17 adds explicit `databaseMutationPerformed: false`, `gitMutationPerformed: false`, and `credentialPersistencePerformed: false` fields to the risk JSON artifact.

## Tests

Run the normal test suite:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Optional PostgreSQL integration tests use the existing local fixture path:

```powershell
$env:DBSTATE_TEST_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:55436/dbstate_slice6_test"
cargo test --test postgres_integration
```

The integration tests are gated. Normal `cargo test` does not require PostgreSQL.

Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database. The fixture setup is test-only and may create or replace test schemas in the configured disposable database.

## Current Limitations

- Only schemas and simple ordinary/base tables with columns are handled.
- Constraints, indexes, views, functions, triggers, grants, reference-data rows, and other object types are deferred.
- Dependency analysis is limited to table depends on schema.
- No target database apply exists.
- No generated SQL is executed.
- PostgreSQL access remains read-only in product commands.
- Release artifact generation writes only under `database/releases/`.
- No browser UI, DbState Service API, Docker product runtime, MCP, AI integration, Deployment Rehearsal, or cross-RDBMS support exists in this slice.
- Open decision: explicit `--repo <path>` or project-folder selection should be considered later. Current behavior uses the current Git repository.
