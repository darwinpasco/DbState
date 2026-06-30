# Slice 13 Workspace Selection

Slice 13 allows the local DbState Service API and browser UI to work against an explicitly selected local DbState project repository path.

This makes the local UI workflow usable across different local repositories without adding a project database, recent-project list, remote repository access, or write workflows.

The CLI remains unchanged. CLI commands still operate against the current working directory.

## Purpose

Before Slice 13, service and UI workflows used only the service process working directory.

Slice 13 adds session-only workspace selection for service requests:

```json
{
  "repositoryPath": "C:\\SourceCodes\\DbState"
}
```

If `repositoryPath` is omitted or empty, the service uses its process working directory.

## Service Request Behavior

These endpoints accept optional `repositoryPath`:

```text
POST /api/v1/repo/status
POST /api/v1/init/plan
POST /api/v1/init/write
POST /api/v1/postgres/inspect
POST /api/v1/postgres/compare
POST /api/v1/postgres/plan
POST /api/v1/postgres/data-compare
```

The service validates and canonicalizes a provided path before running the existing engine operation.

Example:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/repo/status `
  -ContentType "application/json" `
  -Body '{ "repositoryPath": "C:\\SourceCodes\\DbState" }'
```

Invalid path example:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/repo/status `
  -ContentType "application/json" `
  -Body '{ "repositoryPath": "https://example.com/repo.git" }'
```

## Validation Rules

`repositoryPath` must:

- Be a local filesystem path.
- Exist.
- Point to a directory.
- Be canonicalizable by the service process.
- Be inside or equal to a local Git working tree.

The service rejects:

- Nonexistent paths.
- File paths.
- Non-Git directories.
- URL-like values such as `http://`, `https://`, `ssh://`, `postgres://`, or `postgresql://`.
- Remote Git references such as `git@example.com:repo.git`.
- Null-byte path input.

The service does not:

- Clone repositories.
- Fetch repositories.
- Pull, push, stage, or commit.
- Persist paths.
- Maintain recent projects.
- Write a project database or workspace database.

## Response Behavior

Service responses continue to include repository context where relevant:

```json
{
  "repositoryPath": "D:/SourceCodes/DbState",
  "gitRoot": "D:/SourceCodes/DbState",
  "isGitRepository": true,
  "branch": "dev",
  "workingTreeStatus": "clean",
  "isDirty": false
}
```

If a provided path is invalid, the service returns HTTP `400` with a JSON error response.

If the selected path is a Git repository without DbState structure:

- `repo/status` returns HTTP `200` with missing paths and project status.
- Project operations that require DbState structure can return HTTP `409`.

## Browser UI

The UI includes a Workspace panel with:

- Local repository path input.
- Browse button backed by the local service.
- Workspace status button.
- Repository path display.
- Git root display.
- Branch display.
- Working tree status.
- DbState project status.
- Missing path count.
- Init Plan action.
- Initialize DbState Project action with typed confirmation.

The workspace path is session-only:

- It is not stored in local storage.
- It is not stored in session storage.
- It is not placed in the browser URL query string.
- It is not logged by the UI.
- It is sent only in service request bodies when present.

If the field is empty, the UI uses the service working directory.

Slice 15 adds service-backed browsing to this panel. The picker lists service-visible directories only, never files, and does not use browser filesystem APIs. Selecting a folder fills the session-only path field and validates the workspace.

The Workspace page can initialize a selected Git repository as a DbState project through `POST /api/v1/init/write`.

The write request requires:

```json
{
  "repositoryPath": "C:\\SourceCodes\\DbState",
  "confirmInitializeProject": true,
  "confirmationText": "INITIALIZE DBSTATE PROJECT"
}
```

Initialization uses the selected repository's Git root as the project root. It creates only missing standard DbState project folders/files and does not overwrite the existing reference-data registry.

Initialization does not:

- Capture database objects.
- Connect to PostgreSQL.
- Execute SQL.
- Mutate PostgreSQL.
- Stage, commit, push, pull, fetch, or tag Git changes.

## Docker Path Mapping

When running DbState in Docker, `repositoryPath` must refer to a path inside the container.

Example:

```powershell
docker run --rm `
  -p 127.0.0.1:4587:4587 `
  -v "C:\SourceCodes\DbState:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate serve --host 0.0.0.0 --port 4587
```

In the UI, use:

```text
/workspace
```

Additional host repositories must be mounted explicitly. The service cannot access arbitrary host paths from inside the container.

The Browse picker follows the same rule. Inside Docker it can browse only mounted container paths, such as `/workspace`.

## Safety Boundary

Workspace selection itself does not persist paths or add database write workflows.

The UI and service still expose only safe read-only, dry-run/plan-only, or explicitly confirmed local repository initialization workflows:

- Health
- Repo status
- Init plan
- Initialize DbState Project
- PostgreSQL inspect
- PostgreSQL compare
- PostgreSQL plan
- Reference-data compare

No direct database apply exists.

No generated SQL execution exists.

No database mutation behavior is added.

Do not expose the local service publicly.

## Current Limitations

- No project database.
- No workspace database.
- No recent projects list.
- No persistent workspace registry.
- No connection profile persistence.
- No native folder picker.
- No browser filesystem access API.
- No authentication or user model.
- No database write workflows in the UI.
- No export, sync, or release write workflows in the UI.
- No remote repository access.
- No Docker Compose or CI workflow.
- No MCP server or AI integration.
- CLI `--repo <path>` remains a future open decision.
