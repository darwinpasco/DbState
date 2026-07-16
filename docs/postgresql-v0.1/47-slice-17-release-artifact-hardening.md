# Slice 17 Release Artifact Hardening

Slice 17 makes `dbstate release postgres` artifacts easier for technical reviewers to inspect before any manual deployment outside DbState.

Release artifacts remain review-only outputs. DbState does not execute release SQL, apply changes to a database, mutate PostgreSQL, or stage, commit, push, pull, fetch, or tag Git changes.

## Command

```powershell
dbstate release postgres --all --name beta_review --dry-run --format json
dbstate release postgres --all --name beta_review
```

Use `DBSTATE_POSTGRES_URL` for session-only PostgreSQL access when running against a disposable or approved read-only target. Do not store the URL in the repository or generated artifacts.

Do not use production, UAT, staging, or shared databases for tests unless read-only inspection has been explicitly approved.

## Artifact Bundle

Schema/object release bundles are written under `database/releases/objects/`.
Reference-data review script bundles are written under `database/releases/reference-data/`.
Both artifact kinds remain under `database/releases/` and are review-only.

Current bundle files:

```text
database/releases/objects/0001_<release-name>.sql
database/releases/objects/0001_<release-name>.summary.md
database/releases/objects/0001_<release-name>.risk.json
database/releases/objects/0001_<release-name>.manifest.json
```

If the first sequence already exists, DbState chooses the next available sequence.

Dry-run reports planned artifact paths and writes no files.

## SQL Artifact

The SQL artifact is a review document. It includes:

- release header
- sequence and release name
- safety statement
- source and target direction
- review sections
- safe CREATE statements where supported
- review-required comments for changed or database-only objects
- blocked item comments
- deferred object type comments

Review sections include:

```text
Summary
Creates
Review Required
Blocked Items
Deferred Object Types
Warnings
```

Supported repo-only objects can produce CREATE text for:

- schema
- table
- extension
- enum
- sequence
- index
- view

Changed objects produce:

```sql
-- REVIEW REQUIRED: object differs; automatic ALTER is not generated in Slice 17.
```

Database-only objects produce:

```sql
-- REVIEW REQUIRED: object exists only in target database. DbState does not generate DROP.
```

DbState never generates destructive SQL in Slice 17, including:

- DROP
- TRUNCATE
- DELETE
- UPDATE
- MERGE
- destructive ALTER
- ALTER TABLE DROP
- GRANT
- REVOKE
- ALTER OWNER

## Summary Markdown

The summary includes:

- release name and sequence
- generated files
- repository path
- Git root
- branch
- working tree status
- scope
- object counts by status
- object counts by type
- risk summary
- blocked items
- skipped items
- warnings
- deferred object types
- safety statement
- reviewer checklist

Reviewer checklist:

```text
1. Review all REVIEW REQUIRED comments.
2. Review blocked and skipped items.
3. Review deferred object type warnings.
4. Confirm no destructive SQL is present.
5. Confirm object ordering is acceptable.
6. Have a DBA or responsible engineer review before any manual execution outside DbState.
```

## Risk JSON

The risk JSON contains stable fields:

- `command`
- `success`
- `releaseName`
- `releaseSequence`
- `databaseType`
- `generatedArtifacts`
- `repositoryContext`
- `scope`
- `counts`
- `objectTypeCounts`
- `riskLevel`
- `riskReasons`
- `blockedItems`
- `skippedItems`
- `warnings`
- `deferredObjectTypes`
- `destructiveSqlGenerated: false`
- `directApplyAvailable: false`
- `generatedSqlExecutionSupported: false`
- `databaseMutationPerformed: false`
- `gitMutationPerformed: false`
- `credentialPersistencePerformed: false`

Risk levels:

- `low`: only supported safe creates are planned.
- `medium`: review-required items or warnings exist without blocked items.
- `high`: database-only, skipped, or many review-required items need extra review.
- `blocked`: release artifact generation is refused.

## Manifest JSON

The manifest records:

- release name
- release sequence
- DbState package version
- artifact type
- relative artifact path
- size in bytes where available

The manifest is intentionally small. Hashing and timestamp metadata remain deferred.

## Blocked Behavior

Blocked selected plan items make release generation fail.

Write mode:

- returns failure
- writes no partial artifacts
- reports blocked items
- keeps risk level `blocked`

Dry-run can still report planned paths and risk information without writing files.

## Current Limitations

- Full dependency ordering is not implemented.
- Changed objects do not generate unsafe ALTER statements.
- Database-only objects do not generate DROP statements.
- Unsupported and deferred object types remain review-only.
- The browser Release Plan page is review-only and points reviewers to CLI dry-run/generation.
- DbState still does not execute SQL or apply changes to PostgreSQL.
