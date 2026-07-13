# DbState PostgreSQL v0.1.0 Private Beta 2 Smoke Test Matrix

Use this matrix for pre-release smoke and first-tester validation. Fill the Pass/Fail and Notes columns during execution.

Use the tester's own non-production PostgreSQL database. Do not use production, UAT, staging, shared, regulated, or customer-data databases. If a tester needs a sample database, Pagila is recommended:

```text
https://github.com/devrimgunduz/pagila
```

Use a fresh local workspace such as:

```text
C:\DbState\PrivateBetaDemo
```

## Native Installed Binary

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| Help | Run `C:\Program Files\DbState\dbstate.exe --help`. | Help prints current commands. |  |  |
| Start service | Start `Start DbState Local Service`. | Service binds to `127.0.0.1:4587`. |  |  |
| Health endpoint | Run `Invoke-RestMethod http://127.0.0.1:4587/health`. | JSON health response succeeds. |  |  |
| UI load | Open `http://127.0.0.1:4587/`. | UI loads with safety banner. |  |  |
| Workflow mode order | Open Source & Target. | Workflow Mode order is PostgreSQL Inspect Only, Repository to Database Compare, Database to Repository Compare, Reference-Data Compare. Default is PostgreSQL Inspect Only. |  |  |
| Reference-Data Compare guardrail | Select Reference-Data Compare. | Out-of-scope modal appears and UI returns to the previous valid workflow. |  |  |
| Disabled beta controls | Open Compare Options. | Include refs, Exclude refs, Reference-data scope, Reference-data table, and Run Reference Data Compare are disabled. |  |  |
| Non-Git workspace guardrail | Select a non-Git folder and run workspace/repo/init actions. | Not a Git Repository modal appears instead of confusing raw output. |  |  |
| Non-production database inspect | Follow walkthrough through Inspect against the selected non-production database. | Supported object types appear. Pagila should show table, index, view, constraint, and regular-function rows where present. Unsupported object types remain deferred review context. |  |  |
| Results status filter | Run an operation and use the Status dropdown. | All is the default and status filtering works. repoDifferent rows are prioritized near the top when present. |  |  |
| Database to Repository Compare preview | Run Preview Repository Sync in Database to Repository Compare. | Preview rows appear and no files are written. |  |  |
| Database to Repository Compare write | Type `WRITE REPOSITORY FILES` and click Write Selected Repository Changes on a clean repo. | Files are written only under `database/objects/`. PostgreSQL is not mutated. |  |  |
| Database to Repository Release Plan not applicable | Switch to Database to Repository Compare and open Release Plan. | Release Plan shows not-applicable guidance and points users back to Results for repository writes. |  |  |
| Repository to Database compare | Run compare after capture. | Results grid shows object rows and expected statuses. |  |  |
| Object Diff line comparison | Select a table row. | Full Context DDL and Object Only DDL show line-by-line visual DDL comparison. Matched lines are white, different lines are red, source-only lines are green with display-only `+`, target-only lines are red with display-only `-`, and target-only lines are not struck through. |  |  |
| Object Diff marker safety | Review DDL panels and generated repository/release files. | Display-only `+` and `-` markers do not appear in source DDL, target DDL, repository object files, release SQL, or generated review artifacts. |  |  |
| Related Objects unavailable text | Select an object with unavailable related context. | Unavailable sections say `Not available in Private Beta`. |  |  |
| Raw Details evidence | Open Raw Details or Selected JSON Item. | Panel is usable for support evidence and includes direction/context fields such as objectRef, objectType, producingWorkflowMode, sourceType, and targetType. |  |  |
| Repository to Database Release Plan | Run Repository to Database Compare, then open Release Plan. | Release Context, Risk Summary, Object Summary, Release Candidates, Dry-run / Generated Artifacts, Reviewer Checklist, and Safety Statement are readable. |  |  |
| Release dry-run | Run release dry-run from UI or CLI. | Planned artifacts, warnings, risk reasons, and errors appear when relevant. No files are written. |  |  |
| Release write | Type `GENERATE RELEASE ARTIFACTS` and generate artifacts from Repository to Database Compare. | SQL, summary, risk JSON, and manifest are written under `database/releases/` only. |  |  |
| Dirty tree release block | Try Generate Release Artifact with a dirty working tree. | UI surfaces the dirty working tree condition and tells the tester to commit or stash before generating artifacts. |  |  |

## Docker

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| Docker build | Run `docker build -t dbstate-postgres:dev .`. | Image builds successfully. |  |  |
| Docker service mode | Run service with `-p 127.0.0.1:4587:4587` and mounted workspace. | Service starts. |  |  |
| Docker UI load | Open `http://127.0.0.1:4587/`. | UI loads. |  |  |
| Docker workspace path | Select `/workspace` in UI. | Workspace status resolves inside the container. |  |  |
| Docker executable path | Run `/usr/local/bin/dbstate --help` inside the container. | Help prints current commands. |  |  |
| Docker Git mounted workspace | Add Git safe.directory for mounted Windows workspace when needed. | Repository status works for `/workspace`. |  |  |
| Docker PostgreSQL host access | Use `host.docker.internal` for PostgreSQL on the Windows host, adding `--add-host=host.docker.internal:host-gateway` when needed. | Container can inspect the non-production PostgreSQL database. |  |  |

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

## Validation

| Test | Steps | Expected Result | Pass/Fail | Notes |
| --- | --- | --- | --- | --- |
| cargo fmt | Run `cargo fmt --check`. | Passes. |  |  |
| cargo clippy | Run `cargo clippy --all-targets --all-features -- -D warnings`. | Passes. |  |  |
| cargo build | Run `cargo build`. | Passes. |  |  |
| cargo build release | Run `cargo build --release`. | Passes. |  |  |
| cargo test or policy block | Run `cargo test`. | Passes, or is recorded as a Windows Smart App Control / Application Control policy block if generated Rust test binaries are blocked with `An Application Control policy has blocked this file. (os error 4551)`. |  |  |
| Docker build validation | Run `docker build -t dbstate-postgres:dev .`. | Passes. |  |  |

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
| No internal slice labels | Review tester-facing UI and generated artifacts. | Internal implementation slice labels are not shown. |  |  |

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
