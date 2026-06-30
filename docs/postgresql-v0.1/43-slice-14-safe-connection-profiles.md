# Slice 14 Safe Connection Profiles

Slice 14 adds safer local connection handling for the Service API and browser UI.

This slice does not add database apply, SQL execution, database mutation, write workflows, new PostgreSQL object coverage, a project database, a workspace database, cloud sync, keychain integration, or a secret vault.

## Purpose

The goal is to make private beta testing easier without storing secrets.

DbState now supports:

- Session-only PostgreSQL URL requests.
- Session-only `DBSTATE_POSTGRES_URL`.
- Optional local non-secret PostgreSQL connection profiles.
- Session-only password values for profile-based requests.
- A read-only connection test endpoint.

## Connection Precedence

PostgreSQL service operations resolve connection input in this order:

1. Request-level `postgresUrl`.
2. Request-level `connection.profileName` plus optional session-only `connection.password`.
3. Service process `DBSTATE_POSTGRES_URL`.

The CLI behavior remains unchanged. CLI commands still use `--url` or `DBSTATE_POSTGRES_URL`.

Slice 15 Database to Repository service endpoints use the same precedence. Session URL and profile password values remain request-only and are not persisted when previewing or writing repository object files.

## Profile Storage

Profiles are stored outside the repository.

Default locations:

- Windows: `%APPDATA%\DbState\connection-profiles.json`
- Linux: `~/.config/dbstate/connection-profiles.json`
- macOS: `~/Library/Application Support/DbState/connection-profiles.json`

Tests and Docker can override the config directory with:

```text
DBSTATE_CONFIG_DIR
```

The config directory is created only when saving a profile. Read-only operations do not create profile files.

## Profile Fields

Profiles may store only non-secret metadata:

```json
{
  "version": 1,
  "profiles": [
    {
      "name": "exitpass-local",
      "host": "localhost",
      "port": 5433,
      "database": "exitpass_v12_dev",
      "username": "postgres",
      "sslMode": "disable",
      "description": "Local ExitPass development database"
    }
  ]
}
```

Profiles must not store:

- Passwords.
- Tokens.
- Full PostgreSQL URLs.
- Connection strings.
- Secrets.
- Certificate private keys.
- `.pgpass` contents.
- Environment variable values.

DbState rejects profile input containing fields such as `password`, `token`, `secret`, `url`, `uri`, `connectionString`, or `connectionUrl`.

## Service Endpoints

Slice 14 adds JSON-only local service endpoints:

```text
GET    /api/v1/connections/profiles
POST   /api/v1/connections/profiles
PUT    /api/v1/connections/profiles/{name}
DELETE /api/v1/connections/profiles/{name}
POST   /api/v1/connections/test
```

The connection test endpoint accepts:

```json
{}
```

```json
{
  "postgresUrl": "postgres://user:password@localhost:5433/disposable_test_db"
}
```

```json
{
  "connection": {
    "profileName": "exitpass-local",
    "password": "session-only-password"
  }
}
```

The connection test runs only a read-only connectivity query:

```sql
SELECT current_database(), current_user
```

DbState does not execute generated SQL, DDL, DML, migration SQL, release SQL, or apply operations.

## Browser UI

The Source & Target step now supports three connection modes:

- Use session URL.
- Use saved profile.
- Use service environment variable.

Session URL mode sends the URL only with the clicked operation.

Saved profile mode uses non-secret profile metadata and an optional session-only password. The password is not saved, not logged, not placed in browser storage, not returned in service responses, and defensively redacted from Raw JSON.

Environment variable mode sends no connection data. The service uses `DBSTATE_POSTGRES_URL`.

The UI includes non-secret profile create, update, delete, refresh, and connection test controls. It does not include remember password, save password, keychain, vault, write workflows, export write, sync write, release write, direct apply, or SQL execution.

## Docker Guidance

Inside Docker, profiles are stored in the container filesystem unless a config directory is mounted.

PowerShell example:

```powershell
docker run --rm `
  -p 127.0.0.1:4587:4587 `
  -v "${PWD}:/workspace" `
  -v "$env:APPDATA\DbState:/config/dbstate" `
  -e DBSTATE_CONFIG_DIR=/config/dbstate `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate serve --host 0.0.0.0 --port 4587
```

Do not bake credentials into Docker images. Continue to use `DBSTATE_POSTGRES_URL` or session-only UI input for secrets.

## Safety Boundary

Slice 14 preserves the DbState safety model:

- No password persistence.
- No full URL persistence.
- No project database.
- No workspace database.
- No recent projects list.
- No cloud sync.
- No direct database apply.
- No generated SQL execution.
- No database mutation.
- No UI write workflow.

Do not use production, UAT, staging, or shared databases for tests unless you intentionally approve read-only inspection or compare access.

## Validation

Run:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
docker build -t dbstate-postgres:dev .
```

Manual service/UI smoke:

```powershell
cargo run -- serve --host 127.0.0.1 --port 4587
```

Open:

```text
http://127.0.0.1:4587/
```

Confirm profile creation writes only non-secret metadata to the config file and Raw JSON never exposes passwords or raw PostgreSQL URLs.
