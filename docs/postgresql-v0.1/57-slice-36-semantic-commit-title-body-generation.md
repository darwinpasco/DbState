# Slice 36: Semantic Commit Title and Body Generation

## Purpose

Slice 36 productizes deterministic commit-message generation for Private Beta 2.
DbState generates a suggested Git commit title and body from the files currently staged in the local Git index.

The default scope is staged changes only because the message should describe exactly what the next Git commit will include.

## CLI

```text
dbstate commit-message
dbstate commit-message --style conventional
dbstate commit-message --style plain
dbstate commit-message --intent "Support customer email verification"
dbstate commit-message --json
```

`--style conventional` is the default. `--style plain` emits a non-Conventional-Commit title while keeping a technical body.

`--intent` may improve the human-facing title and summary line. It does not remove or falsify the technical change list generated from staged DbState files.

## Recognized Paths

DbState semantically analyzes staged files under:

- `database/objects/`
- `database/reference-data/`
- `database/releases/objects/`
- `database/releases/reference-data/`

Staged files outside these paths are reported as ignored non-DbState files. They are not semantically interpreted.

## Output

Text output includes:

- Suggested Commit Title
- Suggested Commit Body
- Analyzed DbState files
- Ignored non-DbState staged files, when present
- Warnings
- Breaking change warning, when detected

JSON output includes `command`, `success`, `scope`, `style`, `branch`, `ticket`, `title`, `body`, `message`, `breakingChange`, `stagedFiles`, `analyzedFiles`, `ignoredFiles`, `warnings`, and `errors`.

## Git Workflow UI

The Git Workflow page uses the same staged-change commit-message engine.

The page clearly states:

```text
Commit message is generated from staged changes.
```

If no files are staged, DbState shows:

```text
No staged changes found. Stage reviewed DbState paths manually, then regenerate.
```

DbState never stages files from the UI.

## Safety

Commit-message generation is local, deterministic, and offline.

It does not:

- stage or unstage files
- create or amend commits
- switch branches
- push, pull, fetch, or tag
- connect to PostgreSQL
- execute SQL
- execute generated artifacts
- mutate PostgreSQL
- send database definitions to a hosted AI service
- persist secrets or connection URLs

Generated messages are advisory and editable. Developers must still review the staged diff before committing.
