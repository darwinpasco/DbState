# Slice 15 Database To Repository Workflow

Slice 15 adds the safe reverse workflow for users who start with an existing PostgreSQL database and want to capture the current supported database state into a selected local DbState repository.

Preferred product terms:

- Database to Repository.
- Import from Database.
- Sync Repository from Database.
- Capture Current Database State.

The UI and docs avoid "reverse compare" because the workflow is source database to target repository.

## Safety Boundary

Database to Repository may write local files only after explicit confirmation.

Allowed write location:

```text
<selected workspace>/database/objects/
```

The workflow must not:

- Mutate PostgreSQL.
- Execute generated SQL.
- Apply SQL to a database.
- Write outside the selected workspace.
- Write secrets.
- Stage, commit, push, pull, fetch, or tag Git changes.
- Hide local file changes from the user.

PostgreSQL access remains read-only. The service uses the existing PostgreSQL inspection and repository sync engine that renders supported desired-state files. Slice 16 expands that set from schemas and ordinary tables to include extensions, enums, sequences, non-constraint-backed indexes, and views.

## Service Endpoints

Slice 15 adds two local Service API endpoints:

```text
POST /api/v1/postgres/repository-sync/preview
POST /api/v1/postgres/repository-sync/write
```

Slice 15 also adds service-backed workspace browsing endpoints:

```text
GET  /api/v1/workspace/roots
POST /api/v1/workspace/list-directories
POST /api/v1/workspace/validate
```

The directory browsing endpoints are local-only helpers for the browser UI. They list service-visible directories only, do not list files, do not recurse deeply, do not read file contents, and do not create or modify anything. They reject URL-like paths and null-byte input. Workspace validation reuses the existing Git and DbState project checks.

Both endpoints accept the same basic scope model as existing PostgreSQL schema workflows:

```json
{ "scope": "all" }
```

```json
{ "scope": "schema", "schema": "core" }
```

```json
{ "scope": "table", "table": "core.parking_sessions" }
```

Both endpoints accept optional `repositoryPath` for local workspace selection and the existing connection inputs:

1. Request `postgresUrl`.
2. Request `connection.profileName` plus optional session-only password.
3. Service process `DBSTATE_POSTGRES_URL`.

Connection URLs and passwords are session-only and are not returned in responses.

## Preview Behavior

Preview:

- Reads PostgreSQL read-only.
- Reads the selected repository.
- Requires a valid scope.
- Does not require a clean working tree.
- Does not write files.
- Returns planned added, changed, unchanged, and skipped files where available.
- Returns repository context including branch and working tree status.

If the selected workspace is a Git repository but not initialized as a DbState project, preview returns clear guidance to run `dbstate init` first.

## Write Behavior

Write:

- Requires a valid DbState project.
- Requires a clean working tree.
- Requires explicit confirmation.
- Writes only under `database/objects/`.
- Returns created, updated, skipped, and warning information where available.

The confirmation request must include:

```json
{
  "scope": "all",
  "confirmRepositoryWrite": true,
  "confirmationText": "WRITE REPOSITORY FILES"
}
```

If confirmation is missing or incorrect, no files are written.

If the working tree is dirty, no files are written. The user should review, commit, or stash local changes before retrying.

## Browser UI

The Source & Target step includes Workflow Mode:

- Repository to Database Compare.
- PostgreSQL Inspect Only.
- Reference Data Compare.
- Database to Repository.

When Database to Repository is selected:

- Source is PostgreSQL database.
- Target is repository desired state.
- The connection mode selector still applies.
- The workspace selector still applies.
- Compare Options shows Preview Repository Sync and Write Repository Files.

The Source & Target page places PostgreSQL connection controls on the Source side for this workflow. The Target side shows the selected repository workspace, Git root, branch, working tree status, DbState project status, and clean or dirty state.

Workflow Mode appears above the Source and Target panels so users choose direction before reviewing the sides. Repository to Database Compare shows repository as Source and PostgreSQL as Target. PostgreSQL Inspect Only shows PostgreSQL as Source and a read-only catalog view as Target. Reference Data Compare shows repository configured reference data as Source and PostgreSQL as Target.

The Workspace step includes a Browse button. Browse opens a service-backed directory picker with roots, parent navigation, refresh, and Select this folder. The selected folder is copied into the session-only workspace path field and validated. Manual path entry remains available as an advanced fallback.

Write Repository Files is disabled until the user types:

```text
WRITE REPOSITORY FILES
```

The UI does not expose export write, sync-to-database, release write, direct database apply, generated SQL execution, or arbitrary SQL execution.

## Results Grid

Database to Repository responses are mapped into the existing Results grid.

Supported statuses include:

- `added`
- `changed`
- `unchanged`
- `skipped`
- `plannedCreate`
- `plannedUpdate`
- `created`
- `updated`
- `blocked`
- `error`

Source and Target context appears above the grid. Object Diff uses Source and Target labels with type indicators, for example:

- Source type: Database.
- Target type: Repository.

The old labels "Repository Side" and "Database Side" are not used.

## Object Diff DDL

Object Diff shows Source DDL and Target DDL panels for supported objects where detail is available. Slice 16 supports DDL detail for schemas, ordinary tables, extensions, enums, sequences, non-constraint-backed indexes, and views.

Repository DDL is read only from known DbState object files under:

```text
database/objects/
```

The service rejects arbitrary paths, path traversal, non-SQL files, and paths outside the selected repository.

Database DDL uses deterministic PostgreSQL renderers backed by read-only inspection. It does not execute SQL and does not mutate PostgreSQL.

The UI normalizes DDL by trimming whitespace, normalizing line endings, and collapsing repeated blank lines before comparison:

- `Similar` uses success styling when both sides are available and normalized text matches.
- `Different` uses difference styling when both sides are available and normalized text differs.
- `DDL unavailable` uses neutral styling when one or both sides are unavailable.

## Reports And Raw JSON

Reports / Raw JSON includes a Copy JSON button.

The button copies the redacted JSON currently displayed in the UI. The copied text must not include PostgreSQL URLs, passwords, tokens, connection strings, or session secrets.

## Docker Guidance

In Docker service mode, Database to Repository write targets the mounted repository path inside the container.

Example:

```powershell
docker run --rm `
  -p 127.0.0.1:4587:4587 `
  -v "${PWD}:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate serve --host 0.0.0.0 --port 4587
```

Use this workspace path in the UI:

```text
/workspace
```

The container cannot write to arbitrary host paths that were not mounted.

The service-backed directory picker can browse only paths visible inside the container. If the repository is mounted at `/workspace`, select `/workspace` in the UI. The picker does not provide access to host paths that were not mounted.

## Current Limitations

- Only supported PostgreSQL schema and simple ordinary table object files are captured.
- Constraints, functions, triggers, grants, materialized views, and other deferred object types remain out of scope.
- Configured reference-data DML is not generated.
- No SQL is executed by DbState.
- No PostgreSQL mutation exists.
- No Git stage, commit, push, pull, or fetch exists in this workflow.
- No project database, workspace database, recent project list, AI integration, MCP server, Docker Compose, or CI workflow is added.

## Validation

Run:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
docker build -t dbstate-postgres:dev .
```

Optional manual database smoke should use a disposable or approved read-only development PostgreSQL target. Do not use production, UAT, staging, or shared databases without explicit approval.
