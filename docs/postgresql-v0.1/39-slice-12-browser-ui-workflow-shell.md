# Slice 12 Browser UI Workflow Shell

Slice 12 adds a minimal local browser UI shell on top of the existing DbState Service API boundary.

The UI proves the product workflow direction without becoming the engine. It calls existing Slice 11 service endpoints and does not add database capability, write workflows, SQL execution, direct apply behavior, or database mutation behavior.

The CLI and Service API remain supported.

## Start The Local Service

From the repository root:

```powershell
cargo run -- serve --host 127.0.0.1 --port 4587
```

Defaults:

- Host: `127.0.0.1`
- Port: `4587`

The default binding is local-only. Do not expose the service publicly.

## Open The UI

Open:

```text
http://127.0.0.1:4587/
```

The same UI is available at:

```text
http://127.0.0.1:4587/ui
http://127.0.0.1:4587/ui/
```

Static assets:

```text
http://127.0.0.1:4587/ui/app.css
http://127.0.0.1:4587/ui/app.js
```

The UI is embedded in the Rust binary. There is no frontend build pipeline.

## UI Panels

The Slice 12 shell includes:

- Product title: `DbState PostgreSQL v0.1`
- Local-only banner
- Safety banner for no SQL execution, no direct database apply, and no write workflows
- Service health panel
- Repository status panel
- Init plan panel
- PostgreSQL connection input panel
- PostgreSQL inspect panel
- PostgreSQL compare panel
- PostgreSQL plan panel
- Reference-data compare panel
- Raw JSON response viewer

The UI is intentionally small. It is not the full DbState browser product.

## Service Endpoints Used By The UI

The UI calls only these approved endpoints:

```text
GET  /api/v1/health
POST /api/v1/repo/status
POST /api/v1/init/plan
POST /api/v1/postgres/inspect
POST /api/v1/postgres/compare
POST /api/v1/postgres/plan
POST /api/v1/postgres/data-compare
```

The UI does not call or expose:

- Init write
- Export write
- Sync write
- Release artifact write
- Direct database apply
- Generated SQL execution
- Database mutation
- Arbitrary SQL execution

## PostgreSQL URL Handling

The UI has a session-only PostgreSQL URL input.

Rules:

- The URL is sent only in the JSON request body for the clicked PostgreSQL operation.
- The URL is not stored in local storage.
- The URL is not stored in session storage.
- The URL is not placed in the browser URL query string.
- The URL is not logged by the UI.
- The URL is not echoed into response panels.

For repeatable local testing, prefer setting `DBSTATE_POSTGRES_URL` in the service process environment and leaving the UI field empty:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://user:password@localhost:5432/disposable_test_db"
cargo run -- serve
```

Do not use production, UAT, staging, or shared databases for UI tests.

## Scope Controls

PostgreSQL inspect, compare, and plan support:

- `all`
- `schema`
- `table`

Reference-data compare supports:

- `all`
- `table`

Reference-data compare does not support schema scope.

Plan include and exclude values are comma-separated object refs, for example:

```text
table:dbstate_slice2.sample_accounts,schema:public
```

Empty include and exclude entries are ignored.

## Response Display

The UI displays:

- Success or failure
- HTTP status
- Repository branch where present
- Working tree status where present
- Warning and error counts
- Raw formatted JSON

The UI defensively redacts obvious PostgreSQL URLs, password markers, and token markers before showing raw JSON.

The service also performs redaction. The UI redaction is a presentation safeguard, not a replacement for service-side safety.

## Docker Service Mode

Build the image:

```powershell
docker build -t dbstate-postgres:dev .
```

Run the service with local host port publishing:

```powershell
docker run --rm `
  -p 127.0.0.1:4587:4587 `
  -v "${PWD}:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate serve --host 0.0.0.0 --port 4587
```

Open:

```text
http://127.0.0.1:4587/
```

The container binds to `0.0.0.0` inside the container only so Docker can publish the port. The host port should stay bound to `127.0.0.1`.

## Validation

Normal validation:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Manual UI smoke:

```powershell
Invoke-WebRequest http://127.0.0.1:4587/ -UseBasicParsing
Invoke-WebRequest http://127.0.0.1:4587/ui -UseBasicParsing
Invoke-WebRequest http://127.0.0.1:4587/ui/app.css -UseBasicParsing
Invoke-WebRequest http://127.0.0.1:4587/ui/app.js -UseBasicParsing
Invoke-RestMethod http://127.0.0.1:4587/api/v1/health
```

In the UI, click:

- Health
- Repository status
- Init plan
- Inspect with no PostgreSQL URL and no env var, to confirm a safe error
- Compare with no PostgreSQL URL and no env var, to confirm a safe error
- Data compare with no PostgreSQL URL and no env var, to confirm a safe error

Optional PostgreSQL UI smoke should use only a disposable database.

## Current Limitations

- This is not the full browser UI product.
- No React, Vue, Svelte, Angular, Vite, npm, Node, or package file is included.
- No external CDN, font, or script dependency is used.
- No authentication or user model is included.
- No write workflows are included.
- No export, sync, or release write workflow is exposed.
- No direct database apply exists.
- No generated SQL execution exists.
- No database mutation behavior is added.
- No project database or workspace database is included.
- No file watcher, job queue, or background scheduler is included.
- No Docker Compose or CI workflow is included.
- No MCP server or AI integration is included.
- PostgreSQL object coverage remains limited to existing v0.1 slices.
