# PostgreSQL Synchronization Workflows

DbState PostgreSQL v0.1 supports repository synchronization and target planning. It does not directly apply generated SQL to target PostgreSQL databases.

## Source Database to Repository Synchronization

When the source is a PostgreSQL database and the target is the local DbState repository, DbState may:

- Inspect the source PostgreSQL database.
- Compare source state against repository object files.
- Generate schema and configured reference-data diffs.
- Let the user select objects and reference-data changes.
- Update local object files after approval.
- Update configured reference-data files after approval.
- Stage, commit, and push only with explicit approval.

## Repository to Target Database Planning

When the repository is ahead of a target PostgreSQL database, DbState may:

- Inspect the target PostgreSQL database.
- Compare repository desired state against target state.
- Generate object-level diffs.
- Let the user cherry-pick changes.
- Analyze dependencies.
- Classify risk.
- Generate a SQL synchronization script.
- Generate risk and summary reports.
- Generate AI-reviewable artifacts.

DbState must not execute the generated script against the target database.

## Generated SQL Scripts

Generated SQL scripts are version-controlled deployment artifacts. They should be written under `database/releases/` by default.

Generated scripts are not the primary source of truth. Per-object files and configured reference-data files remain the desired-state source of truth.

## Cherry-Picking and Dependency Analysis

Users may include or exclude selected schema objects, object changes, reference tables, rows, and safe columns.

DbState must mark:

- Selected objects that may break if dependencies are excluded.
- Excluded objects required by selected objects.
- Dependent objects affected by an exclusion.

Critical missing dependencies should block script generation by default. Non-critical overrides may be allowed only with explicit user approval and must be recorded.

## Risk Report

Risk reports should identify:

- Destructive changes.
- Data-loss risk.
- Dependency warnings.
- Missing dependencies.
- Reference-data deletes.
- Grant or role risks.
- Unsupported or deferred object handling.
- Explicit overrides.

## Summary Report

Summary reports should list:

- Source and target context using non-secret labels.
- Git branch and commit hash if available.
- Selected objects.
- Excluded objects.
- Generated script path.
- Reference-data changes.
- Risk classification.
- Dependency warnings.

## AI-Reviewable Artifacts

DbState should generate artifacts that Codex or Claude can review without credentials or unmasked sensitive data.

AI agents may explain and summarize artifacts. They must not execute SQL, override warnings, or apply database changes.
