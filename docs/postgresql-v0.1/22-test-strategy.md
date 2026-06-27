# PostgreSQL v0.1 Test Strategy

## Purpose

Testing must prove deterministic behavior, safe artifact generation, PostgreSQL-native handling, and the no-direct-apply boundary.

Tests must verify DbState never directly applies generated synchronization scripts to target PostgreSQL databases.

## Unit Tests

Unit tests should cover:

- Object identity.
- File naming.
- Normalization.
- Diff classification.
- Dependency warning rules.
- Risk classification.
- Reference-data row matching.
- Masking and ignored-column behavior.
- Artifact metadata construction.

## Integration Tests

Integration tests should cover component interaction:

- Repository detection and initialization.
- Git status behavior.
- Source database to repository synchronization.
- Repository to target database comparison.
- Plan generation.
- Artifact generation.
- AI-review context generation.

## PostgreSQL Container Tests

PostgreSQL container tests should provide repeatable source and target databases.

Scenarios should include:

- Schemas, tables, columns, constraints, indexes, views, materialized views, functions, triggers, grants, enums, sequences, and extensions.
- Drift between source, repository, and target.
- Unsupported or deferred objects.
- Permission and connection failures.

## Golden-File Tests

Golden-file tests should verify:

- Exported object files.
- Imported desired-state object representations.
- Generated synchronization scripts.
- Summary markdown.
- Risk JSON.
- AI-review context artifacts.

Golden files should be reviewed when normalization rules change.

## Reference-Data Compare Tests

Tests should cover:

- Registry parsing.
- Business-key matching.
- Inserts.
- Updates.
- Configured deletes.
- Ignored columns.
- Masked columns.
- No unconfigured table comparison.

## Dependency Warning Tests

Tests should cover:

- Selected view with excluded base table.
- Selected function with excluded enum or table.
- Selected foreign key with excluded referenced key.
- Selected trigger with excluded trigger function.
- Selected materialized view with excluded base objects.
- Reference-data row with excluded parent row.

Critical missing dependencies should block script generation by default.

## Risk Classification Tests

Tests should cover destructive schema changes, reference-data deletes, missing dependencies, grants risks, unsupported objects, and explicit overrides.

## CLI JSON Contract Tests

CLI tests should validate JSON output for:

- `dbstate init`.
- `dbstate repo status`.
- `dbstate inspect`.
- `dbstate export`.
- `dbstate compare`.
- `dbstate plan`.
- `dbstate risk`.
- `dbstate data-compare`.

Every important command should produce stable JSON and clear non-zero exits for failures.

## Service API Contract Tests

Service API contract tests should cover project management, Git operations, connection profiles, schema inspection, schema compare, reference-data compare, plan generation, artifact generation, risk reporting, and AI-review context generation.

Exact routes remain open decisions.

## UI Workflow Tests Later

UI workflow tests should verify that the browser UI presents service results, shows dependency and risk warnings, supports artifact review, gates Git actions, and has no direct apply button.

## Security Tests

Security tests should verify:

- Credentials do not reach browser UI.
- Credentials do not appear in logs, repository files, generated artifacts, JSON output, or AI-review context.
- Masked reference-data columns stay masked.
- Local paths and tokens are not written to artifacts.

## Negative Tests for No-Direct-Apply

Negative tests should verify:

- No service operation executes generated SQL against a target database.
- No CLI command applies generated synchronization scripts to target databases.
- Docker automation does not expose direct target-database apply.
- UI has no direct apply workflow.
- AI context contains review data only and no executable action approval.

## Regression Tests for PostgreSQL Edge Cases

Regression tests should be added for overloaded functions, `search_path`, extension-owned objects, expression indexes, partial indexes, materialized views, grants, generated columns, identity columns, partitioned tables, object comments, and deferred objects.
