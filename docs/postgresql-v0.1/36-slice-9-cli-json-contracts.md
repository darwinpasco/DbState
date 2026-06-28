# Slice 9 CLI JSON Contracts

Slice 9 hardens the current DbState PostgreSQL v0.1 CLI surface before adding more product capability.

This slice does not add database coverage, SQL execution, deployment automation, service APIs, browser UI, Docker runtime, MCP, AI integration, or direct database apply.

## Implemented Commands

- `dbstate repo status`
- `dbstate init`
- `dbstate inspect postgres`
- `dbstate export postgres`
- `dbstate sync postgres`
- `dbstate compare postgres`
- `dbstate plan postgres`
- `dbstate release postgres`
- `dbstate data-compare postgres`

All implemented commands support text output and JSON output with `--format json` or `--json`.

## Common JSON Fields

Every JSON response includes:

```json
{
  "command": "repo status",
  "success": true,
  "warnings": [],
  "errors": []
}
```

Repository-bound commands include repository context when available:

```json
{
  "repositoryPath": "D:/SourceCodes/Example",
  "gitRoot": "D:/SourceCodes/Example",
  "isGitRepository": true,
  "branch": "main",
  "workingTreeStatus": "clean",
  "isDirty": false
}
```

PostgreSQL commands include:

```json
{
  "databaseType": "postgresql"
}
```

PostgreSQL object commands include `deferredObjectTypes` where deferred schema object coverage matters.

## Command-Specific Sections

`dbstate repo status` and `dbstate init` report:

- `dbstateProjectStatus`
- `missingPaths`
- `existingPaths`
- `plannedCreates`
- `createdPaths`

`dbstate inspect postgres` reports:

- `inspectionScope`
- `schemas`
- `tables`
- `columns`
- `counts`
- `deferredObjectTypes`

`dbstate export postgres` reports:

- `exportScope`
- `dryRun`
- `selectedSchemas`
- `selectedTables`
- `plannedFiles`
- `createdFiles`
- `skippedFiles`

`dbstate sync postgres` reports:

- `syncScope`
- `dryRun`
- `addedFiles`
- `changedFiles`
- `unchangedFiles`
- `plannedCreates`
- `plannedUpdates`
- `createdFiles`
- `updatedFiles`

`dbstate compare postgres` reports:

- `compareScope`
- `inSync`
- `repoDifferent`
- `repoOnly`
- `databaseOnly`
- `skipped`

Differences are command results, not command failures.

`dbstate plan postgres` reports:

- `planScope`
- `includedObjects`
- `excludedObjects`
- `planItems`
- `blockedItems`
- `dependencyWarnings`
- `compareSummary`

Blocked plan items can appear in a successful plan report. The report is still read-only and in memory.

`dbstate release postgres` reports:

- `releaseName`
- `releaseScope`
- `dryRun`
- `planItems`
- `blockedItems`
- `plannedArtifacts`
- `createdArtifacts`
- `riskLevel`

Blocked selected plan items make release artifact generation fail because artifacts are not written.

`dbstate data-compare postgres` reports:

- `dataCompareScope`
- `compareScope`
- `selectedTables`
- `tableResults`
- `counts`
- `inSync`
- `repoDifferent`
- `repoOnly`
- `databaseOnly`
- `skipped`

`compareScope` is retained for compatibility. `dataCompareScope` is the normalized Slice 9 field for reference-data compare scope.

## JSON Shape Examples

Status:

```json
{
  "command": "repo status",
  "success": true,
  "repositoryPath": "D:/work/example",
  "gitRoot": "D:/work/example",
  "isGitRepository": true,
  "branch": "main",
  "workingTreeStatus": "clean",
  "isDirty": false,
  "dbstateProjectStatus": "completeDbStateStructure",
  "missingPaths": [],
  "existingPaths": [],
  "plannedCreates": [],
  "createdPaths": [],
  "warnings": [],
  "errors": []
}
```

Inspect:

```json
{
  "command": "inspect postgres",
  "success": true,
  "databaseType": "postgresql",
  "inspectionScope": ["schemas", "tables", "columns"],
  "schemas": [],
  "tables": [],
  "columns": [],
  "counts": { "schemas": 0, "tables": 0, "columns": 0 },
  "warnings": [],
  "errors": [],
  "deferredObjectTypes": ["extensions", "indexes", "functions"]
}
```

Data compare:

```json
{
  "command": "data-compare postgres",
  "success": true,
  "databaseType": "postgresql",
  "dataCompareScope": "all",
  "selectedTables": ["dbstate_ref.payment_methods"],
  "counts": {
    "inSync": 1,
    "repoDifferent": 1,
    "repoOnly": 1,
    "databaseOnly": 1,
    "skipped": 0
  },
  "warnings": [],
  "errors": []
}
```

Examples intentionally omit credentials, connection strings, row values, and local machine details beyond non-secret repository paths.

## Exit-Code Rules

Expected exit-code behavior:

- `0` for successful read-only commands.
- `0` for successful dry-run commands.
- `0` for successful local file writes or release artifact generation.
- `0` for compare or data-compare results that contain differences.
- `0` for plan reports that include blocked items when the plan report was generated.
- Non-zero for invalid arguments.
- Non-zero for missing required URL, scope, or release name.
- Non-zero for connection failure.
- Non-zero for missing project structure.
- Non-zero for dirty working tree when a write command requires a clean tree.
- Non-zero for blocked release generation.
- Non-zero for file read/write failure.

## Redaction Rules

JSON and text output must not include:

- Raw PostgreSQL connection URLs.
- Passwords.
- Tokens.
- Credentials.
- Local-only sensitive values.
- Raw masked reference-data values.
- Full row dumps.

Known placeholder secrets are covered by tests for JSON output and PostgreSQL integration output.

## Safety Boundary

DbState PostgreSQL v0.1 product commands may read local repository files, write local object files through explicit export or sync commands, write release artifacts through the explicit release command, and run read-only PostgreSQL catalog or `SELECT` queries where already implemented.

DbState must not execute generated SQL, apply changes to a database, generate reference-data DML in Slice 9, or mutate PostgreSQL through product commands.

Test-only fixture SQL may create schemas, tables, and rows only inside a disposable PostgreSQL database configured through `DBSTATE_TEST_POSTGRES_URL`.

Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database.

## Current Limitations

- No formal JSON Schema files are generated in Slice 9.
- `--repo <path>` remains an open decision. Commands use the current Git repository.
- PostgreSQL object coverage remains limited to existing Slice 1 through Slice 8 scope.
- Reference-data compare remains compare-only.
- Direct database apply remains unavailable.
