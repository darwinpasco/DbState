# Selective Sync and Dependency Analysis

Visual schema compare must allow users to cherry-pick objects and changes. Visual data compare must allow users to cherry-pick configured reference-data changes.

Cherry-picking must be dependency-aware.

## Required behavior

DbState must:

- Mark selected objects that may break because dependencies are excluded.
- Mark excluded objects that are required by selected objects.
- Show dependent objects before an object is excluded.
- Support auto-include suggestions for required dependencies.
- Distinguish user-selected changes from auto-suggested dependencies.
- Block script generation by default when critical dependencies are missing.
- Record non-critical dependency overrides in generated artifacts.

DbState must not silently generate a broken synchronization script.

## Examples

- A selected view depends on an excluded table or column.
- A selected function depends on an excluded table, type, enum, function, or extension.
- A selected foreign key depends on an excluded referenced table or unique key.
- A selected index depends on an excluded table or column.
- A selected trigger depends on an excluded trigger function.
- A selected reference-data row depends on an excluded parent reference-data row.
- A selected grant depends on an excluded role or object.
- A selected materialized view depends on excluded base objects.
- A selected enum-dependent table change depends on an excluded enum change.

## UI states

DbState should provide dependency-aware states such as included, excluded, required dependency, missing dependency, dependent object impacted, safe to include, warning, blocked unless dependencies are included, and explicit override required.

## Artifacts

Dependency warnings must appear in the SQL script header or comments, summary markdown, risk JSON, and AI-reviewable artifacts.

Codex and Claude may review dependency warnings, but must not override them or execute scripts.
