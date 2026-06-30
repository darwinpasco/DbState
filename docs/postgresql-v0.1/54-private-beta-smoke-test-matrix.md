# Private Beta Smoke Test Matrix

Use this matrix for pre-release smoke and first-tester validation. Fill the Pass/Fail and Notes columns during execution.

## Native Installed Binary

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| Help | Run `C:\Program Files\DbState\dbstate.exe --help`. | Help prints current commands. |  |  |
| Start service | Start `Start DbState Local Service`. | Service binds to `127.0.0.1:4587`. |  |  |
| Health endpoint | Run `Invoke-RestMethod http://127.0.0.1:4587/health`. | JSON health response succeeds. |  |  |
| UI load | Open `http://127.0.0.1:4587/`. | UI loads with safety banner. |  |  |
| ParkingDemo inspect | Follow walkthrough through Inspect. | Supported object types appear. |  |  |
| Database to Repository preview | Run Preview Repository Sync. | Preview rows appear and no files are written. |  |  |
| Database to Repository write | Type `WRITE REPOSITORY FILES` and run write on a clean repo. | Files are written only under `database/objects/`. |  |  |
| Repository to Database compare | Run compare after capture. | Results grid shows object rows and expected statuses. |  |  |
| Object Diff | Select a table row. | Full Context DDL, Object Only DDL, Related Objects, and Raw Details work. |  |  |
| Release dry-run | Run release dry-run from CLI. | Planned artifacts and risk output appear, no files written. |  |  |
| Release write | Run release write from CLI. | SQL, summary, risk JSON, and manifest are written under `database/releases/`. |  |  |

## Docker

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| Docker build | Run `docker build -t dbstate-postgres:dev .`. | Image builds successfully. |  |  |
| Docker service mode | Run service with `-p 127.0.0.1:4587:4587` and mounted workspace. | Service starts. |  |  |
| Docker UI load | Open `http://127.0.0.1:4587/`. | UI loads. |  |  |
| Docker workspace path | Select `/workspace` in UI. | Workspace status resolves inside the container. |  |  |

## CLI

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| Repo status | Run `dbstate repo status --format json`. | Repository context JSON returns. |  |  |
| Init dry-run | Run `dbstate init --dry-run --format json`. | Planned paths return and no files are written. |  |  |
| Inspect | Run `dbstate inspect postgres --all --format json`. | Read-only object inventory returns. |  |  |
| Compare | Run `dbstate compare postgres --all --format json`. | Differences return without file writes. |  |  |
| Plan | Run `dbstate plan postgres --all --format json`. | Plan items and warnings return. |  |  |
| Release dry-run | Run `dbstate release postgres --all --name beta_review --dry-run --format json`. | Planned artifacts return and no files are written. |  |  |
| Release write | Run `dbstate release postgres --all --name beta_review`. | Review artifacts are written under `database/releases/`. |  |  |
| Data compare empty registry | Run configured reference-data compare against empty registry. | Valid empty result or clear configured-table guidance. |  |  |

## Installer

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| Install | Run setup `.exe`. | DbState installs to `C:\Program Files\DbState`. |  |  |
| Start Menu service shortcut | Click `Start DbState Local Service`. | Console service starts locally. |  |  |
| UI shortcut | Click `Open DbState UI`. | Browser opens local UI. |  |  |
| Installed folder excludes source | Inspect installed folder. | No `src`, `.git`, Cargo files, tests, workspaces, or profiles. |  |  |
| Uninstall | Uninstall from Windows Apps or Control Panel. | Installed app files are removed. |  |  |

## Safety

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| No direct apply | Review UI, CLI help, and docs. | No direct database apply workflow exists. |  |  |
| No SQL execution | Review UI, CLI help, and release artifacts. | DbState does not execute SQL. |  |  |
| No PostgreSQL mutation | Run inspect, compare, preview, and release dry-run. | PostgreSQL remains unchanged. |  |  |
| No Git automation | Run write workflows. | DbState does not stage, commit, push, pull, fetch, or tag. |  |  |
| No password persistence | Create and inspect non-secret profile storage. | No password is stored. |  |  |
| No raw URL in Raw JSON | Use session URL and inspect Reports / Raw JSON. | Raw URL is redacted. |  |  |

## Smoke Result Summary

- Tester:
- Date:
- DbState tag:
- Installer filename:
- OS:
- Browser:
- Overall result:
- Blocking issues:
- Follow-up needed:
