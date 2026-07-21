# DbState PostgreSQL v0.1 Private Beta 2 Tester Announcement And Feedback

## Short Announcement

Hi `<Name>`, I am opening a small private beta for DbState PostgreSQL v0.1 Private Beta 2.

DbState is a local, Git-backed workflow for reviewing and versioning PostgreSQL database state. This beta focuses on schema compare, object diff, review-only release planning, reference-data review, Git workflow visibility, and disposable Database State CI.

This is not a production deployment tool yet. It does not apply SQL from the browser UI and it does not mutate Git.

Would you be willing to test it on a sample or development PostgreSQL database and send structured feedback?

## Longer Tester Email

Subject:

```text
DbState PostgreSQL v0.1 Private Beta 2 - tester invite
```

Body:

```text
Hi <Tester Name>,

I am inviting a small group of technical testers to try DbState PostgreSQL v0.1 Private Beta 2.

I am inviting you because you are comfortable enough with PostgreSQL, Git, and developer tooling to give useful workflow feedback, not just surface-level UI feedback.

What DbState is

DbState is a local, Git-backed PostgreSQL database state and review workflow. It helps inspect a PostgreSQL database, export desired-state files into a repository, compare repository state to a database, review object-level differences, generate review-only SQL artifacts, and prepare commit or pull request text.

What to install or use

Use the package or source access here:

<Download or repo link>

The tester onboarding guide explains the setup options:

docs/postgresql-v0.1/60-private-beta-2-tester-onboarding.md

Use generic local paths such as:

C:\DbState\dbstate.exe
C:\DbState\YourDatabaseRepo
C:\DbState\dbstate-ci

What I would like you to test

- open or initialize a workspace
- configure a PostgreSQL connection profile
- run Check Status
- run Schema Compare: Database to Repository
- run Schema Compare: Repository to Database in review mode
- open Object Diff
- use Release Plan and review-only release artifact generation
- preview generated release artifacts
- try Reference Data Compare if you have safe lookup/configuration tables
- try Reference Data Database-to-Repository export
- try Reference Data Repository-to-Database Data Diff
- try reference-data review-only script generation
- review Git Workflow suggestions
- open the Database State CI guidance page
- optionally run Database State CI manually from PowerShell against a disposable PostgreSQL database
- confirm warning and error messages are visible and understandable

What not to use it for

Please do not use this beta against production, staging, UAT, shared, regulated, customer-data, or business-critical databases.

DbState does not directly apply database changes from the browser UI. DbState does not mutate Git. Database State CI is CLI-only and must be run only against an explicitly disposable PostgreSQL validation database.

How to send feedback

Send feedback to:

<Feedback destination>

For urgent questions, use:

<Support/contact channel>

Expected time commitment

A focused first pass usually takes 30 to 60 minutes if you already have a sample or development PostgreSQL database. A deeper pass through reference data, release artifacts, and Database State CI may take longer.

Please include the DbState version or tag, Windows version, PostgreSQL version, workflow tested, what worked, what failed, and any redacted screenshots or logs.

Thank you for helping test this early version.
```

## Short Chat Or DM Version

```text
Hi <Name>, I am opening a small private beta for DbState PostgreSQL v0.1 Private Beta 2.

DbState is a local Git-backed workflow for reviewing and versioning PostgreSQL database state. This beta focuses on schema compare, object diff, review-only release planning, reference-data review, Git workflow visibility, and disposable Database State CI.

This is not a production deployment tool yet. It does not apply SQL from the browser UI and it does not mutate Git.

Would you be willing to test it on a sample or development PostgreSQL database and send structured feedback?
```

## Tester Onboarding Checklist

Setup:

- confirm Windows machine
- confirm Git is installed
- confirm PostgreSQL access
- confirm access to a sample or development database
- install or run DbState
- open `http://127.0.0.1:4587`

Core workflows:

- open or select workspace
- create or select connection profile
- run Check Status
- run Schema Compare: Database to Repository
- run Schema Compare: Repository to Database review
- open Object Diff
- use Release Plan
- use Release Artifact Preview
- run Reference Data Compare
- run Reference Data Database-to-Repository export
- run Reference Data Repository-to-Database Data Diff
- generate a Reference Data review-only script
- review Git Workflow suggestions
- open Database State CI guidance page
- optionally run CLI Database State CI with disposable PostgreSQL
- check Warnings panel behavior

Safety checks:

- verify no Run CI button appears in the web app
- verify no database apply, SQL execution, or database sync button appears in the web app
- verify Git commands are suggestions only
- verify generated SQL is review-only
- verify Database State CI uses an explicitly disposable PostgreSQL database

## Feedback Form Template

Copy and complete this template:

```text
Tester name:
Date:
DbState version/tag:
OS/version:
PostgreSQL version:
Database type/size:
Distribution used: binary / source checkout / package

Workflows tested:

What worked well:

What was confusing:

What failed:

Steps to reproduce:

Expected result:

Actual result:

Screenshots/logs:

Any generated SQL that looked unsafe:

Missing object types or unsupported PostgreSQL features:

Performance notes:

UI/wording feedback:

Severity:

Would you use this again? Why or why not?

Permission to follow up:
```

Do not include passwords, full PostgreSQL URLs, production data, customer data, regulated data, tokens, certificates, or secrets.

## Bug Report Template

```text
Title:
Severity:
Environment:
Workflow:

Steps to reproduce:

Expected:

Actual:

Logs/screenshots:

Workaround, if any:

Data safety concern? yes/no
```

## Severity Guide

Blocker:
prevents testing or risks unsafe behavior.

High:
major workflow broken, incorrect output, or serious confusion.

Medium:
workflow issue with workaround, incomplete support, or confusing UI.

Low:
copy, polish, minor usability issue, or docs issue.

## Tester Expectations

Testers should:

- use a sample or development PostgreSQL database only
- keep database credentials out of feedback
- review generated files before running any manual Git command
- treat release artifacts and reference-data scripts as review-only output
- report confusing wording even when the workflow technically works
- report any generated SQL that looks unsafe or surprising

Testers should not:

- use DbState as a production deployment tool
- point Database State CI at a non-disposable database
- paste secrets or full PostgreSQL URLs into feedback
- expect hosted collaboration features
- expect DbState to stage, commit, push, pull, fetch, tag, or create branches
- expect the browser UI to run Database State CI

## Known Limitations Summary

- PostgreSQL-only.
- Local-only private beta.
- No hosted or team workflow.
- No production or staging deployment workflow.
- No direct database apply from the browser UI.
- No Git mutation by DbState.
- Database State CI is manual terminal only.
- Reference-data scripts are review-only.
- Release artifacts are review-only.
- Docker and disposable PostgreSQL validation environments are managed by the user.
- Destructive SQL generation is intentionally absent.

## Out Of Scope For This Beta

Do not test Private Beta 2 as:

- a production deployment tool
- a hosted service
- a team collaboration server
- a migration runner
- a Git automation bot
- a replacement for PostgreSQL backup/restore
- a tool for versioning transactional production data
- a non-PostgreSQL database workflow

