# Slice 22: Private Beta UI Guardrails, Release Plan Actions, and Line Diff

Slice 22 tightens the DbState PostgreSQL private beta UI without changing the core safety boundary.

## Workflow Mode order

The private beta UI presents workflow modes in this order:

1. PostgreSQL Inspect Only
2. Repository to Database Compare
3. Database to Repository Compare
4. Reference-Data Compare

PostgreSQL Inspect Only is the default workflow.

## Beta guardrails

If the selected local repository path is not a Git repository, Workspace actions show a `Not a Git Repository` modal instead of leaving testers with raw service JSON. DbState still allows a Git repository that has not yet been initialized as a DbState project to use Init Plan and Initialize DbState Project.

Reference-Data Compare was intentionally guarded in Slice 22. Slice 34 removes that guardrail and enables the UI workflow for configured reference-data compare. The workflow remains read-only, registry-bound, and does not generate DML or apply data changes.

The following controls remain disabled in the current beta UI:

- Include refs
- Exclude refs

The following Reference-Data Compare controls are enabled as of Slice 34:

- Reference-data registry status
- Configured table selection
- Reference-data scope
- Reference-data table
- Run Reference Data Compare

## Direction-specific workflows

Database to Repository Compare captures supported PostgreSQL object definitions into repository files under `database/objects/`. The repository write remains a controlled local file write and requires typed confirmation. It does not mutate PostgreSQL and does not stage, commit, fetch, pull, push, or tag Git changes.

Repository to Database Compare is the workflow that uses Release Plan. Release Plan can dry-run or generate reviewable release artifact files under `database/releases/` through the local DbState Service.

Release artifact generation requires:

- a release name
- clean repository working tree
- typed confirmation `GENERATE RELEASE ARTIFACTS` for write

The generated release artifact bundle may include:

- SQL synchronization/review script
- summary markdown
- risk JSON
- manifest JSON

The SQL file is reviewable output only. DbState does not execute SQL and does not apply database changes.

## Release Plan readability

Release Plan no longer renders selected objects as a single inline paragraph. It now presents:

- release context
- risk summary
- object summary
- CLI command guidance
- release candidates table
- dry-run/generated artifact result
- reviewer checklist
- safety statement

## Object Diff line-by-line DDL comparison

Full Context DDL and Object Only DDL now render aligned line-by-line Source and Target panels.

Line display rules:

- matched lines are white
- different lines are red
- source-only lines are green with a display-only plus marker and a blank target counterpart
- target-only lines are red with a display-only minus marker and a blank source counterpart

Target-only lines in Repository to Database Compare are review-required. They do not mean DbState will automatically delete database objects or database lines. DbState does not generate destructive database changes in this beta.

Target-only lines in Database to Repository Compare can indicate repository-file content that may be removed if the user writes repository changes.

The plus and minus markers are visual indicators only. They are rendered as separate UI marker elements and must not be included in Source DDL, Target DDL, repository object files, release artifact SQL, generated review artifacts, or future repository/database write logic.

## Safety boundary

Slice 22 does not add SQL execution, PostgreSQL mutation, direct database apply, destructive SQL generation, Git automation, telemetry, hosted mode, or secret persistence.
