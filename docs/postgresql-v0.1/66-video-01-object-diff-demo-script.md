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

## Verified Non-Secret Profile Values

The script defaults match the non-secret `Pagila-Local` connection profile:

- host: `localhost`
- port: `5433`
- database: `pagila`
- username: `exitpass`
- SSL mode: `disable`

The password is not stored in the profile and is not documented here. It must be supplied at runtime through `DBSTATE_DEMO_PG_PASSWORD`.

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

- `DBSTATE_DEMO_PG_HOST`
- `DBSTATE_DEMO_PG_PORT`
- `DBSTATE_DEMO_PG_DATABASE`
- `DBSTATE_DEMO_PG_USERNAME`
- `DBSTATE_DEMO_PG_PASSWORD`
- `DBSTATE_DEMO_PSQL_PATH`

Defaults:

- `DBSTATE_DEMO_PG_HOST`: `localhost`
- `DBSTATE_DEMO_PG_PORT`: `5433`
- `DBSTATE_DEMO_PG_DATABASE`: `pagila`
- `DBSTATE_DEMO_PG_USERNAME`: `exitpass`
- `DBSTATE_DEMO_PSQL_PATH`: `psql` on `PATH`

`DBSTATE_DEMO_PG_PASSWORD` has no default and is required. The script passes it to `psql` through process environment variable `PGPASSWORD`, not through command-line arguments.

## Local Allowlist

`Apply` and `Reset` are refused unless:

- host is exactly `localhost` or `127.0.0.1`
- database is exactly `pagila`
- target table is fixed as `public.country`
- target column is fixed as `iso_code`

This is local-demo-only. It must not be used for production, staging, UAT, regulated, shared, or customer databases.

## Output Contract

Successful operations and controlled reset no-op results emit one compact JSON object to standard output.

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
- `PsqlUnavailable`
- `UnsafeHost`
- `UnexpectedDatabase`
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
- `psql` unavailable
- unsafe host
- unexpected database
- connection failure
- `public.country` missing
- `Apply` requested when `iso_code` already exists
- SQL execution failure
- post-change verification failure
- malformed or unexpected database state

The script does not silently repair unexpected state.

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
