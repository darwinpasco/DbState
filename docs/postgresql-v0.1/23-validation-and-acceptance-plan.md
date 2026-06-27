# PostgreSQL v0.1 Validation and Acceptance Plan

## MVP Acceptance Scenario

1. Clone or open a Git repo.
2. Connect to a PostgreSQL source database.
3. Export or synchronize selected objects into per-object files.
4. Show visual diff.
5. Commit and push repo changes with approval.
6. Connect to a PostgreSQL target database.
7. Compare repo against target.
8. Cherry-pick changes.
9. Warn about missing dependencies.
10. Generate a version-controlled SQL synchronization script.
11. Generate summary and risk artifacts.
12. Let Codex or Claude review the artifacts.
13. Never apply the script directly to the target database.

## Validation Environments

- Local developer machine.
- PostgreSQL container fixture environment.
- Native Windows, macOS, and Linux smoke environments when packaging starts.
- Docker automation environment when Docker scope is implemented.

## Sample PostgreSQL Databases Needed

- Minimal source database with schemas, tables, constraints, indexes, and views.
- Function and trigger fixture database.
- Grants and roles fixture database.
- Reference-data fixture database.
- Target database with intentional drift.
- Unsupported object fixture database.

## Baseline Repo Fixture

The baseline repository fixture should contain:

- `database/objects/` with known desired-state files.
- `database/reference-data/dbstate.reference-data.yml`.
- `database/reference-data/tables/` with controlled table files.
- `database/releases/` for generated artifacts.

## Source Database Fixture

The source fixture should include objects that are missing, changed, and unchanged relative to the baseline repo.

## Target Database Fixture

The target fixture should include drift that requires generated SQL planning but no direct apply.

## Drift Scenarios

- Added schema.
- Added table.
- Changed column.
- Added constraint.
- Changed view.
- Changed function.
- Missing trigger.
- Grant drift.
- Extension present or missing.

## Reference-Data Scenarios

- Inserted row.
- Updated row.
- Configured delete.
- Ignored column change.
- Masked column change.
- Unconfigured table ignored.

## Dependency Warning Scenarios

- Selected view with excluded base table.
- Selected foreign key with excluded referenced key.
- Selected trigger with excluded trigger function.
- Selected materialized view with excluded base object.
- Selected reference-data row with excluded parent row.

## Generated Artifact Review

Generated artifacts should be reviewed for:

- Script metadata.
- Selected and excluded objects.
- Dependency warnings.
- Risk classification.
- Explicit overrides.
- No secrets.
- Git status visibility.

## AI-Review Context Validation

AI-review context should include deterministic artifacts only. It should exclude raw credentials, connection strings, tokens, local-only paths, unmasked PII, and transactional production data.

AI text should remain draft-only.

## Manual Review Checklist

- Git is source of truth.
- Per-object files are desired state.
- Generated SQL is a version-controlled deployment artifact.
- Source database to repo synchronization is approval-gated.
- Repo to target planning generates scripts only.
- Critical missing dependencies block by default.
- Generated artifacts are reviewable.
- No direct target-database apply exists.
- AI cannot execute SQL or override deterministic warnings.

## Done Criteria for v0.1

- The critical acceptance scenario passes.
- Required P0 backlog items pass validation.
- No-direct-apply negative tests pass.
- Secret leakage tests pass.
- PostgreSQL container fixture tests pass for supported object set.
- CLI JSON contracts are stable for MVP commands.
- UI workflow presents service output without doing core work directly.
- Docker behavior, if included in v0.1, matches native automation boundaries.
- Open decisions that block v0.1 are resolved or explicitly deferred.
