# Video 01 Object Diff Demo Script

This note documents the controlled PowerShell script used by Cycle 3 of **DbState Demo: Capture and Review PostgreSQL Schema Changes in Git**.

The PostgreSQL table change is performed outside DbState to simulate an ordinary schema change. DbState does not execute or apply this change.

## Script

```text
demo/video-01/scripts/invoke-pagila-country-object-diff-change.ps1
```

The script supports only:

```powershell
-Action Status
-Action Apply
-Action Reset
```

It does not accept arbitrary SQL, schema names, table names, column names, data types, connection strings, or remote database targets.

## Docker Demo Environment

Docker is the canonical Video 1 PostgreSQL execution environment.

Verified non-secret values:

- container: `exitpass-postgres`
- image: `postgres:16-alpine`
- host port: `5433`
- container port: `5432`
- database: `pagila`
- username: `exitpass`
- saved DbState profile: `Pagila-Local`
- SSL mode: `disable`

The password is not documented here. It must be supplied at runtime through `DBSTATE_DEMO_PG_PASSWORD`.

The demo credential flow is:

```text
Darwin launches the demo runner
-> launcher securely prompts for the Pagila-Local password
-> launcher places it in DBSTATE_DEMO_PG_PASSWORD for the current process only
-> Playwright inherits the environment variable
-> Playwright visibly enters the password into DbState's session-only password field
-> this Cycle 3 script inherits the same environment variable
-> the launcher removes the environment variable after the workflow finishes
```

## Target Change

The target is fixed:

- table: `public.country`
- column: `iso_code`
- type: nullable `varchar(2)`

Apply executes only:

```sql
ALTER TABLE public.country
ADD COLUMN iso_code varchar(2);
```

Reset executes only:

```sql
ALTER TABLE public.country
DROP COLUMN iso_code;
```

No default value, `NOT NULL`, index, constraint, data update, additional column, or additional table change is included.

## Environment Variables

The script reads:

- `DBSTATE_DEMO_PG_EXECUTION_MODE`
- `DBSTATE_DEMO_PG_CONTAINER`
- `DBSTATE_DEMO_PG_HOST`
- `DBSTATE_DEMO_PG_PORT`
- `DBSTATE_DEMO_PG_DATABASE`
- `DBSTATE_DEMO_PG_USERNAME`
- `DBSTATE_DEMO_PG_PASSWORD`
- `DBSTATE_DEMO_PSQL_PATH`

Docker-mode defaults:

- `DBSTATE_DEMO_PG_EXECUTION_MODE`: `docker`
- `DBSTATE_DEMO_PG_CONTAINER`: `exitpass-postgres`
- `DBSTATE_DEMO_PG_HOST`: `127.0.0.1`
- `DBSTATE_DEMO_PG_PORT`: `5432`
- `DBSTATE_DEMO_PG_DATABASE`: `pagila`
- `DBSTATE_DEMO_PG_USERNAME`: `exitpass`

Optional local-mode defaults:

- `DBSTATE_DEMO_PG_EXECUTION_MODE`: `local`
- `DBSTATE_DEMO_PG_HOST`: `127.0.0.1` or `localhost`
- `DBSTATE_DEMO_PG_PORT`: `5433`
- `DBSTATE_DEMO_PG_DATABASE`: `pagila`
- `DBSTATE_DEMO_PG_USERNAME`: `exitpass`
- `DBSTATE_DEMO_PSQL_PATH`: `psql` on `PATH`, unless explicitly set

`DBSTATE_DEMO_PG_PASSWORD` has no default and is required. The script passes it to `psql` through process environment variable `PGPASSWORD`, not through command-line arguments.

In Docker mode, the password is forwarded by inherited environment variable name with `--env PGPASSWORD`. The script does not pass the password as a Docker argument value.

## Docker Execution

Docker mode runs `psql` inside the approved container:

```text
docker exec -i --env PGPASSWORD exitpass-postgres psql ...
```

The script verifies:

- `docker` is available
- Docker engine can be reached
- `exitpass-postgres` exists
- `exitpass-postgres` is running
- `psql` exists inside the container
- container name is exactly `exitpass-postgres`
- host is local
- internal PostgreSQL port is exactly `5432`
- database is exactly `pagila`
- username is exactly `exitpass`

The script does not select alternate containers, inspect unrelated PostgreSQL containers, or require a temporary wrapper script.

## Local Allowlist

Local execution mode is retained for development portability. It is not the canonical Video 1 path.

Local mode is refused unless:

- host is exactly `localhost` or `127.0.0.1`
- port is exactly `5433`
- database is exactly `pagila`
- username is exactly `exitpass`
- target table is fixed as `public.country`
- target column is fixed as `iso_code`

This script is local-demo-only. It must not be used for production, staging, UAT, regulated, shared, or customer databases.

## State Query Contract

The script verifies `public.country` separately before reading column metadata. A missing table is a controlled `TableMissing` failure, not an absent-column state.

The column metadata query returns exactly one row and four fields:

```text
columnExists|dataType|maximumLength|nullable
```

Expected absent-column state:

```text
false|||
```

Expected present-column state:

```text
true|character varying|2|YES
```

The parser accepts CRLF or LF output, ignores blank surrounding lines, and requires exactly one non-empty data row. It rejects missing fields, extra fields, invalid booleans, invalid maximum length values, invalid nullability values, multiple rows, empty output, and unexpected metadata for an absent column.

Docker diagnostics on standard error do not corrupt state parsing because process exit code, standard output, and standard error are captured separately.

## Output Contract

Successful operations, controlled reset no-op results, and controlled failures emit one compact JSON object to standard output.

Fields:

- `action`
- `status`
- `database`
- `table`
- `column`
- `columnExists`
- `dataType`
- `maximumLength`
- `nullable`
- `changed`

Stable success/no-op statuses:

- `Ready`
- `Applied`
- `Reset`
- `AlreadyReset`

Controlled failure statuses include:

- `MissingPassword`
- `DockerUnavailable`
- `ContainerMissing`
- `ContainerStopped`
- `ContainerPsqlUnavailable`
- `UnsafeContainer`
- `UnsupportedExecutionMode`
- `PsqlUnavailable`
- `UnsafeHost`
- `UnexpectedPort`
- `UnexpectedDatabase`
- `UnexpectedUsername`
- `ConnectionFailed`
- `TableMissing`
- `AlreadyApplied`
- `VerificationFailed`
- `MalformedState`

Diagnostics may be written to standard error, but they must not contain credentials or a full connection string.

## Exit Behavior

Exit `0`:

- `Status` completed successfully
- `Apply` added the column and post-change verification passed
- `Reset` removed the column and post-change verification passed
- `Reset` found the column already absent and returned `AlreadyReset`

Nonzero:

- missing required password
- Docker unavailable or Docker engine unreachable
- approved container missing or stopped
- `psql` unavailable in the container or local environment
- unsafe host, port, container, database, or username
- unsupported execution mode
- connection failure
- `public.country` missing
- `Apply` requested when `iso_code` already exists
- SQL execution failure
- post-change verification failure
- malformed or unexpected database state

The script does not silently repair unexpected state.

## Validation Sequence

Run the full Docker-backed lifecycle against the local demo container:

```text
Status, Apply, Status, repeated Apply, Reset, Status, repeated Reset
```

Expected results:

| Step | Expected status | Exit code |
| --- | --- | ---: |
| Initial Status | `Ready`, column absent | 0 |
| Apply | `Applied`, column present | 0 |
| Status | `Ready`, `character varying`, `2`, nullable | 0 |
| Repeated Apply | `AlreadyApplied` controlled failure | nonzero |
| Reset | `Reset`, column absent | 0 |
| Status | `Ready`, column absent | 0 |
| Repeated Reset | `AlreadyReset`, unchanged | 0 |

The final validation state must have `public.country.iso_code` absent.

## Expected DbState Cycle 3 Behavior

Before `Apply`:

- initial schema capture is committed
- working tree is clean
- `database/objects/tables/public.country.sql` exists
- repository SQL does not contain `iso_code`
- PostgreSQL `public.country` does not contain `iso_code`

After `Apply`:

- PostgreSQL `public.country` contains nullable `iso_code varchar(2)`
- repository `public.country.sql` remains unchanged
- Git working tree remains clean
- DbState has not written repository files
- no Git operation has occurred

After Preview Repository Sync:

- `public.country` is classified as different
- Results can select `public.country`
- Object Diff shows database/source DDL containing `iso_code`
- Object Diff shows repository/target DDL without `iso_code`
- preview remains read-only
- Git Workflow exposes a schema-change branch recommendation

After external branch change and repository write:

- Ungit creates the exact branch recommended by Git Workflow
- DbState refreshes and confirms the branch is non-protected
- DbState writes only the selected updated repository file
- Repository Files shows `iso_code` in `public.country.sql`
- Git Workflow recommends the updated file and commit text
- Ungit stages and commits exactly those recommendations

## Safety Boundary

The script:

- operates only on the approved local Pagila demo database
- rejects remote hosts
- rejects unexpected database names
- rejects unexpected usernames
- rejects unexpected Docker containers
- accepts no arbitrary SQL
- modifies only `public.country.iso_code`
- never prints credentials
- never modifies repository files
- never runs Git commands
- never starts DbState
- never generates repository SQL files
- never performs schema capture
- never stages or commits anything
- never changes another table
- never changes another column
- never modifies data rows
