# PostgreSQL v0.1 UI Workflows

The browser UI is the primary human presentation layer. It does not perform core work directly.

## Project Open and Clone

Users can open an existing repository or clone a repository through DbState. The UI shows project validation, Git branch, and working tree status.

## Connection Setup

Users configure PostgreSQL source and target connections. Credentials stay behind the service boundary. The browser UI must not receive raw credentials.

## Object Explorer

The object explorer shows PostgreSQL schemas, extensions, enums, sequences, tables, indexes, views, materialized views, functions, triggers, and grants.

## Visual Schema Compare

The schema compare view shows differences between source database and repository, or repository and target database.

Users can inspect object-level diffs before selecting changes.

## Visual Data Compare

The data compare view shows configured reference-data differences only. DbState must not treat all table data as version-controlled.

## Cherry-Pick Selection

Users can include or exclude specific objects, changes, reference tables, rows, and safe columns.

The UI must distinguish user-selected changes from auto-suggested dependencies.

## Dependency Warning Display

The UI must show:

- Required dependencies.
- Missing dependencies.
- Selected objects that may break.
- Excluded objects required by selected objects.
- Dependent objects affected by exclusions.

Critical missing dependencies should block script generation by default.

## Risk Review

Risk review shows destructive changes, reference-data deletes, dependency blockers, grants risks, unsupported object warnings, and explicit override requirements.

## Script Generation

Users generate SQL synchronization scripts only after reviewing selected scope, dependencies, and risk.

There is no direct apply button.

## Generated Artifact Review

The UI shows generated script path, summary markdown, risk JSON, dependency warnings, and AI-reviewable context.

Generated artifacts must appear in Git status and Git diff.

## Git Stage, Commit, and Push Review

Users can stage, unstage, commit, and push through explicit approval flows. DbState must not auto-commit or auto-push.

## AI-Assisted Draft Review

The UI may show AI-drafted branch names, commit messages, PR titles, PR bodies, and comments.

AI drafts must be editable and user-approved before use.

AI must not post, push, commit, create branches, create PRs, approve changes, or execute SQL without explicit user approval.
