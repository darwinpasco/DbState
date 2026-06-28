# DbState PostgreSQL v0.1 Slice 5 Implementation Notes

Slice 5 adds repository desired-state import and repository-to-PostgreSQL compare for the limited object set already supported by Slices 2 through 4.

The command is read-only for both sides:

- It reads supported local desired-state files under `database/objects/schemas/` and `database/objects/tables/`.
- It inspects PostgreSQL schemas, ordinary tables, and columns through read-only catalog queries.
- It reports whether supported objects are in sync, different, repository-only, or database-only.
- It does not write repository files.
- It does not execute DDL, DML, synchronization SQL, generated SQL, or deployment SQL.

## Command

Use an environment variable for the session-only PostgreSQL URL where practical:

```powershell
$env:DBSTATE_POSTGRES_URL = "<session-only-postgres-url>"
```

Compare all supported objects:

```powershell
cargo run -- compare postgres --all
```

Compare one schema and its supported tables:

```powershell
cargo run -- compare postgres --schema dbstate_slice2
```

Compare one supported table:

```powershell
cargo run -- compare postgres --table dbstate_slice2.sample_accounts
```

Return JSON:

```powershell
cargo run -- compare postgres --all --format json
```

The command also accepts `--url <session-only-postgres-url>`, but environment variable usage is preferred because command-line arguments can be retained in shell history.

## Compare Versus Export And Sync

`dbstate export postgres` creates missing desired-state files and skips existing files.

`dbstate sync postgres` is the explicit source database to repository synchronization command. It can create added files and update changed files under `database/objects/`.

`dbstate compare postgres` is read-only. It imports local desired-state text, renders the current PostgreSQL inventory with the same deterministic Slice 3 renderer, and compares normalized text. Differences are reported only. Differences do not make the command fail.

## Supported Scope

Slice 5 compares only:

- Schemas.
- Simple ordinary PostgreSQL tables.
- Columns inside supported tables.

Deferred object types are reported as deferred and are not compared deeply yet:

- Extensions.
- Enums.
- Sequences.
- Primary keys.
- Foreign keys.
- Unique constraints.
- Check constraints.
- Indexes.
- Views.
- Materialized views.
- Functions.
- Triggers.
- Grants.
- RLS policies.
- Comments.
- Ownership.
- Privileges.
- Reference-data rows.

Table definitions remain intentionally incomplete for full rebuild fidelity in Slice 5. Constraints, indexes, grants, ownership, comments, partitioning, and storage details are still out of scope.

## Classification

Compare results use these categories:

- `inSync`: local desired-state file exists and matches the rendered database object.
- `repoDifferent`: local desired-state file exists and differs from the rendered database object.
- `repoOnly`: local desired-state file exists, but the selected database object is not present in the inspected PostgreSQL inventory.
- `databaseOnly`: PostgreSQL object exists, but no supported local desired-state file exists.
- `skipped`: unsupported, deferred, invalid file name, unsafe path, or not selected.

## Repository Import Rules

Supported files are discovered from:

```text
database/objects/schemas/*.sql
database/objects/tables/*.sql
```

Schema file names map to schema names:

```text
database/objects/schemas/<schema-name>.sql
```

Table file names map to schema-qualified table names:

```text
database/objects/tables/<schema-name>.<table-name>.sql
```

Slice 5 does not parse SQL deeply. It uses file names for supported object identity and compares normalized file text against rendered live object text.

## Safety

The compare command:

- Does not persist the PostgreSQL URL.
- Does not write the PostgreSQL URL to repository files.
- Redacts connection strings from text output, JSON output, warnings, and errors.
- Does not create connection profiles.
- Does not create local machine-specific configuration.
- Does not stage, commit, or push Git changes.
- Does not read or write `database/releases/`.
- Does not apply SQL to a database.

The command may run when the working tree is dirty because it is read-only, but it reports repository status where available.

## Tests

Run normal tests:

```powershell
cargo test
```

Run formatting and lint checks:

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Optional PostgreSQL integration tests use the existing local fixture path:

```powershell
$env:DBSTATE_TEST_POSTGRES_URL = "<local-disposable-test-postgres-url>"
cargo test --test postgres_integration
```

Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database. The fixture SQL is test-only and may create schemas and tables in the configured disposable database.

## Current Limitations

- No target deployment planning exists yet.
- No SQL synchronization script generation exists yet.
- No deployment artifact generation under `database/releases/` exists yet.
- No data compare or reference-data row compare exists yet.
- No dependency analysis exists yet.
- No Deployment Rehearsal exists yet. It remains a premium later feature direction, not Slice 5 scope.
- No browser UI, DbState Service API, Docker product runtime, MCP server, or AI integration exists yet.
- No command applies SQL to a database.

## Open Decision

Explicit `--repo <path>` or project-folder selection should be considered later. Current Slice 5 behavior uses the current Git repository.
