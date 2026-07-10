# DbState PostgreSQL v0.1.0 Private Beta 2 Golden-Path Walkthrough

This walkthrough is the repeatable tester path for DbState PostgreSQL v0.1.0 Private Beta 2.

Use your own non-production PostgreSQL database and a fresh local Git repository for the DbState workspace. If you do not already have a suitable non-production database, Pagila is recommended as a safe sample database for testing:

```text
https://github.com/devrimgunduz/pagila
```

Pagila is useful for DbState testing because it contains tables, relationships, indexes, views, functions, and sample data. DbState PostgreSQL v0.1.0 Private Beta 2 intentionally supports only the documented beta object subset, so unsupported objects should appear as deferred or unavailable review context rather than as generated database changes.

DbState remains local-first and safety-first:

- DbState does not execute SQL.
- DbState does not apply generated SQL to a database.
- DbState does not mutate PostgreSQL.
- DbState does not stage, commit, push, pull, fetch, or tag Git changes.
- Release artifacts are review-only.

Do not use production, UAT, staging, shared, regulated, or customer-data databases for this walkthrough.

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
C:\DbState\PrivateBetaDemo
```

PostgreSQL target:

```text
Database: your own non-production PostgreSQL database
Recommended sample if needed: pagila
Connection URL shape: postgres://<user>:<password>@127.0.0.1:<port>/<database>
```

Use a local disposable password or a non-production credential only. Do not put real credentials in feedback, screenshots, logs, or repository files.

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

## 3. Choose A Non-Production PostgreSQL Database

Use a PostgreSQL database that is safe to inspect and compare during a beta test.

Allowed examples:

- A local disposable PostgreSQL database.
- A personal development database.
- A database created only for this beta walkthrough.
- Pagila loaded into a local or disposable PostgreSQL instance.

Do not use:

- production
- UAT
- staging
- shared team databases
- regulated-data databases
- customer-data databases

DbState inspect, compare, preview, and release dry-run paths are read-only against PostgreSQL. The Database to Repository Compare write path writes repository files only. This beta still must not be pointed at sensitive database environments because screenshots, object names, comments, or review artifacts may expose information.

## 4. Optional Pagila Sample Setup

Skip this section if you already have a suitable non-production PostgreSQL database.

Pagila repository:

```text
https://github.com/devrimgunduz/pagila
```

### Local psql

Use this path if `psql` is installed on your machine:

```powershell
cd C:\SourceCodes
git clone https://github.com/devrimgunduz/pagila.git
cd C:\SourceCodes\pagila

psql -h 127.0.0.1 -p <port> -U <user> -d postgres -c "DROP DATABASE IF EXISTS pagila WITH (FORCE);"
psql -h 127.0.0.1 -p <port> -U <user> -d postgres -c "CREATE DATABASE pagila;"
psql -h 127.0.0.1 -p <port> -U <user> -d pagila -v ON_ERROR_STOP=1 -f .\pagila-schema.sql
psql -h 127.0.0.1 -p <port> -U <user> -d pagila -v ON_ERROR_STOP=1 -f .\pagila-data.sql
```

### Docker psql client

Use this path if Docker is available but local `psql` is not installed. The PostgreSQL server can still be local, remote development, or another disposable instance reachable from Docker.

```powershell
cd C:\SourceCodes
git clone https://github.com/devrimgunduz/pagila.git
cd C:\SourceCodes\pagila
$env:PGPASSWORD = Read-Host "PostgreSQL password"

docker run --rm `
  --add-host=host.docker.internal:host-gateway `
  -v "C:\SourceCodes\pagila:/pagila" `
  -e PGPASSWORD=$env:PGPASSWORD `
  postgres:16 `
  psql -h host.docker.internal -p <port> -U <user> -d postgres -c "DROP DATABASE IF EXISTS pagila WITH (FORCE);"

docker run --rm `
  --add-host=host.docker.internal:host-gateway `
  -v "C:\SourceCodes\pagila:/pagila" `
  -e PGPASSWORD=$env:PGPASSWORD `
  postgres:16 `
  psql -h host.docker.internal -p <port> -U <user> -d postgres -c "CREATE DATABASE pagila;"

docker run --rm `
  --add-host=host.docker.internal:host-gateway `
  -v "C:\SourceCodes\pagila:/pagila" `
  -e PGPASSWORD=$env:PGPASSWORD `
  postgres:16 `
  psql -h host.docker.internal -p <port> -U <user> -d pagila -v ON_ERROR_STOP=1 -f /pagila/pagila-schema.sql

docker run --rm `
  --add-host=host.docker.internal:host-gateway `
  -v "C:\SourceCodes\pagila:/pagila" `
  -e PGPASSWORD=$env:PGPASSWORD `
  postgres:16 `
  psql -h host.docker.internal -p <port> -U <user> -d pagila -v ON_ERROR_STOP=1 -f /pagila/pagila-data.sql

Remove-Item Env:\PGPASSWORD
```

Expected result:

- A non-production `pagila` database exists.
- Pagila tables, relationships, indexes, views, functions, and sample data are loaded.

Durable first-class constraint coverage remains deferred in v0.1. Constraint context may appear in Object Diff review where available.

## 5. Create Fresh Private Beta Demo Git Repository

```powershell
New-Item -ItemType Directory -Force C:\DbState\PrivateBetaDemo | Out-Null
cd C:\DbState\PrivateBetaDemo
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
$env:DBSTATE_POSTGRES_URL = "postgres://<user>:<password>@127.0.0.1:<port>/<database>"
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
2. Navigate to `C:\DbState\PrivateBetaDemo`.
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
cd C:\DbState\PrivateBetaDemo
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
$env:DBSTATE_POSTGRES_URL = "postgres://<user>:<password>@127.0.0.1:<port>/<database>"
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
- Rows include supported objects from your selected non-production database. Pagila should produce table, index, view, and function-related review context, with unsupported object types handled as beta limitations.
- Columns stay in Object Diff details for selected tables, not as top-level Results filter rows.

## 13. Run Database To Repository Compare Preview

Source & Target page:

1. Set Workflow Mode to `Database to Repository Compare`.
2. Confirm Source is PostgreSQL database.
3. Confirm Target is repository desired state.

Compare Options page:

1. Scope: `All`.
2. Click `Preview Repository Sync`.

Expected result:

- Preview shows added, changed, unchanged, or skipped files.
- No repository files are written during preview.
- No PostgreSQL mutation occurs.

## 14. Write Selected Repository Changes

Compare Options page:

1. Review the preview first.
2. Confirm working tree is clean.
3. Type:

```text
WRITE REPOSITORY FILES
```

4. Click `Write Selected Repository Changes`.

Expected result:

- Supported desired-state files are written under `database/objects/`.
- Nothing is written outside the selected workspace.
- PostgreSQL is not mutated.
- No SQL is executed.
- Git does not stage or commit files.

## 15. Commit Captured Desired-State Files Manually

```powershell
cd C:\DbState\PrivateBetaDemo
git status --short --branch --untracked-files=all
git add database\objects
git commit -m "feat: capture private beta database state"
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
- Raw Details shows redacted JSON for support evidence and troubleshooting.
- Raw Details is not the primary review workflow.
- DDL comparison shows Similar, Different, or DDL unavailable.
- Source-only lines use a display-only `+` marker.
- Target-only lines use a display-only `-` marker.
- Diff markers are visual indicators only. They are not part of source DDL, target DDL, repository files, release SQL, or generated review artifacts.

## 19. Run Release Dry-Run From UI

UI:

1. Source & Target page: set Workflow Mode to `Repository to Database Compare`.
2. Compare Options page: Scope `All`.
3. Click `Run Compare`.
4. Open `Release Plan`.
5. Enter release name:

```text
beta_review
```

6. Click `Dry-run Release Artifact`.

Expected result:

- Release Plan shows planned artifact paths, risk information, warnings, and errors where relevant.
- No files are written under `database/releases/`.
- DbState does not execute SQL.
- DbState does not mutate PostgreSQL.
- DbState does not stage or commit Git changes.

## 20. Generate Release Artifacts From UI

Release Plan page:

1. Review the dry-run result first.
2. Type:

```text
GENERATE RELEASE ARTIFACTS
```

3. Click `Generate Release Artifact`.

Expected result:

- Release artifacts are written under `database/releases/`.
- DbState does not execute release SQL.
- DbState does not mutate PostgreSQL.
- DbState does not stage or commit artifacts.

## 21. CLI Release Fallback

Use the CLI if the UI release path cannot be used in the current environment.

Dry-run:

```powershell
cd C:\DbState\PrivateBetaDemo

$env:DBSTATE_POSTGRES_URL = "postgres://<user>:<password>@127.0.0.1:<port>/<database>"

C:\SourceCodes\DbState\target\debug\dbstate.exe release postgres --all --name beta_review --dry-run --format json
```

Write review artifacts:

```powershell
cd C:\DbState\PrivateBetaDemo

C:\SourceCodes\DbState\target\debug\dbstate.exe release postgres --all --name beta_review
```

Expected result:

- CLI dry-run writes no files.
- CLI write creates review artifacts under `database/releases/`.
- DbState does not execute release SQL.
- DbState does not mutate PostgreSQL.
- DbState does not stage or commit artifacts.

## 22. Review Generated Artifacts

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

## 23. Commit Release Artifacts Manually

DbState does not stage or commit release artifacts. If the artifacts are acceptable, commit manually:

```powershell
cd C:\DbState\PrivateBetaDemo
git status --short --branch --untracked-files=all
git add database\releases
git commit -m "chore: add beta review release artifacts"
```

## 24. Reports And Raw JSON

UI Reports / Raw JSON page:

1. Click `Copy JSON`.
2. Paste into a scratch file.
3. Confirm output is redacted.

Expected result:

- No raw PostgreSQL URL.
- No password.
- No token.
- No secret fields.

## 25. Docker Walkthrough

Use Docker if the tester does not want to install Rust locally.

Build image:

```powershell
cd C:\SourceCodes\DbState
docker build -t dbstate-postgres:dev .
```

Run service in Docker:

```powershell
docker run --rm `
  --add-host=host.docker.internal:host-gateway `
  --entrypoint sh `
  -p 127.0.0.1:4587:4587 `
  -v "C:\DbState\PrivateBetaDemo:/workspace" `
  -w /workspace `
  -e DBSTATE_POSTGRES_URL="postgres://<user>:<password>@host.docker.internal:<port>/<database>" `
  dbstate-postgres:dev `
  -c "git config --global --add safe.directory /workspace && /usr/local/bin/dbstate serve --host 0.0.0.0 --port 4587"
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

## 26. Cleanup

Stop service with `Ctrl+C`.

If you created a disposable PostgreSQL database for this walkthrough, drop or remove it according to your local test environment policy.

Remove the fresh workspace only if you no longer need it:

```powershell
Remove-Item -LiteralPath C:\DbState\PrivateBetaDemo -Recurse -Force
```

Do not remove a workspace that contains feedback artifacts or changes you still need.

## 27. Troubleshooting

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

### PostgreSQL Target Not Ready

Confirm your selected non-production PostgreSQL database accepts connections. Example:

```powershell
pg_isready -h 127.0.0.1 -p <port> -U <user> -d <database>
```

### Cannot Connect To Database

Native service should use:

```text
postgres://<user>:<password>@127.0.0.1:<port>/<database>
```

Docker service should use:

```text
postgres://<user>:<password>@host.docker.internal:<port>/<database>
```

### UI Shows Failed To Fetch

- Confirm the service terminal is still running.
- Confirm the URL and port.
- Refresh the browser.

### DBSTATE_POSTGRES_URL Not Set

Set it in the same terminal before starting the service:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://<user>:<password>@127.0.0.1:<port>/<database>"
cargo run -- serve --host 127.0.0.1 --port 4587
```

### Workspace Is Not A Git Repo

Run:

```powershell
cd C:\DbState\PrivateBetaDemo
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
C:\DbState\PrivateBetaDemo
```

Container path:

```text
/workspace
```

Use `/workspace` in the UI when running the service in Docker.

## 28. Private Beta Readiness Checklist

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

## 29. Feedback

Use:

```text
docs/postgresql-v0.1/49-private-beta-feedback-template.md
```

Before sending feedback, redact passwords, full PostgreSQL URLs, production data, customer data, regulated data, tokens, certificates, and secrets.
