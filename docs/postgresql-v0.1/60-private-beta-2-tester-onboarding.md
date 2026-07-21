# DbState PostgreSQL v0.1 Private Beta 2 Tester Onboarding

## What DbState Is

DbState is a local-first PostgreSQL database state workflow for private beta testers.

DbState helps you inspect a PostgreSQL database, write database desired-state files into a Git repository, compare repository state back to a database, review object-level differences, generate review-only SQL artifacts, and prepare Git handoff text.

Git is the source of truth for the database state you decide to keep. One durable database object is represented as one repository file where possible.

## What This Beta Is For

Private Beta 2 is for testing the local PostgreSQL workflow:

- creating or opening a DbState workspace
- connecting to a PostgreSQL database you are allowed to inspect
- exporting schema objects into `database/objects/`
- comparing repository state to a PostgreSQL database
- reviewing Object Diff and Data Diff output
- generating review-only release artifacts
- exporting and comparing configured reference data
- using Git Workflow guidance for manual commits and pull requests
- optionally running Database State CI from a terminal against a disposable PostgreSQL database

## What DbState Does Not Do

DbState does not:

- apply SQL to your source database from the browser UI
- run Database State CI from the browser UI
- run Docker from the browser UI
- execute release artifacts
- execute reference-data review scripts
- execute reference-data DML during Database State CI
- generate destructive SQL such as `DROP TABLE`, `DROP DOMAIN`, or `DROP AGGREGATE`
- stage, commit, push, pull, fetch, tag, switch branches, or create branches for you
- store passwords or full PostgreSQL URLs
- target non-PostgreSQL databases in this beta

The browser UI is for review and preparation. You run Git commands, Docker commands, and Database State CI commands manually from your terminal.

## Prerequisites

Use a Windows machine with:

- Git
- PowerShell
- PostgreSQL access
- a PostgreSQL database you are allowed to inspect and version
- DbState binary or source checkout
- Docker Desktop, only if you want to run the optional Database State CI smoke test
- basic PowerShell familiarity

Use only local, development, or sample databases for private beta testing. Do not use production, staging, UAT, shared, regulated, customer-data, or business-critical databases.

Suggested local folders:

```text
C:\DbState
C:\DbState\YourDatabaseRepo
C:\DbState\dbstate.exe
C:\DbState\dbstate-ci
```

## Distribution Option A: Release Binary Drop

If you receive `dbstate.exe` directly:

1. Create a local folder:

```powershell
New-Item -ItemType Directory -Force C:\DbState | Out-Null
```

2. Place the binary at:

```text
C:\DbState\dbstate.exe
```

3. Check that it runs:

```powershell
C:\DbState\dbstate.exe --help
```

## Distribution Option B: Source Checkout

If you receive source access instead of a binary:

1. Clone the repository to a local folder of your choice.
2. Build the release binary:

```powershell
cargo build --release
```

3. Run DbState from the checkout:

```powershell
.\target\release\dbstate.exe --help
```

Tester installer/package scripts may exist for Darwin's packaging process. You do not need to build installers unless Darwin explicitly asks you to test installer packaging.

## First Run Workflow

1. Create or choose a Git repository for database state.

```powershell
New-Item -ItemType Directory -Force C:\DbState\YourDatabaseRepo | Out-Null
Set-Location C:\DbState\YourDatabaseRepo
git init
```

2. Start DbState from a binary drop:

```powershell
C:\DbState\dbstate.exe serve --host 127.0.0.1 --port 4587
```

Or start DbState from a source checkout:

```powershell
cargo build --release
.\target\release\dbstate.exe serve --host 127.0.0.1 --port 4587
```

3. Open:

```text
http://127.0.0.1:4587
```

4. Select or open your workspace:

```text
C:\DbState\YourDatabaseRepo
```

5. Create or select a PostgreSQL connection profile.

Profiles store non-secret metadata only. Passwords and full PostgreSQL URLs are session-only and must not be saved in the repository.

6. Run Check Status.

7. Run Schema Compare: Database to Repository.

8. Review generated repository files under:

```text
database/objects/
database/reference-data/
database/releases/
```

9. Commit manually using Git after review.

Example manual Git commands:

```powershell
git status --short --branch --untracked-files=all
git add database/objects/
git add database/reference-data/
git commit -m "sync: update database state"
```

DbState does not run these Git commands for you.

10. Use Object Diff and Release Plan as review tools.

11. Use the Git Workflow page for suggested commit and pull request text.

DbState does not push, pull, fetch, tag, create branches, or apply SQL to your source database.

## Optional Database State CI Smoke

Database State CI is a CLI command. The web app provides guidance only and does not run CI.

CI must point only at a disposable PostgreSQL validation database. Do not point CI at development, staging, production, shared, regulated, customer-data, or business-critical databases.

Database State CI:

- executes repository desired-state object SQL only against the supplied disposable validation database
- requires `--disposable`
- does not execute release artifacts
- does not execute reference-data review scripts
- does not execute reference-data DML
- does not mutate Git
- does not store the PostgreSQL URL or password

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
C:\DbState\dbstate.exe ci validate `
  --repository C:\DbState\YourDatabaseRepo `
  --postgres-url "postgres://postgres:$DbStateCiPostgresPassword@127.0.0.1:55432/your_database_ci_validation" `
  --disposable
```

Run JSON CI:

```powershell
C:\DbState\dbstate.exe ci validate `
  --repository C:\DbState\YourDatabaseRepo `
  --postgres-url "postgres://postgres:$DbStateCiPostgresPassword@127.0.0.1:55432/your_database_ci_validation" `
  --disposable `
  --json
```

Run Markdown report CI:

```powershell
New-Item -ItemType Directory -Force C:\DbState\dbstate-ci | Out-Null

C:\DbState\dbstate.exe ci validate `
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

## What To Test

Use this checklist during private beta testing:

- Workspace opens
- Check Status works
- PostgreSQL profile can be configured without saving passwords
- Schema Compare: Database to Repository works
- Schema Compare: Repository to Database works in review mode
- Object Diff works
- Release Plan generates review-only artifacts
- Release Artifact Preview works
- Reference Data Compare works
- Reference Data Database-to-Repository export works
- Reference Data Repository-to-Database Data Diff works
- Reference Data review-only script generation works
- Git Workflow page shows branch, status, and suggested commit or pull request text
- Database State CI page shows commands only
- Warnings panel shows operation warnings
- no Run CI button appears
- no Apply, Execute, or Sync to Database button appears

## Feedback Template

Copy this into your feedback channel:

```text
Tester name:
OS and version:
PostgreSQL version:
Database type and approximate size:
DbState version or tag:

Workflow tested:

What worked:

What failed:

Expected behavior:

Actual behavior:

Screenshots or redacted logs:

Did any generated SQL look unsafe?

Was any UI wording confusing?

Suggested severity:
blocker / high / medium / low
```

Do not send passwords, full PostgreSQL URLs, production data, customer data, regulated data, tokens, certificates, or secrets.

## Known Limitations

- PostgreSQL-only for this beta.
- Local service only.
- No hosted or team workflow.
- No direct database apply from the browser UI.
- No Git mutation by DbState.
- No production deployment workflow.
- No Database State CI execution from the browser UI.
- Reference-data scripts are review-only.
- Release artifacts are review-only.
- Destructive SQL generation is intentionally absent.

## Darwin Packaging Checklist

Before sending Private Beta 2 to testers, Darwin should:

- confirm tag `v0.1.0-private-beta.2` exists
- run `cargo build --release`
- decide the distribution route: binary drop, source checkout, or existing package script
- avoid committing generated binaries
- include this tester onboarding guide
- include `docs/postgresql-v0.1/59-private-beta-2-release-readiness.md`
- include `docs/postgresql-v0.1/52-private-beta-known-limitations.md`
- include `docs/postgresql-v0.1/49-private-beta-feedback-template.md`

