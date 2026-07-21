# DbState PostgreSQL v0.1 Private Beta 2 Release Readiness

## Purpose

This checklist prepares `DbState PostgreSQL v0.1 Private Beta 2` for Darwin's final tag and release review.

Target audience: private beta testers using DbState locally against their own PostgreSQL development or sample databases.

Supported posture:

- local-only DbState service and browser UI
- Git-backed desired-state repository
- review-first schema and reference-data workflows
- Git remains the source of truth
- browser UI is review and preparation only
- Database State CI is CLI-only

DbState does not directly apply database changes from the browser UI. DbState does not mutate Git, execute release artifacts, execute reference-data review scripts, execute reference-data DML during CI, or generate destructive SQL such as `DROP TABLE`, `DROP DOMAIN`, or `DROP AGGREGATE`.

## Private Beta 2 Capability Summary

Private Beta 2 is expected to include:

- schema compare workflows for repository-to-database and database-to-repository review
- Object Diff for schema objects
- release planning and review-only release artifacts
- release artifact preview
- candidate selection for review artifact generation
- PostgreSQL object support for schemas, extensions, enums, domains, sequences, regular tables, partitioned parent tables, constraints, functions, aggregates, triggers, materialized views, views, grants, and RLS policies
- reference-data compare workflows
- reference-data database-to-repository export and onboarding
- reference-data repository-to-database table summaries and Data Diff
- reference-data review-only script generation
- Git Workflow visibility
- semantic commit and pull request guidance
- Database State CI CLI validation against an explicit disposable PostgreSQL database
- Database State CI guidance page in the web app
- warning and operation-result message-bar UI polish

## Explicit Non-Goals And Limitations

Private Beta 2 does not include:

- direct database apply from the browser UI
- production or staging targeting helpers
- Git staging, committing, pushing, pulling, fetching, tagging, switching, or branch creation by DbState
- automatic CI execution from the web UI
- reference-data DML execution by Database State CI
- release artifact execution by Database State CI
- destructive generated SQL
- `DROP TABLE`, `DROP DOMAIN`, or `DROP AGGREGATE` generation
- non-PostgreSQL database editions
- hosted service posture
- automatic Docker or PostgreSQL management from the web UI

The user must manage any disposable PostgreSQL or Docker validation environment manually.

## Required Validation Before Tagging

Run these from the repository root:

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
cargo build --release
git diff --check
```

All commands should pass before Darwin creates the Private Beta 2 tag.

## Web Smoke Checklist

Start the local service:

```powershell
.\target\release\dbstate.exe serve --host 127.0.0.1 --port 4587
```

Open:

```text
http://127.0.0.1:4587
```

Verify:

- Workspace opens
- Check Status works
- Schema Compare opens
- Object Diff opens
- Release Plan opens
- Reference Data pages open
- Git Workflow opens
- Database State CI page opens
- no Run CI button appears
- no Apply, Execute, or Sync to Database button appears
- disposable Database State CI password guidance remains placeholder-only
- Warnings panel shows warnings from the latest operation result

## Database State CI Smoke

These commands are manual terminal guidance. The web app must not run Docker, connect to PostgreSQL for CI, run Database State CI, execute SQL, or store the disposable PostgreSQL password.

Set a temporary disposable password:

```powershell
$DbStateCiPostgresPassword = "<DISPOSABLE_POSTGRES_PASSWORD>"
```

Start a disposable PostgreSQL container:

```powershell
docker rm -f dbstate-ci-your-database 2>$null

docker run --name dbstate-ci-your-database `
  -e POSTGRES_PASSWORD=$DbStateCiPostgresPassword `
  -e POSTGRES_DB=your_database_ci_validation `
  -p 55432:5432 `
  -d postgres:16

for ($i = 1; $i -le 30; $i++) {
  docker exec dbstate-ci-your-database pg_isready -U postgres -d your_database_ci_validation
  if ($LASTEXITCODE -eq 0) { break }
  Start-Sleep -Seconds 1
}
```

Run text CI:

```powershell
.\target\release\dbstate.exe ci validate `
  --repository C:\DbState\YourDatabaseRepo `
  --postgres-url "postgres://postgres:$DbStateCiPostgresPassword@127.0.0.1:55432/your_database_ci_validation" `
  --disposable
```

Run JSON CI:

```powershell
.\target\release\dbstate.exe ci validate `
  --repository C:\DbState\YourDatabaseRepo `
  --postgres-url "postgres://postgres:$DbStateCiPostgresPassword@127.0.0.1:55432/your_database_ci_validation" `
  --disposable `
  --json
```

Run Markdown report CI:

```powershell
New-Item -ItemType Directory -Force C:\DbState\dbstate-ci | Out-Null

.\target\release\dbstate.exe ci validate `
  --repository C:\DbState\YourDatabaseRepo `
  --postgres-url "postgres://postgres:$DbStateCiPostgresPassword@127.0.0.1:55432/your_database_ci_validation" `
  --disposable `
  --report C:\DbState\dbstate-ci\database-state-ci-report.md

Get-Content C:\DbState\dbstate-ci\database-state-ci-report.md -Raw
```

Clean the disposable container:

```powershell
docker rm -f dbstate-ci-your-database 2>$null
```

Expected successful result:

```text
Success: true
Result: PASS
Object files: total=<n>, applied=<n>, skipped=<n>, failed=0
Compare-back: repositoryOnly=0, databaseOnly=0, different=0, unexpectedDrift=false
```

Expected non-fatal warnings may include:

- skipped bootstrap public schema
- deferred and retried dependent functions
- ignored PostgreSQL bootstrap public schema grant noise during CI compare-back

## Packaging Notes

Existing Windows private beta packaging guidance lives in:

- `docs/postgresql-v0.1/53-private-beta-installer-distribution.md`
- `packaging/windows/Build-WindowsInstaller.ps1`
- `packaging/windows/Build-PrivateBetaPackage.ps1`

Do not commit generated installer binaries.

If Darwin does not use the installer pipeline for a tester drop, Private Beta 2 can be distributed as:

- source repository
- release binary built with `cargo build --release`
- documentation and validation checklist
- tester instructions

Do not invent a new installer or packaging process for this release.

## Manual Final Tagging Step

Darwin should tag only after validation and smoke testing pass:

```powershell
git tag -a v0.1.0-private-beta.2 -m "DbState PostgreSQL v0.1 Private Beta 2"
git push origin v0.1.0-private-beta.2
```

These commands are the manual final release step. Codex must not run them during release-readiness preparation.

