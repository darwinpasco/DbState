# Slice 13B Results Grid Usability

Slice 13B refines the browser UI Results grid from the Slice 13A schema compare workflow shell.

This is a UI-only usability slice. It does not add backend database capability, service write endpoints, connection profiles, SQL execution, direct database apply, database mutation, or new PostgreSQL object coverage.

## Results Grid Changes

The Results step now shows source and target context above the table:

- Source.
- Target.
- Operation.

The grid columns are:

- Include.
- Object type.
- Schema.
- Object name.
- Status.
- Planned operation.
- Warnings.

Source and Target are no longer repeated as table columns.

## Object Type Mapping

The UI maps known response items into concrete object types:

- `schema`
- `table`
- `referenceDataTable`
- `referenceDataRow`
- `unknown` only as a fallback

Repository object file paths are cleaned before display:

- `database/objects/schemas/core.sql` becomes object type `schema`, schema `core`, object name `core`.
- `database/objects/tables/core.parking_sessions.sql` becomes object type `table`, schema `core`, object name `parking_sessions`.
- `database/objects/tables/parking_sessions.sql` becomes object type `table`, blank schema, object name `parking_sessions`.

The Schema column should contain only schema names, not repository paths. The Object name column should not show `.sql` suffixes.

## Inspect Results

Successful inspect responses now produce result rows where available:

- Schema rows map to `schema`.
- Table rows map to `table`.

Inspect rows use status `inspected` and no planned operation.

Columns from inspect responses are intentionally not shown as top-level Results rows. When a table row is selected, available column details appear in Object Diff. This keeps the main grid focused on schema and table objects while still exposing table detail.

## Object Type Filter

The Results step includes a client-side Object type filter.

Values:

- All.
- Schema.
- Table.
- Reference data, only when the workflow mode or latest response is Reference-data compare.
- Unknown or Skipped, only when the latest rows include unknown or skipped items.

Filtering does not call the service again. It preserves the latest response, raw JSON, and include checkbox state.

The filter does not include Column. Columns are table details in this UI shell, not top-level compare rows.

## Compare Options

Schema and Table are dropdown controls.

- Before Inspect runs, both dropdowns contain only All and the UI shows guidance to run Inspect first.
- After a successful Inspect response, Schema is populated from inspected schemas.
- After a successful Inspect response, Table is populated from inspected tables.
- If Schema is All, Table lists schema-qualified table names.
- If a specific Schema is selected, Table lists tables under that schema.

Request mapping:

- Schema All and Table All maps to scope `all`.
- Specific Schema and Table All maps to scope `schema`.
- Any specific Table maps to scope `table`.

Include refs and Exclude refs remain optional advanced text fields. They are empty by default and the UI shows only format examples as helper text.

Reference-data table selection is a dropdown. It is populated from the latest Reference-data compare response when configured table results are available. It is not prefilled with sample values.

## Status Badges And Legend

Statuses are shown as text badges with color coding:

- `inSync`: repository and database match.
- `repoDifferent`: object exists in both but differs.
- `repoOnly`: object exists only in repository desired state.
- `databaseOnly`: object exists only in PostgreSQL target.
- `skipped`: object was found but is not safely comparable.
- `blocked`: operation is blocked by safety or dependency rules.
- `error`: operation failed or returned errors.
- `inspected`: object was read from PostgreSQL inspect output.

The text label remains visible so color is not the only signal.

## Service Errors

Service errors are not converted into fake object rows.

If a response fails and contains errors:

- The Results grid shows zero object rows unless real object rows are present.
- The error appears above the table.
- The Warnings step shows the error.
- Reports / Raw JSON still shows the redacted service response.
- Object Diff remains empty until a real object row is selected.

If Compare fails because the selected workspace is a Git repository but not yet an initialized DbState project, the Results error summary and Warnings panel add this guidance:

```text
This workspace is a Git repository but not yet an initialized DbState project. Run dbstate init from this workspace, then export or sync desired state before comparing.
```

The raw JSON remains available in Reports / Raw JSON, but the error is not shown as an object row.

## Safety Boundary

Slice 13B preserves the existing UI safety boundary:

- UI calls only approved safe service endpoints.
- Workspace path remains session-only.
- PostgreSQL URL remains session-only.
- No write buttons are added.
- No export write workflow is added.
- No sync write workflow is added.
- No release artifact write workflow is added.
- No direct database apply exists.
- No generated SQL execution exists.
- No database mutation exists.

No React, Vue, Svelte, Angular, Vite, npm, Node, package file, external CDN, external fonts, or external scripts are added.

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

Confirm Inspect produces schema and table rows where available, table Object Diff shows columns where available, Source and Target appear above the grid, Source and Target are not grid columns, status badges and legend are visible, the Object type filter works client-side, and service errors appear as errors rather than object rows.
