# Slice 11 Service API Boundary

Slice 11 introduces a minimal local DbState Service API boundary for future browser UI and tool integration.

The service is a thin HTTP JSON wrapper around existing safe DbState engine operations. It is not a browser UI, not a hosted service, not a multi-user server, and not a deployment engine.

The CLI remains supported.

## Start The Local Service

Default local startup:

```powershell
cargo run -- serve
```

Explicit host and port:

```powershell
cargo run -- serve --host 127.0.0.1 --port 4587
```

Defaults:

- Host: `127.0.0.1`
- Port: `4587`

Startup output includes the local URL and the safety note that DbState does not execute generated SQL or apply database changes.

Do not expose the Slice 11 service publicly. Authentication, users, TLS, remote deployment, and public server hardening are not included.

## Endpoint List

Slice 11 exposes JSON-only endpoints:

```text
GET  /health
GET  /api/v1/health
POST /api/v1/repo/status
POST /api/v1/init/plan
POST /api/v1/postgres/inspect
POST /api/v1/postgres/compare
POST /api/v1/postgres/plan
POST /api/v1/postgres/data-compare
GET  /api/v1/connections/profiles
POST /api/v1/connections/profiles
PUT  /api/v1/connections/profiles/{name}
DELETE /api/v1/connections/profiles/{name}
POST /api/v1/connections/test
```

No write endpoints are exposed in Slice 11.

Not exposed:

- Init write
- Export write
- Sync write
- Release artifact write
- Direct database apply
- Generated SQL execution
- Database mutation

## Request Model

Requests use JSON bodies for `POST` endpoints.

Slice 11 originally used the service process working directory only. Slice 13 adds an optional session-only `repositoryPath` field for safe service endpoints:

```json
{
  "repositoryPath": "D:/work/example",
  "scope": "all"
}
```

If `repositoryPath` is omitted or empty, the service uses its current working directory. If it is provided, it must be a local directory that exists and is inside a Git working tree. DbState canonicalizes it before use, does not persist it, does not keep a recent-project list, and does not clone, fetch, pull, push, stage, or commit repositories from service endpoints.

URL-like values such as `https://example.com/repo.git`, `ssh://...`, `git@...`, or `postgres://...` are rejected as repository paths.

PostgreSQL endpoints may use `postgresUrl` for the current request only:

```json
{
  "postgresUrl": "postgres://user:password@localhost:5432/disposable_test_db",
  "scope": "all"
}
```

The URL is session-only:

- It is not persisted.
- It is not written to repository files.
- It is not returned in responses.
- It is not logged by the service.

If `postgresUrl` is omitted, the endpoint uses `DBSTATE_POSTGRES_URL` if it is set.

Slice 14 also allows request-level non-secret profile selection:

```json
{
  "connection": {
    "profileName": "exitpass-local",
    "password": "session-only-password"
  },
  "scope": "all"
}
```

Connection resolution order:

1. Request `postgresUrl`.
2. Request `connection.profileName` plus optional session-only `connection.password`.
3. Service process `DBSTATE_POSTGRES_URL`.

Profiles store only non-secret metadata: name, host, port, database, username, SSL mode, optional description, and optional default schema. Profiles never store passwords, tokens, full URLs, or connection strings.

Preferred local usage:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://user:password@localhost:5432/disposable_test_db"
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/postgres/inspect `
  -ContentType "application/json" `
  -Body '{ "scope": "all" }'
```

Do not use production, UAT, staging, or shared databases for service examples or tests.

## Scope Mapping

Supported scope values:

```json
{ "scope": "all" }
```

```json
{ "scope": "schema", "schema": "dbstate_slice2" }
```

```json
{ "scope": "table", "table": "dbstate_slice2.sample_accounts" }
```

`data-compare` supports only:

```json
{ "scope": "all" }
```

```json
{ "scope": "table", "table": "dbstate_ref.payment_methods" }
```

`schema` scope is rejected for `data-compare`.

Plan endpoint include and exclude examples:

```json
{
  "scope": "all",
  "include": ["table:dbstate_slice2.sample_accounts"],
  "exclude": ["schema:public"]
}
```

Invalid scope returns HTTP `400` with a JSON error response.

## Response Model

Service responses follow the Slice 9 JSON contract style where practical.

Common fields:

```json
{
  "command": "repo status",
  "success": true,
  "warnings": [],
  "errors": []
}
```

Repository-bound responses include repository context when available:

```json
{
  "repositoryPath": "D:/work/example",
  "gitRoot": "D:/work/example",
  "isGitRepository": true,
  "branch": "main",
  "workingTreeStatus": "clean",
  "isDirty": false
}
```

When `repositoryPath` is supplied, responses report the normalized local repository context for that selected workspace. The repo status endpoint can report Git status even when the DbState project structure is incomplete. Project operations return a clear project-structure error when the selected Git repository has not been initialized with `dbstate init`.

PostgreSQL responses include:

```json
{
  "databaseType": "postgresql"
}
```

Responses must not include:

- Raw PostgreSQL connection URLs
- Passwords
- Tokens
- Credentials
- Raw masked reference-data values
- Full row dumps

## Endpoint Examples

Health:

```powershell
Invoke-RestMethod http://127.0.0.1:4587/health
Invoke-RestMethod http://127.0.0.1:4587/api/v1/health
```

Repository status:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/repo/status `
  -ContentType "application/json" `
  -Body "{}"
```

Init plan only:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/init/plan `
  -ContentType "application/json" `
  -Body '{ "dryRun": true }'
```

PostgreSQL inspect:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/postgres/inspect `
  -ContentType "application/json" `
  -Body '{ "scope": "all" }'
```

Compare:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/postgres/compare `
  -ContentType "application/json" `
  -Body '{ "scope": "all" }'
```

Plan:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/postgres/plan `
  -ContentType "application/json" `
  -Body '{ "scope": "all", "include": [], "exclude": [] }'
```

Reference-data compare:

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/postgres/data-compare `
  -ContentType "application/json" `
  -Body '{ "scope": "all" }'
```

Connection profiles:

```powershell
Invoke-RestMethod `
  -Method Get `
  -Uri http://127.0.0.1:4587/api/v1/connections/profiles
```

```powershell
Invoke-RestMethod `
  -Method Post `
  -Uri http://127.0.0.1:4587/api/v1/connections/test `
  -ContentType "application/json" `
  -Body '{ "connection": { "profileName": "exitpass-local", "password": "session-only-password" } }'
```

## HTTP Status Behavior

Expected status codes:

- `200` for successful endpoint execution, even if compare differences exist.
- `400` for invalid request bodies, invalid scopes, unsupported write requests, or missing required request values.
- `404` for unknown routes.
- `409` for repository state conflicts such as missing DbState project structure.
- `500` for unexpected service errors.
- `503` for PostgreSQL connection failures.

Plan reports with blocked items may still return `200` if a plan report was produced.

## Docker Service Mode

Native service mode binds to `127.0.0.1` by default.

When running inside Docker, bind the service to `0.0.0.0` inside the container and publish the host port to `127.0.0.1`:

```powershell
docker run --rm `
  -p 127.0.0.1:4587:4587 `
  -v "${PWD}:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate serve --host 0.0.0.0 --port 4587
```

Host smoke check:

```powershell
Invoke-RestMethod http://127.0.0.1:4587/health
```

This is local development usage only. Do not publish the service on a public interface.

## Tests

Normal validation:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Optional gated PostgreSQL integration tests:

```powershell
$env:DBSTATE_TEST_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:5432/disposable_test_db"
cargo test --test postgres_integration
```

Only use disposable test databases. Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database.

## Current Limitations

- No browser UI is included.
- No authentication or user model is included.
- No write endpoints are included.
- No project database or workspace database is included.
- No long-running job queue, background scheduler, or file watcher is included.
- Connection profiles are local non-secret metadata only. Password persistence, keychain integration, vault integration, cloud sync, and team-shared profiles are not included.
- No Docker Compose or CI workflow is included.
- No MCP server or AI integration is included.
- No SQL execution or database apply exists.
- PostgreSQL object coverage remains limited to existing v0.1 slices.
- `--repo <path>` and service-side project selection remain open decisions. Current behavior uses the service process working directory.
