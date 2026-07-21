# DbState PostgreSQL v0.1 Private Beta 2 Smoke Test Matrix

Use this matrix for Private Beta 2 pre-release smoke and tester validation.

Use a sample or development PostgreSQL database only. Do not use production, staging, UAT, shared, regulated, customer-data, or business-critical databases.

Fill in Pass/Fail and Notes / Evidence during execution.

| Area | Test | Steps | Expected Result | Pass/Fail | Notes / Evidence |
| --- | --- | --- | --- | --- | --- |
| Workspace | Open workspace | Open `http://127.0.0.1:4587`, select `C:\DbState\YourDatabaseRepo`. | Workspace path is visible and accepted. |  |  |
| Workspace | Check Status | Click Check Status. | Workspace and repository status refresh through one clear action. |  |  |
| Workspace | Warnings panel dirty workspace warning | Create an unrelated untracked file, click Check Status, then open Warnings. | Main alert shows warning count and Warnings panel shows warning detail once. |  |  |
| Workspace | Folder picker cleanup | Open Select Workspace Folder. | Roots, Up, Refresh, and Select this folder are present; Service working directory quick-select is absent; selected row is visually clear. |  |  |
| Connection profile | Create/select PostgreSQL profile | Create or select a non-production PostgreSQL profile. | Profile can be used without storing a password or full PostgreSQL URL. |  |  |
| Connection profile | Source profile is not CI target | Open Database State CI page after selecting a profile. | Page explains the profile identifies the source database and CI uses a separate disposable database. |  |  |
| Schema compare | Database to Repository | Run Schema Compare: Database to Repository. | Supported object differences appear and no database changes are applied. |  |  |
| Schema compare | Repository to Database review | Run Schema Compare: Repository to Database. | Review results appear without direct database apply controls. |  |  |
| Schema compare | Filters still work | Use schema/table filters and include/exclude refs where applicable. | Results reflect selected filters. |  |  |
| Schema compare | Compare Options cleanup | Open Compare Options. | Implementation coverage checkbox row is absent; schema/table/reference-data controls remain. |  |  |
| Object coverage | Domains | Compare or export a database with a domain. | Domain appears as a supported schema object. |  |  |
| Object coverage | Aggregates | Compare or export a database with an aggregate. | Aggregate appears as a supported schema object. |  |  |
| Object coverage | Partitioned parent tables | Compare or export a partitioned parent table. | Parent table is represented under table desired-state files. |  |  |
| Object coverage | Functions | Compare or export functions. | Functions appear with deterministic object identity. |  |  |
| Object coverage | Materialized views | Compare or export materialized views. | Materialized views appear as supported objects. |  |  |
| Object coverage | Indexes on materialized views | Run Database State CI or review object inventory with a materialized-view index. | Materialized view is available before its index in CI build behavior. |  |  |
| Object coverage | Grants | Compare or export grants. | Grants are represented as reviewable desired-state objects. |  |  |
| Object coverage | RLS policies | Compare or export RLS policies. | RLS policies appear without destructive RLS state changes. |  |  |
| Object Diff | Object SQL review | Select a changed schema object and open Object Diff. | Full Context DDL, Object Only DDL, Related Objects, and Raw Details are review-only and readable. |  |  |
| Object Diff | No apply controls | Inspect Object Diff. | No Apply, Execute, or Sync to Database controls appear. |  |  |
| Release Plan | Run plan | Open Release Plan for Schema Compare: Repository to Database. | Plan shows candidates, risk context, safety statement, and reviewer checklist. |  |  |
| Release Plan | Candidate selection | Select a subset of release candidates. | Generated plan/artifacts reflect only selected candidates and dependency warnings remain visible. |  |  |
| Release Plan | Review-only artifact generation | Generate release artifacts after typed confirmation. | Artifacts are written under `database/releases/objects/` and are review-only. |  |  |
| Release Plan | Release artifact preview | Click Preview for generated `.sql`, `.summary.md`, `.risk.json`, or `.manifest.json`. | Preview opens read-only and shows the selected artifact. |  |  |
| Release Plan | No artifact execution | Review UI, output, and generated files. | Release artifacts are not executed by DbState. |  |  |
| Release Plan | No destructive SQL generation | Inspect generated release SQL. | Unexpected `DROP TABLE`, `DROP DOMAIN`, `DROP AGGREGATE`, DELETE, TRUNCATE, or MERGE statements are absent. |  |  |
| Reference Data | Registry | Open Reference Data workflow with configured registry. | Registry status is visible and invalid registry errors are clear. |  |  |
| Reference Data | DB-to-Repo export/onboarding | Load database tables, select key/masked/ignored columns, preview YAML, then write after confirmation. | Files are written only under `database/reference-data/`. |  |  |
| Reference Data | Repo-to-DB Data Diff | Run Reference Data Compare: Repository to Database. | Results are table-first; Data Diff opens per selected table. |  |  |
| Reference Data | Review-only script generation | Generate reference-data review script after typed confirmation. | Artifacts are written under `database/releases/reference-data/`; database-only rows do not generate DELETE. |  |  |
| Reference Data | No DML execution | Review reference-data workflows and CI output. | DbState does not execute reference-data inserts, updates, deletes, merges, or review scripts. |  |  |
| Git Workflow | Branch/status visibility | Open Git Workflow. | Current branch, protected status, dirty status, and dirty path count are visible. |  |  |
| Git Workflow | Dirty warning | Create unrelated dirty file and refresh status. | Warning is visible and scoped write guard behavior is understandable. |  |  |
| Git Workflow | Semantic commit/PR guidance | Generate or review suggested commit and PR text. | Suggestions are based on staged or written DbState context and remain editable/copyable. |  |  |
| Git Workflow | Git commands are suggestions only | Inspect Git Workflow commands. | DbState does not run Git commands; no automatic staging, commit, push, pull, fetch, tag, switch, or branch creation occurs. |  |  |
| Database State CI | Web page command guidance only | Open Database State CI page. | Page explains CI and shows commands only; the web app does not run CI. |  |  |
| Database State CI | Password placeholder-only | Inspect CI command blocks. | Commands use `$DbStateCiPostgresPassword` and `<DISPOSABLE_POSTGRES_PASSWORD>`, not a concrete password. |  |  |
| Database State CI | No Run CI button | Inspect Database State CI page. | No Run CI button appears. |  |  |
| Database State CI | Optional CLI validate | Run `dbstate ci validate --repository C:\DbState\YourDatabaseRepo --postgres-url "postgres://postgres:$DbStateCiPostgresPassword@127.0.0.1:55432/your_database_ci_validation" --disposable` against a disposable database. | CI validates repository desired-state object SQL against the disposable database only. |  |  |
| Database State CI | JSON/report output | Run optional CI with `--json` and `--report C:\DbState\dbstate-ci\database-state-ci-report.md`. | JSON/report output is structured and redacts credentials. |  |  |
| Database State CI | Release artifacts not executed | Include release artifacts in repository and run CI. | Release artifacts are counted/safety-checked but not executed. |  |  |
| Database State CI | Reference-data DML not executed | Include reference-data files and review scripts, then run CI. | Reference-data YAML is validated; DML and review scripts are not executed. |  |  |
| Safety | No direct browser apply | Review all browser pages. | No Apply, Execute, or Sync to Database controls appear. |  |  |
| Safety | No CI endpoint | Review service routes or UI source where applicable. | No `/api/v1/ci` endpoint is added for browser CI execution. |  |  |
| Safety | No concrete disposable passwords | Review tester-facing docs and CI guidance page. | No `POSTGRES_PASSWORD=postgres` or concrete disposable PostgreSQL URL password appears. |  |  |
| Safety | No Darwin-local paths | Review tester-facing docs. | No Darwin-local repository paths are used as tester defaults. |  |  |
| Safety | No production/staging/shared DB usage | Review docs and UI copy. | Testers are told to use sample/development databases only and disposable DBs for CI. |  |  |
| Packaging/docs | Onboarding doc present | Open `docs/postgresql-v0.1/60-private-beta-2-tester-onboarding.md`. | Tester onboarding guide is present. |  |  |
| Packaging/docs | Announcement/feedback doc present | Open `docs/postgresql-v0.1/61-private-beta-2-tester-announcement-and-feedback.md`. | Announcement, email, feedback, bug report, and severity guide are present. |  |  |
| Packaging/docs | Release readiness doc present | Open `docs/postgresql-v0.1/59-private-beta-2-release-readiness.md`. | Private Beta 2 release readiness checklist is present. |  |  |
| Packaging/docs | Golden Path doc present | Open `docs/postgresql-v0.1/48-slice-18-golden-path-private-beta-walkthrough.md`. | Current Private Beta 2 walkthrough is present. |  |  |
| Packaging/docs | Smoke Test Matrix present | Open this smoke matrix. | Current Private Beta 2 matrix is present. |  |  |

## Stop Testing And Report Immediately

Stop testing and report a blocker if:

- DbState suggests applying SQL directly to a source database from the browser.
- DbState mutates Git without explicit manual user action.
- generated SQL appears destructive unexpectedly.
- a CI command points at a non-disposable database.
- a PostgreSQL password appears as a concrete default.
- a Run CI button appears in the browser.
- an Apply, Execute, or Sync to Database button appears.

## Severity Guide

Blocker:
prevents testing or risks unsafe behavior.

High:
major workflow broken, incorrect output, or serious confusion.

Medium:
workflow issue with workaround, incomplete support, or confusing UI.

Low:
copy, polish, minor usability issue, or docs issue.

## Smoke Result Summary

- Tester:
- Date:
- DbState tag:
- Distribution route:
- OS:
- Browser:
- PostgreSQL version:
- Overall result:
- Blocking issues:
- Follow-up needed:
