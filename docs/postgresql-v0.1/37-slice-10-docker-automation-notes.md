# Slice 10 Docker Automation Notes

Slice 10 adds Docker packaging for headless DbState PostgreSQL CLI automation.

The Docker image runs the existing `dbstate` binary. In Slice 10 it was CLI-only. After Slice 11, the same image can also run the minimal local Service API boundary. After Slice 12, service mode also serves the static local browser UI shell. It is not a full browser UI product, Docker Compose stack, CI workflow, MCP server, AI integration, Deployment Rehearsal, or deployment engine.

Docker does not change DbState safety boundaries. DbState still does not execute generated SQL, apply database changes, mutate PostgreSQL, or hide deployment behavior.

## Build The Image

From the repository root:

```powershell
docker build -t dbstate-postgres:dev .
```

The Dockerfile uses a multi-stage build:

- `rust:1-bookworm` builds the release binary.
- `debian:bookworm-slim` runs the final CLI image.
- The final image contains the `dbstate` binary on `PATH`.
- The final image includes the Git client because DbState repository commands call Git.
- The final image marks `/workspace` as a Git safe directory for mounted repository use.
- The final image uses `/workspace` as the default working directory.
- The final image runs as a non-root `dbstate` user.

No credentials, connection strings, generated artifacts, local user paths, or test passwords are baked into the image.

## Run Help

PowerShell:

```powershell
docker run --rm dbstate-postgres:dev dbstate --help
```

bash or zsh:

```bash
docker run --rm dbstate-postgres:dev dbstate --help
```

## Mount A Repository

DbState commands operate against the current working directory inside the container. Mount the project repository at `/workspace`.

PowerShell:

```powershell
docker run --rm `
  -v "${PWD}:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate repo status --format json
```

bash or zsh:

```bash
docker run --rm \
  -v "$PWD:/workspace" \
  -w /workspace \
  dbstate-postgres:dev \
  dbstate repo status --format json
```

Dry-run initialization:

```powershell
docker run --rm `
  -v "${PWD}:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate init --dry-run --format json
```

## Workspace Path Selection In Service Mode

After Slice 13, the Service API and browser UI can accept a session-only `repositoryPath` field. In Docker, that path must be a path inside the container, not the host path. If the host repository is mounted at `/workspace`, use:

```text
/workspace
```

Additional host repositories must be mounted explicitly. The service cannot access arbitrary host paths that were not mounted into the container. DbState does not persist workspace paths, maintain a recent-project list, clone repositories, fetch, pull, push, stage, or commit from service endpoints.

## PostgreSQL Connection URL

Use the session-only `DBSTATE_POSTGRES_URL` environment variable. Do not bake credentials into the image or commit them to the repository.

PowerShell:

```powershell
$env:DBSTATE_POSTGRES_URL = "postgres://user:password@host.docker.internal:5432/disposable_test_db"
docker run --rm `
  -v "${PWD}:/workspace" `
  -w /workspace `
  -e DBSTATE_POSTGRES_URL `
  dbstate-postgres:dev `
  dbstate inspect postgres --format json
```

bash or zsh:

```bash
export DBSTATE_POSTGRES_URL="postgres://user:password@host.docker.internal:5432/disposable_test_db"
docker run --rm \
  -v "$PWD:/workspace" \
  -w /workspace \
  -e DBSTATE_POSTGRES_URL \
  dbstate-postgres:dev \
  dbstate inspect postgres --format json
```

The raw connection URL is not persisted and must not appear in text output, JSON output, generated files, or release artifacts.

## Connection Profiles In Docker

After Slice 14, the Service API and browser UI can use optional local non-secret connection profiles.

Profiles inside Docker are stored in the container filesystem unless a config directory is mounted. For persistent Docker profiles, mount a config directory and set `DBSTATE_CONFIG_DIR`.

PowerShell:

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

Profiles store host, port, database, username, SSL mode, and description only. Passwords, tokens, and full PostgreSQL URLs must remain session-only and must not be baked into images.

## Docker Networking

Connecting from a container to PostgreSQL depends on host and Docker networking.

For Windows and macOS Docker Desktop, `host.docker.internal` usually points from the container to the host machine:

```text
postgres://user:password@host.docker.internal:5432/disposable_test_db
```

For Linux, users may need one of these approaches:

- A user-defined Docker network and a PostgreSQL container hostname.
- Host networking where appropriate for local testing.
- The host gateway IP.
- An explicit database container name on a shared Docker network.

Slice 10 does not add Docker Compose or networking scripts.

## Supported CLI Surface

The Docker image exposes the same CLI surface as the native binary:

- `dbstate repo status`
- `dbstate init`
- `dbstate inspect postgres`
- `dbstate export postgres`
- `dbstate sync postgres`
- `dbstate compare postgres`
- `dbstate plan postgres`
- `dbstate release postgres`
- `dbstate data-compare postgres`
- `dbstate serve`

Representative JSON commands:

```powershell
docker run --rm -v "${PWD}:/workspace" -w /workspace dbstate-postgres:dev dbstate repo status --format json
docker run --rm -v "${PWD}:/workspace" -w /workspace dbstate-postgres:dev dbstate init --dry-run --format json
docker run --rm -v "${PWD}:/workspace" -w /workspace -e DBSTATE_POSTGRES_URL dbstate-postgres:dev dbstate inspect postgres --format json
docker run --rm -v "${PWD}:/workspace" -w /workspace -e DBSTATE_POSTGRES_URL dbstate-postgres:dev dbstate compare postgres --all --format json
docker run --rm -v "${PWD}:/workspace" -w /workspace -e DBSTATE_POSTGRES_URL dbstate-postgres:dev dbstate plan postgres --all --format json
docker run --rm -v "${PWD}:/workspace" -w /workspace -e DBSTATE_POSTGRES_URL dbstate-postgres:dev dbstate release postgres --all --name docker_test --dry-run --format json
docker run --rm -v "${PWD}:/workspace" -w /workspace -e DBSTATE_POSTGRES_URL dbstate-postgres:dev dbstate data-compare postgres --all --format json
```

## Service Mode

Native service mode binds to `127.0.0.1` by default. Inside Docker, bind to `0.0.0.0` in the container only when publishing the host port to a local host address.

PowerShell:

```powershell
docker run --rm `
  -p 127.0.0.1:4587:4587 `
  -v "${PWD}:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate serve --host 0.0.0.0 --port 4587
```

Host smoke check:

```powershell
Invoke-RestMethod http://127.0.0.1:4587/health
```

bash or zsh:

```bash
docker run --rm \
  -p 127.0.0.1:4587:4587 \
  -v "$PWD:/workspace" \
  -w /workspace \
  dbstate-postgres:dev \
  dbstate serve --host 0.0.0.0 --port 4587
```

The published port should stay bound to `127.0.0.1` on the host. Do not expose the local service publicly.

Open the Slice 12 UI shell:

```text
http://127.0.0.1:4587/
```

The UI is static and served by the local service. It does not add write workflows, direct database apply, SQL execution, or database mutation behavior.

## Write Behavior

Docker does not make read-only commands write files.

Read-only commands:

- `dbstate repo status`
- `dbstate inspect postgres`
- `dbstate compare postgres`
- `dbstate plan postgres`
- `dbstate data-compare postgres`

Commands that may write to the mounted repository only when explicitly invoked:

- `dbstate init`
- `dbstate export postgres`
- `dbstate sync postgres`
- `dbstate release postgres`

Object files may be written only under `database/objects/` by existing export and sync behavior.

Release artifacts may be written only under `database/releases/` by the explicit release command.

DbState does not stage, commit, push, execute release SQL, or apply changes to a database.

## Line Endings And Permissions

The Docker image runs Linux.

- Files generated inside the container may use Linux line endings.
- File ownership or permissions may differ on Linux hosts.
- Windows Docker Desktop usually maps mounted files to the Windows user.
- If permission issues occur, use Docker user flags or adjust host directory permissions.

Slice 10 does not add complex permission handling.

## Disposable PostgreSQL Validation

The following local validation uses a disposable PostgreSQL container. Do not point examples at production, UAT, staging, or any shared database.

PowerShell:

```powershell
docker network create dbstate-slice10-net
docker rm -f dbstate-slice10-pg 2>$null

docker run --name dbstate-slice10-pg `
  --network dbstate-slice10-net `
  -e POSTGRES_PASSWORD=dbstate_test_only `
  -e POSTGRES_DB=dbstate_slice10_test `
  -d postgres:16

do {
  Start-Sleep -Seconds 1
  docker exec dbstate-slice10-pg pg_isready -U postgres -d dbstate_slice10_test
} until ($LASTEXITCODE -eq 0)

Get-Content tests\fixtures\postgresql\slice2-basic.sql |
  docker exec -i dbstate-slice10-pg psql -U postgres -d dbstate_slice10_test

docker run --rm `
  --network dbstate-slice10-net `
  -v "${PWD}:/workspace" `
  -w /workspace `
  -e DBSTATE_POSTGRES_URL="postgres://postgres:dbstate_test_only@dbstate-slice10-pg:5432/dbstate_slice10_test" `
  dbstate-postgres:dev `
  dbstate inspect postgres --format json

docker rm -f dbstate-slice10-pg
docker network rm dbstate-slice10-net
```

bash or zsh:

```bash
docker network create dbstate-slice10-net
docker rm -f dbstate-slice10-pg 2>/dev/null || true

docker run --name dbstate-slice10-pg \
  --network dbstate-slice10-net \
  -e POSTGRES_PASSWORD=dbstate_test_only \
  -e POSTGRES_DB=dbstate_slice10_test \
  -d postgres:16

until docker exec dbstate-slice10-pg pg_isready -U postgres -d dbstate_slice10_test; do
  sleep 1
done

docker exec -i dbstate-slice10-pg psql -U postgres -d dbstate_slice10_test < tests/fixtures/postgresql/slice2-basic.sql

docker run --rm \
  --network dbstate-slice10-net \
  -v "$PWD:/workspace" \
  -w /workspace \
  -e DBSTATE_POSTGRES_URL="postgres://postgres:dbstate_test_only@dbstate-slice10-pg:5432/dbstate_slice10_test" \
  dbstate-postgres:dev \
  dbstate inspect postgres --format json

docker rm -f dbstate-slice10-pg
docker network rm dbstate-slice10-net
```

## Current Limitations

- Docker packages the CLI, the minimal local Service API boundary, and the static browser UI shell.
- No full browser UI product, hosted service mode, or multi-user server mode is included.
- No Docker Compose file is provided.
- No CI workflow is provided.
- No Kubernetes deployment is provided.
- No generated SQL is executed by DbState.
- No direct database apply exists.
- PostgreSQL object coverage remains limited to existing Slice 1 through Slice 12 behavior.
- `--repo <path>` remains an open decision. Current behavior uses the mounted working directory.
