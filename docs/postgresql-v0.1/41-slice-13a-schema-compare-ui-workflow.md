# Slice 13A Schema Compare UI Workflow

Slice 13A refactors the local browser UI shell into a schema compare workflow pattern.

The goal is to make the private beta UI feel like a database compare tool while preserving the DbState safety model:

- Repository desired state is the source side.
- PostgreSQL is inspected and compared read-only.
- Plans and release artifacts remain reviewable outputs.
- No direct database apply exists.
- No generated SQL execution exists.
- No database write workflows are exposed in this UI shell.

This slice is a corrective UI slice. It does not add connection profiles, backend database capability, object coverage, or release artifact writing from the UI. Slice 15 later adds a tightly gated Database to Repository repository-file write workflow.

## Workflow Model

The UI is organized as a left-navigation workflow:

1. Workspace
2. Source & Target
3. Compare Options
4. Results
5. Object Diff
6. Warnings
7. Release Plan
8. Reports / Raw JSON
9. About / Safety

Only the active workflow step is shown in the main content area.

## Workspace

The Workspace step keeps Slice 13 session-only repository selection:

- Local repository path input.
- Check workspace.
- Repo status.
- Init plan.
- Repository path, Git root, branch, working tree status, project status, and missing path count.

The workspace path is not persisted. DbState does not maintain a recent-project list, clone repositories, fetch repositories, pull, push, stage, or commit from the UI.

## Source And Target

The Source & Target step presents:

- Source: repository desired state from the selected workspace.
- Target: PostgreSQL database through a session-only URL, a saved non-secret profile plus optional session-only password, or `DBSTATE_POSTGRES_URL`.
- Workflow mode selector:
  - Repo state to PostgreSQL compare.
  - PostgreSQL inspect only.
  - Reference-data compare.
  - Database to Repository, added in Slice 15.

The PostgreSQL URL is sent only with the clicked operation. It is not stored by the UI, placed in the browser URL, or displayed in response panels. Slice 14 adds profile mode for non-secret connection metadata. The profile password field is session-only and is not persisted.

## Compare Options

The Compare Options step includes:

- Scope: all, schema, or table.
- Schema input.
- Table input.
- Include refs input.
- Exclude refs input.
- Reference-data scope and table input.
- Disabled future object-type filters for indexes, views, functions, triggers, and grants.

Buttons call only existing safe service endpoints:

- Inspect.
- Run Compare.
- Run Plan.
- Run Reference Data Compare.
- Preview Repository Sync, added in Slice 15 for Database to Repository.
- Write Repository Files, added in Slice 15 and gated by typed confirmation plus clean working tree.

## Results Grid

The Results step maps existing service responses into a best-effort grid. Slice 13B refines this grid so it behaves more like a database schema compare tool.

Columns:

- Include.
- Object type.
- Schema.
- Object name.
- Status.
- Planned operation.
- Warnings.

Source and target are shown as workflow context above the grid instead of repeated columns.

The include checkboxes are UI-only state in Slice 13A. They do not write files and do not call write endpoints.

The Slice 13B grid includes:

- Object-type filter.
- Compact status legend.
- Status badges.
- Clean schema and object names derived from object refs or repository file paths.
- Inspect rows for schemas, tables, and columns when available.
- Service errors shown as errors, not fake object rows.

## Object Diff

The Object Diff step shows the selected row:

- Object identity.
- Status.
- Planned operation.
- Warning count.
- Source detail when available.
- Target detail when available.
- Selected JSON item.

Detailed DDL diffing is not implemented in this slice. Unavailable details are labeled as not available yet.

## Warnings

The Warnings step shows:

- Service warnings.
- Service errors.
- Dependency warnings.
- Blocked items.
- Deferred object types.

## Release Plan

The Release Plan step is review-only.

It shows selected included objects from the UI results grid and explains that release artifact preview will be connected in a later safe write-gated slice.

The Slice 13A UI does not write release artifacts and does not expose a release write endpoint.

## Reports And Raw JSON

The Reports / Raw JSON step shows the latest service response summary and redacted JSON. Slice 15 adds Copy JSON, which copies the redacted JSON currently displayed.

The UI defensively redacts obvious PostgreSQL URLs, password markers, and token markers before display. The service remains responsible for response redaction.

## Service Endpoints

The UI calls only:

```text
GET  /api/v1/health
POST /api/v1/repo/status
POST /api/v1/init/plan
POST /api/v1/postgres/inspect
POST /api/v1/postgres/compare
POST /api/v1/postgres/plan
POST /api/v1/postgres/data-compare
POST /api/v1/postgres/repository-sync/preview
POST /api/v1/postgres/repository-sync/write
```

The repository-sync write endpoint writes only repository object files after explicit confirmation. It does not mutate PostgreSQL, execute SQL, or write release artifacts.

## Technology Boundary

The UI remains static embedded assets:

- HTML.
- CSS.
- Plain JavaScript.

No React, Vue, Svelte, Angular, Vite, npm, Node, package file, external CDN, external fonts, or external scripts are added.

## Safety Boundary

Slice 13A does not add:

- Password, token, or full PostgreSQL URL persistence.
- Project database or workspace database.
- Recent projects list.
- Export write workflow in UI.
- Sync write workflow in UI.
- Release artifact write workflow in UI.
- Direct database apply.
- Generated SQL execution.
- Database mutation.
- Docker Compose.
- CI workflow.
- MCP server.
- AI integration.

Do not expose the local service publicly.

Do not use production, UAT, staging, or shared databases for UI tests.

## Validation

Run:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
docker build -t dbstate-postgres:dev .
```

Manual UI smoke:

```powershell
cargo run -- serve --host 127.0.0.1 --port 4587
```

Open:

```text
http://127.0.0.1:4587/
```

Confirm the workflow navigation is visible, active steps switch the main content, workspace selection still works, compare options show scope controls, results show a grid, object diff and warnings steps exist, release plan is review-only, and raw JSON remains available.
