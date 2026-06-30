# Slice 18 Golden-Path Private Beta Walkthrough

This walkthrough is the repeatable private beta path for DbState PostgreSQL v0.1. It uses a disposable ParkingDemo PostgreSQL database and a fresh local Git repository at `C:\DbState\ParkingDemo`.

DbState remains local-first and safety-first:

- DbState does not execute SQL.
- DbState does not apply generated SQL to a database.
- DbState does not mutate PostgreSQL.
- DbState does not stage, commit, push, pull, fetch, or tag Git changes.
- Release artifacts are review-only.

Do not use production, UAT, staging, or shared databases for this walkthrough.

## 1. Prerequisites

Install or have available:

- Windows PowerShell.
- Git.
- Docker Desktop.
- A browser.
- Rust toolchain if using the native developer path.

Repository paths used in this walkthrough:

```text
C:\SourceCodes\DbState
C:\DbState\ParkingDemo
```

Disposable PostgreSQL target:

```text
Container: dbstate-parking-demo-pg
Database: dbstate_parking_demo
Port: 55417
Password: dbstate_test_only
Connection URL: postgres://postgres:dbstate_test_only@localhost:55417/dbstate_parking_demo
```

The password and URL above are for local disposable testing only.

## 2. Build DbState

Native developer path:

```powershell
cd C:\SourceCodes\DbState
cargo build
```

Expected result:

- `C:\SourceCodes\DbState\target\debug\dbstate.exe` exists.

Docker path:

```powershell
cd C:\SourceCodes\DbState
docker build -t dbstate-postgres:dev .
```

Expected result:

- Docker image `dbstate-postgres:dev` builds successfully.

## 3. Start Disposable PostgreSQL

```powershell
docker rm -f dbstate-parking-demo-pg 2>$null

docker run --name dbstate-parking-demo-pg `
  -e POSTGRES_PASSWORD=dbstate_test_only `
  -e POSTGRES_DB=dbstate_parking_demo `
  -p 55417:5432 `
  -d postgres:16
```

Wait for readiness:

```powershell
do {
  Start-Sleep -Seconds 1
  docker exec dbstate-parking-demo-pg pg_isready -U postgres -d dbstate_parking_demo
} until ($LASTEXITCODE -eq 0)
```

Expected result:

- `pg_isready` reports that PostgreSQL accepts connections.

## 4. Load ParkingDemo Sample Schema

The sample schema lives at:

```text
C:\SourceCodes\DbState\docs\postgresql-v0.1\samples\parking-demo.sql
```

Load it into the disposable database:

```powershell
cd C:\SourceCodes\DbState
Get-Content docs\postgresql-v0.1\samples\parking-demo.sql |
  docker exec -i dbstate-parking-demo-pg psql -U postgres -d dbstate_parking_demo
```

The sample includes:

- `pgcrypto` extension.
- `parking` schema.
- `parking.parking_session_status` enum.
- `parking.ticket_number_seq` sequence.
- `parking.lots`, `parking.vehicles`, and `parking.parking_sessions` tables.
- Primary key, foreign key, unique, and check constraints that PostgreSQL naturally creates.
- Non-constraint-backed indexes.
- `parking.open_parking_sessions` view.

Durable first-class constraint coverage remains deferred in v0.1. Constraint context may appear in Object Diff review where available.

## 5. Create Fresh ParkingDemo Git Repository

```powershell
New-Item -ItemType Directory -Force C:\DbState\ParkingDemo | Out-Null
cd C:\DbState\ParkingDemo
git init
git branch -M dev
git status --short --branch --untracked-files=all
```

Expected result:

- The repository is on branch `dev`.
- There is no `database/` directory yet.

## 6. Start DbState Service

Native service path:

```powershell
cd C:\SourceCodes\DbState
$env:DBSTATE_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:55417/dbstate_parking_demo"
cargo run -- serve --host 127.0.0.1 --port 4587
```

Expected startup result:

- Service URL is `http://127.0.0.1:4587/`.
- Default binding is local-only.
- Startup text says DbState does not execute generated SQL or apply database changes.

If port `4587` is busy, stop the old service or use another port:

```powershell
cargo run -- serve --host 127.0.0.1 --port 4588
```

## 7. Open Browser UI

Open:

```text
http://127.0.0.1:4587/
```

If the UI looks stale, press `Ctrl+F5`.

Expected result:

- DbState PostgreSQL v0.1 UI loads.
- Safety banner states local-only, no direct database apply, and no SQL execution.

## 8. Select Workspace With Browse

Workspace page:

1. Click `Browse`.
2. Navigate to `C:\DbState\ParkingDemo`.
3. Click `Select this folder`.
4. Click `Check Workspace`.

Expected result:

- Workspace is a Git repository.
- DbState project status indicates missing DbState structure.
- Missing paths are listed.

## 9. Initialize DbState Project From UI

Workspace page:

1. Click `Init Plan`.
2. Review planned creates.
3. Type:

```text
INITIALIZE DBSTATE PROJECT
```

4. Click `Initialize DbState Project`.
5. Click `Check Workspace` if the status does not refresh automatically.

Expected result:

- `database/` structure is created.
- Missing paths becomes `0`.
- DbState project status becomes complete.
- No PostgreSQL connection is needed for initialization.
- No SQL is executed.
- No Git add, commit, push, pull, or fetch happens.

## 10. Commit Initialized Structure Manually

DbState does not stage or commit changes. The tester does this manually:

```powershell
cd C:\DbState\ParkingDemo
git status --short --branch --untracked-files=all
git add database
git commit -m "chore: initialize DbState project structure"
```

Expected result:

- Git records the initial DbState project structure.
- Working tree is clean.

## 11. Configure Connection

Recommended for this walkthrough: use the service environment variable already set in the service process:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:55417/dbstate_parking_demo"
```

UI Source & Target page:

1. Set Connection Mode to `Use service environment variable`.
2. Do not paste the URL into the UI unless testing session URL mode.

Optional profile mode:

- Save only host, port, database, username, SSL mode, and description.
- Do not save passwords or full PostgreSQL URLs.
- Use a session-only password when testing the profile connection.

## 12. Run Inspect

Source & Target page:

1. Set Workflow Mode to `PostgreSQL Inspect Only`.
2. Confirm Source is PostgreSQL database.
3. Confirm Target is read-only catalog view.

Compare Options page:

1. Scope: `All`.
2. Click `Inspect`.

Expected Results page:

- Object type filters include:
  - Extension
  - Enum
  - Sequence
  - Index
  - View
- Rows include supported ParkingDemo objects if present.
- Columns stay in Object Diff details for selected tables, not as top-level Results filter rows.

## 13. Run Database To Repository Preview

Source & Target page:

1. Set Workflow Mode to `Database to Repository`.
2. Confirm Source is PostgreSQL database.
3. Confirm Target is repository desired state.

Compare Options page:

1. Scope: `All`.
2. Click `Preview Repository Sync`.

Expected result:

- Preview shows added, changed, unchanged, or skipped files.
- No repository files are written during preview.
- No PostgreSQL mutation occurs.

## 14. Write Repository Files

Compare Options page:

1. Review the preview first.
2. Confirm working tree is clean.
3. Type:

```text
WRITE REPOSITORY FILES
```

4. Click `Write Repository Files`.

Expected result:

- Supported desired-state files are written under `database/objects/`.
- Nothing is written outside the selected workspace.
- PostgreSQL is not mutated.
- No SQL is executed.
- Git does not stage or commit files.

## 15. Commit Captured Desired-State Files Manually

```powershell
cd C:\DbState\ParkingDemo
git status --short --branch --untracked-files=all
git add database\objects
git commit -m "feat: capture ParkingDemo database state"
```

Expected result:

- Captured desired-state files are committed manually.
- Working tree is clean.

## 16. Run Repository To Database Compare

UI:

1. Source & Target page: set Workflow Mode to `Repository to Database Compare`.
2. Confirm Source is repository desired state.
3. Confirm Target is PostgreSQL database.
4. Compare Options page: Scope `All`.
5. Click `Run Compare`.

Expected result:

- Results grid shows object rows by type.
- Most freshly captured objects should be `inSync`.
- Any skipped or deferred objects are visible as review items.

## 17. Review Results Grid

In Results:

1. Use object type filter.
2. Try Schema, Table, Extension, Enum, Sequence, Index, and View.
3. Review status badges and legend.
4. Select a table row.

Expected result:

- Source and Target context appears above the grid.
- Source and Target are not table columns.
- Service errors do not appear as fake object rows.

## 18. Review Object Diff

Object Diff page:

1. Confirm selected object summary.
2. Confirm Source and Target type indicators.
3. Confirm `Full Context DDL` is selected by default.
4. Switch to `Object Only DDL`.
5. Switch to `Related Objects`.
6. Switch to `Raw Details`.

Expected result:

- Full Context DDL shows the selected table plus related index DDL where available.
- Object Only DDL shows only the durable object.
- Related Objects groups related details.
- Raw Details shows redacted JSON.
- DDL comparison shows Similar, Different, or DDL unavailable.

## 19. Run Release Dry-Run From CLI

```powershell
cd C:\DbState\ParkingDemo

$env:DBSTATE_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:55417/dbstate_parking_demo"

C:\SourceCodes\DbState\target\debug\dbstate.exe release postgres --all --name beta_review --dry-run --format json
```

Expected result:

- Command returns JSON.
- Planned artifact paths are shown.
- No files are written under `database/releases/`.
- Risk information is visible.

## 20. Run Release Write From CLI

```powershell
cd C:\DbState\ParkingDemo

C:\SourceCodes\DbState\target\debug\dbstate.exe release postgres --all --name beta_review
```

Expected result:

- Release artifacts are written under `database/releases/`.
- DbState does not execute release SQL.
- DbState does not apply changes to PostgreSQL.
- DbState does not stage or commit artifacts.

## 21. Review Generated Artifacts

```powershell
Get-ChildItem database\releases
Get-Content database\releases\*beta_review.summary.md
Get-Content database\releases\*beta_review.risk.json
Get-Content database\releases\*beta_review.manifest.json
```

Review the SQL artifact:

```powershell
Get-Content database\releases\*beta_review.sql
```

Safety search:

```powershell
Select-String -Path database\releases\*.sql `
  -Pattern "DROP TABLE|DROP SCHEMA|DROP COLUMN|ALTER TABLE DROP|TRUNCATE|DELETE FROM|UPDATE |MERGE|GRANT|REVOKE|ALTER OWNER|EXECUTE" `
  -CaseSensitive:$false
```

Expected result:

- SQL artifact has DbState release header.
- SQL artifact has review sections.
- Summary contains reviewer checklist.
- Risk JSON has safety flags set to `false`.
- Manifest lists generated artifacts.
- No unsafe executable SQL is generated.

## 22. Commit Release Artifacts Manually

DbState does not stage or commit release artifacts. If the artifacts are acceptable, commit manually:

```powershell
cd C:\DbState\ParkingDemo
git status --short --branch --untracked-files=all
git add database\releases
git commit -m "chore: add beta review release artifacts"
```

## 23. Reports And Raw JSON

UI Reports / Raw JSON page:

1. Click `Copy JSON`.
2. Paste into a scratch file.
3. Confirm output is redacted.

Expected result:

- No raw PostgreSQL URL.
- No password.
- No token.
- No secret fields.

## 24. Docker Walkthrough

Use Docker if the tester does not want to install Rust locally.

Build image:

```powershell
cd C:\SourceCodes\DbState
docker build -t dbstate-postgres:dev .
```

Run service in Docker:

```powershell
docker run --rm `
  -p 127.0.0.1:4587:4587 `
  -v "C:\DbState\ParkingDemo:/workspace" `
  -w /workspace `
  -e DBSTATE_POSTGRES_URL="postgres://postgres:dbstate_test_only@host.docker.internal:55417/dbstate_parking_demo" `
  dbstate-postgres:dev `
  dbstate serve --host 0.0.0.0 --port 4587
```

Open:

```text
http://127.0.0.1:4587/
```

Use this workspace path in the UI:

```text
/workspace
```

Docker can browse only paths mounted into the container. It cannot browse arbitrary host paths.

## 25. Cleanup

Stop service with `Ctrl+C`.

Remove the disposable PostgreSQL container:

```powershell
docker rm -f dbstate-parking-demo-pg
```

Remove the fresh workspace only if you no longer need it:

```powershell
Remove-Item -LiteralPath C:\DbState\ParkingDemo -Recurse -Force
```

Do not remove a workspace that contains feedback artifacts or changes you still need.

## 26. Troubleshooting

### Port 4587 Already In Use

Stop the existing service or use another port:

```powershell
cargo run -- serve --host 127.0.0.1 --port 4588
```

Open:

```text
http://127.0.0.1:4588/
```

### dbstate.exe Locked By Running Service

Stop the service terminal with `Ctrl+C`, then rebuild.

### Docker Desktop Not Running

Start Docker Desktop and rerun:

```powershell
docker ps
```

### PostgreSQL Container Not Ready

Wait:

```powershell
docker exec dbstate-parking-demo-pg pg_isready -U postgres -d dbstate_parking_demo
```

### Cannot Connect To Database

Native service should use:

```text
postgres://postgres:dbstate_test_only@localhost:55417/dbstate_parking_demo
```

Docker service should use:

```text
postgres://postgres:dbstate_test_only@host.docker.internal:55417/dbstate_parking_demo
```

### UI Shows Failed To Fetch

- Confirm the service terminal is still running.
- Confirm the URL and port.
- Refresh the browser.

### DBSTATE_POSTGRES_URL Not Set

Set it in the same terminal before starting the service:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:55417/dbstate_parking_demo"
cargo run -- serve --host 127.0.0.1 --port 4587
```

### Workspace Is Not A Git Repo

Run:

```powershell
cd C:\DbState\ParkingDemo
git init
git branch -M dev
```

### Workspace Is Git Repo But Not DbState Project

Use the Workspace page:

1. Type `INITIALIZE DBSTATE PROJECT`.
2. Click `Initialize DbState Project`.
3. Commit the generated structure manually.

### Working Tree Dirty Blocks Write

Review changes:

```powershell
git status --short --branch --untracked-files=all
```

Commit or stash manually before running write actions.

### Release Write Blocked

Check:

- Working tree is clean.
- DbState project structure is complete.
- Selected plan has no blocked items.
- Release name is valid.

### Browser Cache Stale

Press `Ctrl+F5`.

### Docker Path Mapping Confusion

Host path:

```text
C:\DbState\ParkingDemo
```

Container path:

```text
/workspace
```

Use `/workspace` in the UI when running the service in Docker.

## 27. Private Beta Readiness Checklist

- Can run without installing Rust using Docker.
- Can run natively with cargo for developers.
- Can initialize a fresh Git repository from the UI.
- Can connect with service environment variable, session URL, or non-secret profile.
- Can inspect PostgreSQL.
- Can capture database state to repository files.
- Can compare repository desired state to PostgreSQL.
- Can review Results grid.
- Can review Object Diff Full Context DDL.
- Can review Object Only DDL.
- Can review Related Objects.
- Can review Raw Details.
- Can generate release artifacts.
- Can review SQL, summary markdown, risk JSON, and manifest JSON.
- No direct apply exists.
- No SQL execution exists.
- No PostgreSQL mutation exists.
- No Git auto stage, commit, or push exists.
- Testers have feedback template.
- Known limitations are documented.

## 28. Feedback

Use:

```text
docs/postgresql-v0.1/49-private-beta-feedback-template.md
```

Before sending feedback, redact passwords, full PostgreSQL URLs, production data, tokens, certificates, and secrets.
