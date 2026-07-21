# DbState PostgreSQL v0.1 Private Beta 2 Golden-Path Walkthrough

This walkthrough is the main tester path for DbState PostgreSQL v0.1 Private Beta 2.

Use a sample or development PostgreSQL database that you are allowed to inspect and version. Do not use production, staging, UAT, shared, regulated, customer-data, or business-critical databases.

DbState remains local-first and review-first:

- Git is the source of truth for repository database state.
- The browser UI is for review and preparation.
- DbState does not apply database changes from the browser UI.
- DbState does not mutate Git.
- DbState does not run Database State CI from the browser.
- DbState does not execute release artifacts.
- DbState does not execute reference-data review scripts.
- DbState does not execute reference-data DML during Database State CI.
- DbState does not generate destructive SQL such as `DROP TABLE`, `DROP DOMAIN`, or `DROP AGGREGATE`.

## Supported Private Beta 2 Coverage

Private Beta 2 includes review workflows for:

- schemas
- extensions
- enums
- domains
- sequences
- regular tables
- partitioned parent tables
- constraints
- functions
- aggregates
- triggers
- materialized views
- views
- grants
- RLS policies
- reference-data workflows

Unsupported PostgreSQL features should be reported as coverage gaps with the object type, object name, and workflow where the gap appeared.

## Step 1: Prepare Local Inputs

Expected local folders:

```text
C:\DbState
C:\DbState\YourDatabaseRepo
C:\DbState\dbstate-ci
```

If you received a binary drop, place it at:

```text
C:\DbState\dbstate.exe
```

If you are using a source checkout, run commands from your source checkout root.

Expected result:

- you have a DbState executable
- you have a Git repository folder for the database state
- you have access to a sample or development PostgreSQL database

Report if it fails:

- missing executable
- missing Git
- missing PostgreSQL access
- unclear setup instructions

## Step 2: Start DbState Locally

Binary drop:

```powershell
C:\DbState\dbstate.exe serve --host 127.0.0.1 --port 4587
```

Source checkout:

```powershell
cargo build --release
.\target\release\dbstate.exe serve --host 127.0.0.1 --port 4587
```

Open:

```text
http://127.0.0.1:4587
```

Expected result:

- DbState starts on `127.0.0.1:4587`
- the browser UI opens
- the UI shows the Workspace area

Report if it fails:

- service does not start
- browser cannot connect
- unexpected port conflict
- startup output exposes secrets

## Step 3: Select Or Open A Workspace

In Workspace:

1. Open Select Workspace Folder.
2. Browse to:

```text
C:\DbState\YourDatabaseRepo
```

3. Select the folder.

Expected result:

- Workspace shows the selected repository path
- folder browsing shows useful roots such as `C:\`
- the selected folder row is visually clear
- there is no Service working directory quick-select

Report if it fails:

- selected folder is unclear
- folder picker cannot navigate
- workspace path is not shown
- service working directory appears as a quick-select option

## Step 4: Run Check Status

Click:

```text
Check Status
```

Expected result:

- status refreshes in the Workspace area
- current Git branch and worktree state are visible
- operation result appears in the main alert/message bar
- Warnings panel shows warning details when warnings exist
- clean workspaces show no warnings

Report if it fails:

- Check Status is missing
- old separate Check Workspace and Repo Status primary buttons appear
- operation result appears only in the footer
- Warnings panel says `No warnings yet.` when the latest operation has warnings

## Step 5: Create Or Select A PostgreSQL Profile

Use Source & Target or connection profile controls to create or select a PostgreSQL profile.

Profiles should store only non-secret metadata such as host, port, database, username, SSL mode, and profile name. Passwords and full PostgreSQL URLs are session-only.

Expected result:

- profile can be selected for workflows
- profile password is not persisted
- raw PostgreSQL URL is not shown in reports or Raw JSON

Report if it fails:

- password is saved
- full PostgreSQL URL appears in copied output or Raw JSON
- source profile is confused with Database State CI target

## Step 6: Run Schema Compare - Database To Repository

Choose:

```text
Schema Compare: Database to Repository
```

In Compare Options:

1. Confirm schema/table filters are available.
2. Confirm Include refs and Exclude refs remain available where applicable.
3. Confirm the old implementation coverage checkbox row is not shown.
4. Run the compare.

Expected result:

- compare runs against the selected PostgreSQL profile
- Results show supported schema object differences
- no database changes are applied
- no Git changes are staged or committed

Report if it fails:

- compare options are confusing
- implementation coverage checkboxes still appear
- unsupported object types are silently ignored
- any UI suggests applying SQL to the database

## Step 7: Write Repository Desired-State Files

If the compare identifies objects to write, follow the UI confirmation for writing repository files.

Expected result:

- desired-state files are written under `database/objects/`
- release artifact folders remain under `database/releases/`
- DbState does not mutate PostgreSQL
- DbState does not stage or commit Git changes
- Git Workflow guidance is available after write

Manually inspect Git status:

```powershell
Set-Location C:\DbState\YourDatabaseRepo
git status --short --branch --untracked-files=all
```

Example manual commit after review:

```powershell
git add database/objects/
git add database/reference-data/
git commit -m "sync: update database state"
```

Expected result:

- Git shows only files you expected DbState to write
- manual Git commands are run by you, not by DbState

Report if it fails:

- files are written outside `database/objects/`, `database/reference-data/`, or `database/releases/`
- DbState stages or commits files
- write confirmation is unclear

## Step 8: Run Schema Compare - Repository To Database Review

Choose:

```text
Schema Compare: Repository to Database
```

Run compare in review mode.

Expected result:

- Results show repository/database status
- Object Diff is available for selected schema objects
- Source and target context is clear
- no Apply, Execute, or Sync to Database controls appear

Report if it fails:

- Object Diff opens the wrong object
- Source/Target direction is confusing
- any direct database apply control appears

## Step 9: Review Object Diff

Open Object Diff for a changed object.

Expected result:

- object SQL is readable
- Full Context DDL and Object Only DDL are available for schema workflows
- Related Objects and Raw Details remain review-only
- no SQL is run

Report if it fails:

- DDL panels are blank when object SQL exists
- object identity is unclear
- diff markers appear as if they were real SQL content

## Step 10: Use Release Plan And Artifact Preview

Open Release Plan from Schema Compare: Repository to Database.

Run a dry-run first. Then generate review artifacts only if the UI shows the expected confirmation.

Expected result:

- Release Plan appears for schema Repository-to-Database workflow
- candidate selection is available
- generated schema artifacts go under `database/releases/objects/`
- release artifact preview is read-only
- generated SQL remains review-only
- no release artifact is executed
- no destructive SQL is generated

Report if it fails:

- Release Plan is missing from schema Repository-to-Database workflow
- generated artifacts are not under `database/releases/objects/`
- preview does not open
- generated SQL includes unexpected destructive statements

## Step 11: Run Reference Data Workflows If Applicable

Use reference-data workflows only for lookup/configuration data that is safe to version.

Database to Repository:

- load database tables
- confirm registry status is visible
- select tables and key/masked/ignored columns
- preview reference YAML
- write reference-data files only after confirmation

Repository to Database:

- run reference-data compare
- confirm Results are table-first
- open Data Diff for a selected table
- open row-level Reference-Data Row Data Diff
- generate review-only data script if appropriate

Expected result:

- reference-data files are written under `database/reference-data/`
- review-only scripts are written under `database/releases/reference-data/`
- database-only rows do not generate DELETE statements
- masked values are not exposed
- no reference-data DML is executed
- no Apply, Execute, or Sync to Database controls appear

Report if it fails:

- Results show row spam instead of table summaries
- Object Diff appears for reference-data workflows
- masked values are exposed
- DELETE, MERGE, or TRUNCATE appears in generated reference-data scripts

## Step 12: Open Git Workflow

Open:

```text
Git Workflow
```

Expected result:

- current branch is visible
- protected branch status is visible
- dirty worktree status and dirty path count are visible
- intended or written DbState paths are visible when available
- suggested commit title and body are visible
- suggested pull request title and body are visible
- Git commands are advisory only
- large change sets use grouped `git add` guidance

Report if it fails:

- Git Workflow suggests that DbState will run Git commands
- suggested commit or PR text is blank after writes
- command guidance includes unrelated paths

## Step 13: Open Database State CI Page

Open:

```text
Database State CI
```

Expected result:

- page explains Database State CI is command guidance only
- page uses the selected workspace path when available
- page separates source profile from disposable CI database
- disposable password uses a placeholder or PowerShell variable
- no Run CI button appears
- no Docker command is run by the web app
- no database connection is opened by the web app for CI

Report if it fails:

- page shows a concrete disposable password
- page points CI at the source profile database
- page hardcodes tester-irrelevant local paths
- Run CI appears as a browser button

## Step 14: Optional CLI Database State CI

Run this only if you have Docker Desktop and want to test disposable database validation.

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

Clean up:

```powershell
docker rm -f dbstate-ci-your-database 2>$null
```

Expected result:

- CI refuses to run without `--disposable`
- CI rejects non-empty user databases
- CI validates repository object SQL only against the disposable database
- release artifacts are not executed
- reference-data review scripts are not executed
- reference-data DML is not executed
- text, JSON, and Markdown output redact credentials

Report if it fails:

- CI points at a non-disposable database
- output exposes the disposable password
- release artifacts or reference-data scripts appear to run
- compare-back drift lacks object details

## Step 15: Review Warnings Panel

Create a harmless local warning condition, such as an unrelated untracked file in the workspace, then click Check Status.

Expected result:

- main alert/message bar shows warning count
- Warnings panel shows the warning detail
- duplicate warning text is not repeated
- warning clears after the condition is removed and Check Status is run again

Report if it fails:

- warning count appears but Warnings panel says `No warnings yet.`
- duplicate warnings make the panel noisy
- errors are hidden

## Final Smoke Result

Record:

- tester name
- date
- DbState version/tag
- distribution route
- PostgreSQL version
- workflows tested
- pass/fail result
- blocker/high/medium/low issues
- redacted screenshots or logs
