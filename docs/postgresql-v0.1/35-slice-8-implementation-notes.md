# Slice 8 Implementation Notes

Slice 8 adds basic configured reference-data compare for DbState PostgreSQL v0.1.

This slice is compare-only. It reads explicit reference-data configuration and desired rows from the repository, queries configured PostgreSQL tables with read-only `SELECT`, and reports row differences by business key.

DbState does not generate DML. DbState does not generate release artifacts. DbState does not apply reference-data changes to a database.

## Registry

Configure controlled reference-data tables in:

```text
database/reference-data/dbstate.reference-data.yml
```

Example:

```yaml
version: 1
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
```

`tables: []` is valid and means no reference-data tables are controlled.

Rules:

- `name` must be schema-qualified.
- `file` must stay under `database/reference-data/tables/`.
- `key` must contain at least one column.
- Key columns cannot be ignored or masked.
- `ignoreColumns`, `maskedColumns`, and `allowDeletes` are optional.
- `allowDeletes` is recorded only. Slice 8 does not generate delete DML.

## Table File

Reference-data rows live under:

```text
database/reference-data/tables/
```

Example:

```yaml
table: dbstate_ref.payment_methods
key:
  - code
rows:
  - code: CASH
    name: Cash
    is_active: true
    sort_order: 10
  - code: QRPH
    name: QRPh
    is_active: true
    sort_order: 20
```

Rules:

- `table` must match the registry table name.
- `key` must match the registry key.
- Row keys must be unique.
- Every row must include the key columns.
- Row values must be scalar YAML values for Slice 8.

Do not store secrets, credentials, tokens, connection strings, local paths, production data, or unmasked PII in reference-data files.

## Command

Use `DBSTATE_POSTGRES_URL` for the session-only PostgreSQL connection string:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://user:password@localhost:5432/disposable_test_db"
cargo run -- data-compare postgres --all
```

Select one configured table:

```powershell
cargo run -- data-compare postgres --table dbstate_ref.payment_methods
```

JSON output:

```powershell
cargo run -- data-compare postgres --all --format json
```

`--url <postgres-url>` is also supported, but environment variable usage is preferred because command-line arguments can be retained in shell history.

The raw connection URL is not persisted and must not appear in text output, JSON output, errors, warnings, or logs.

## Compare Behavior

Slice 8 compares only tables listed in the registry. Unconfigured tables are ignored.

Rows are matched by business key and classified as:

- `inSync`
- `repoDifferent`
- `repoOnly`
- `databaseOnly`
- `skipped`

Ignored columns do not create differences.

Masked columns are not value-compared and raw masked values are not shown in text or JSON output. Output may show the masked column name so reviewers know masking affected comparison.

Changed columns are reported by name only. Before and after values are not dumped.

## Difference From Other Commands

`dbstate compare postgres` compares schema desired-state files against PostgreSQL schema objects.

`dbstate sync postgres` updates local schema desired-state files from a source PostgreSQL database.

`dbstate plan postgres` builds an in-memory schema plan with limited dependency warnings.

`dbstate release postgres` writes review-only release artifacts under `database/releases/`.

`dbstate data-compare postgres` is read-only. It compares configured reference-data rows and writes no files.

## Tests

Run the normal test suite:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Optional PostgreSQL integration tests use the existing gated local fixture pattern:

```powershell
$env:DBSTATE_TEST_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:55436/dbstate_slice8_test"
cargo test --test postgres_integration
```

The integration tests are gated. Normal `cargo test` does not require PostgreSQL.

Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database. The fixture setup is test-only and may create schemas, tables, and rows in the configured disposable database.

## Current Limitations

- Only explicitly configured reference-data tables are compared.
- Transactional data compare is not supported.
- Unconfigured tables are ignored.
- Binary columns, large object columns, complex nested values, environment overlays, dataset grouping, and policy files are not supported.
- No `INSERT`, `UPDATE`, `DELETE`, `MERGE`, DML, or synchronization script is generated.
- No deployment artifacts under `database/releases/` are generated.
- No target database apply exists.
- PostgreSQL access remains read-only in product commands.
- Repository compare is read-only.
- Open decision: explicit `--repo <path>` or project-folder selection should be considered later. Current behavior uses the current Git repository.
