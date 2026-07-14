const UI_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>DbState PostgreSQL v0.1</title>
  <link rel="stylesheet" href="/ui/app.css?v=slice22-beta-ui">
</head>
<body>
  <header class="app-header">
    <div class="brand-block">
      <h1>DbState PostgreSQL v0.1</h1>
      <p>Schema compare workflow shell over the local DbState Service API.</p>
    </div>
    <div class="header-status">
      <span class="status-pill" id="service-pill">Service not checked</span>
      <span class="status-pill local-only">Local only</span>
    </div>
  </header>

  <section class="safety-strip">
    <strong>Safety boundary:</strong>
    No SQL execution. No direct database apply. Controlled local repository/release file writes only after explicit typed confirmation.
  </section>

  <section class="context-strip" aria-label="Workspace summary">
    <span>Workspace: <strong id="header-workspace">service working directory</strong></span>
    <span>Branch: <strong id="header-branch">unknown</strong></span>
    <span>Working tree: <strong id="header-tree">unknown</strong></span>
  </section>

  <div class="app-layout">
    <nav class="workflow-nav" aria-label="Schema compare workflow">
      <button type="button" class="workflow-step active" data-step="workspace" data-testid="tab-workspace">1. Workspace</button>
      <button type="button" class="workflow-step" data-step="source-target" data-testid="tab-source-target">2. Source &amp; Target</button>
      <button type="button" class="workflow-step" data-step="compare-options" data-testid="tab-compare-options">3. Compare Options</button>
      <button type="button" class="workflow-step" data-step="results" data-testid="tab-results">4. Results</button>
      <button type="button" class="workflow-step" data-step="object-diff" data-testid="tab-object-diff">5. Object Diff</button>
      <button type="button" class="workflow-step" data-step="warnings" data-testid="tab-warnings">6. Warnings</button>
      <button type="button" class="workflow-step" data-step="release-plan" data-testid="tab-release-plan">7. Release Plan</button>
      <button type="button" class="workflow-step" data-step="reports" data-testid="tab-reports-raw-json">8. Reports / Raw JSON</button>
      <button type="button" class="workflow-step" data-step="about">9. About / Safety</button>
    </nav>

    <main class="workflow-main">
      <section class="workflow-panel active" id="step-workspace">
        <div class="panel-heading">
          <h2>Workspace</h2>
          <p>Select the local DbState project repository that the service can access.</p>
        </div>
        <div class="form-grid">
          <label for="workspace-path">Local repository path
            <input id="workspace-path" type="text" autocomplete="off" spellcheck="false" placeholder="Leave empty to use the service working directory">
          </label>
        </div>
        <p class="note">Session-only. The path is not persisted. DbState does not clone or fetch repositories. The service must already have filesystem access.</p>
        <div class="button-row">
          <button type="button" data-action="workspace-browse" data-testid="workspace-browse">Browse</button>
          <button type="button" data-action="health" data-testid="workspace-health">Health</button>
          <button type="button" data-action="workspace-status" data-testid="workspace-check">Check Workspace</button>
          <button type="button" data-action="repo-status" data-testid="workspace-repo-status">Repo Status</button>
          <button type="button" data-action="init-plan" data-testid="workspace-init-plan">Init Plan</button>
        </div>
        <div class="subsection">
          <h3>Initialize DbState Project</h3>
          <p class="note">This creates local DbState project folders/files only. It does not connect to PostgreSQL, execute SQL, mutate a database, or commit Git changes.</p>
          <label for="init-confirmation">Type INITIALIZE DBSTATE PROJECT
            <input id="init-confirmation" type="text" autocomplete="off" spellcheck="false">
          </label>
          <div class="button-row">
            <button type="button" data-action="init-write" data-testid="workspace-initialize-project" disabled>Initialize DbState Project</button>
          </div>
        </div>
        <div class="directory-picker" id="directory-picker" hidden>
          <div class="directory-picker-header">
            <h3>Select Workspace Folder</h3>
            <button type="button" class="secondary-button" data-action="directory-close">Cancel</button>
          </div>
          <p class="note">Browse folders visible to the DbState service process. In Docker, this means container paths such as /workspace.</p>
          <div class="directory-picker-error" id="directory-picker-error"></div>
          <div class="directory-picker-controls">
            <label for="directory-picker-path">Current path
              <input id="directory-picker-path" type="text" autocomplete="off" spellcheck="false">
            </label>
            <div class="button-row">
              <button type="button" data-action="directory-roots">Roots</button>
              <button type="button" data-action="directory-up">Up</button>
              <button type="button" data-action="directory-refresh">Refresh</button>
              <button type="button" data-action="directory-select">Select this folder</button>
            </div>
          </div>
          <div class="directory-root-list" id="directory-root-list"></div>
          <div class="directory-list" id="directory-list"></div>
        </div>
        <dl class="summary-list" id="workspace-summary"></dl>
      </section>

      <section class="workflow-panel" id="step-source-target">
        <div class="panel-heading">
          <h2>Source &amp; Target</h2>
          <p id="source-target-description">DbState compares repository desired state to a PostgreSQL database through read-only service operations.</p>
        </div>
        <div class="subsection">
          <h3>Workflow Mode</h3>
          <select id="workflow-mode" data-testid="workflow-mode">
            <option value="schemaRepoToDatabase">Schema Compare: Repository to Database</option>
            <option value="schemaDatabaseToRepository">Schema Compare: Database to Repository</option>
            <option value="referenceDataRepoToDatabase">Reference Data Compare: Repository to Database</option>
            <option value="referenceDataDatabaseToRepository">Reference Data Compare: Database to Repository</option>
          </select>
        </div>
        <div class="split-pane">
          <section class="subsection" id="source-panel">
            <h3>Source</h3>
            <p><strong id="source-kind">Repository desired state</strong> <span class="muted">(<span id="source-type">Repository</span>)</span></p>
            <div id="source-content">
              <div id="repository-context">
                <dl class="summary-list compact" id="source-summary">
                  <dt>Workspace</dt><dd id="repository-workspace">service working directory</dd>
                  <dt>Git root</dt><dd id="repository-git-root">unknown</dd>
                  <dt>Branch</dt><dd id="repository-branch">unknown</dd>
                  <dt>Working tree</dt><dd id="repository-tree">unknown</dd>
                  <dt>DbState project</dt><dd id="repository-project">unknown</dd>
                  <dt>Dirty</dt><dd id="repository-dirty">unknown</dd>
                </dl>
              </div>
            </div>
          </section>
          <section class="subsection" id="target-panel">
            <h3>Target</h3>
            <p><strong id="target-kind">PostgreSQL database</strong> <span class="muted">(<span id="target-type">Database</span>)</span></p>
            <div id="target-content">
              <div id="postgres-connection-context">
                <label for="connection-mode">Connection mode
                  <select id="connection-mode" data-testid="connection-mode">
                    <option value="sessionUrl">Use session URL</option>
                    <option value="profile">Use saved profile</option>
                    <option value="environment">Use service environment variable</option>
                  </select>
                </label>
                <div id="session-url-panel" data-testid="target-connection-input">
                  <label for="postgres-url">PostgreSQL session-only URL
                    <input id="postgres-url" data-testid="source-connection-input" type="password" autocomplete="off" spellcheck="false" placeholder="Prefer DBSTATE_POSTGRES_URL in the service environment">
                  </label>
                  <p class="note">The URL is sent only with the operation you click. It is not persisted, logged by the UI, or displayed in response panels.</p>
                </div>
                <div id="profile-panel" hidden>
                  <div class="form-grid">
                    <label for="profile-select">Saved profile
                      <select id="profile-select">
                        <option value="">No profile selected</option>
                      </select>
                    </label>
                    <label for="profile-password">Session-only password
                      <input id="profile-password" type="password" autocomplete="off" spellcheck="false">
                    </label>
                  </div>
                  <dl class="summary-list compact" id="profile-summary"></dl>
                  <details>
                    <summary>Manage non-secret profile</summary>
                    <div class="form-grid">
                      <label for="profile-name">Name
                        <input id="profile-name" type="text" autocomplete="off">
                      </label>
                      <label for="profile-host">Host
                        <input id="profile-host" type="text" autocomplete="off">
                      </label>
                      <label for="profile-port">Port
                        <input id="profile-port" type="number" min="1" max="65535" value="5432">
                      </label>
                      <label for="profile-database">Database
                        <input id="profile-database" type="text" autocomplete="off">
                      </label>
                      <label for="profile-username">Username
                        <input id="profile-username" type="text" autocomplete="off">
                      </label>
                      <label for="profile-sslmode">SSL mode
                        <select id="profile-sslmode">
                          <option value="disable">disable</option>
                          <option value="prefer" selected>prefer</option>
                          <option value="require">require</option>
                          <option value="verify-ca">verify-ca</option>
                          <option value="verify-full">verify-full</option>
                        </select>
                      </label>
                      <label for="profile-description">Description
                        <input id="profile-description" type="text" autocomplete="off">
                      </label>
                    </div>
                  </details>
                  <div class="button-row">
                    <button type="button" data-action="profiles-refresh">Refresh Profiles</button>
                    <button type="button" data-action="profile-save">Save Profile</button>
                    <button type="button" data-action="profile-delete">Delete Profile</button>
                    <button type="button" data-action="connection-test" data-testid="source-target-run">Test Connection</button>
                  </div>
                  <p class="note">Profiles store host, port, database, username, SSL mode, and description only. Passwords, tokens, and full URLs are never saved.</p>
                </div>
                <div id="environment-panel" hidden>
                  <p class="note">The service will use DBSTATE_POSTGRES_URL from its process environment. The UI sends no URL or password.</p>
                </div>
              </div>
              <div id="catalog-context" class="note" hidden>
                Read-only catalog view. PostgreSQL inspect reads schemas, tables, and columns without changing repository files or the database.
              </div>
            </div>
          </section>
        </div>
      </section>

      <section class="workflow-panel" id="step-compare-options">
        <div class="panel-heading">
          <h2>Compare Options</h2>
          <p>Choose the scope and run a read-only compare, plan, inspect, or configured reference-data compare.</p>
        </div>
        <div class="form-grid">
          <label for="compare-schema">Schema
            <select id="compare-schema">
              <option value="">All</option>
            </select>
          </label>
          <label for="compare-table">Table
            <select id="compare-table">
              <option value="">All</option>
            </select>
          </label>
          <label for="plan-include">Include refs
            <input id="plan-include" type="text" autocomplete="off" disabled>
          </label>
          <label for="plan-exclude">Exclude refs
            <input id="plan-exclude" type="text" autocomplete="off" disabled>
          </label>
          <label for="data-scope">Reference-data scope
            <select id="data-scope">
              <option value="selected">Selected configured tables</option>
              <option value="all">All configured tables</option>
            </select>
          </label>
          <label for="data-table">Reference-data table
            <select id="data-table">
              <option value="">No configured tables loaded</option>
            </select>
          </label>
        </div>
        <p class="note">Run Inspect first to populate schema and table lists.</p>
        <p class="note">Reference-data compare is read-only. DbState does not insert, update, delete, merge, or apply data changes. Only tables listed in <code>database/reference-data/dbstate.reference-data.yml</code> are reference-data tables. Masked columns remain masked in results.</p>
        <div class="object-filter-row" aria-label="Object type filters">
          <label><input type="checkbox" checked disabled> schemas</label>
          <label><input type="checkbox" checked disabled> tables</label>
          <label><input type="checkbox" disabled> indexes future</label>
          <label><input type="checkbox" disabled> views future</label>
          <label><input type="checkbox" checked disabled> materialized views</label>
          <label><input type="checkbox" checked disabled> functions</label>
          <label><input type="checkbox" checked disabled> triggers</label>
          <label><input type="checkbox" checked disabled> grants</label>
          <label><input type="checkbox" checked disabled> RLS policies</label>
        </div>
        <div class="button-row">
          <button type="button" data-action="inspect" data-standard-operation-action>Refresh Database Inventory</button>
          <button type="button" data-action="compare" data-standard-operation-action>Run Compare</button>
          <button type="button" data-action="plan" data-standard-operation-action>Run Plan</button>
          <button type="button" data-action="data-compare" data-standard-operation-action>Run Reference Data Compare</button>
        </div>
        <section class="subsection reference-data-setup-panel" id="reference-data-panel" data-testid="reference-data-compare-panel">
          <h3>Reference-Data Compare</h3>
          <p class="note">The repository registry is the source of truth. DbState does not infer or enroll reference-data tables automatically.</p>
          <div class="button-row">
            <button type="button" data-action="reference-data-status" data-testid="reference-data-status">Load Registry Status</button>
            <button type="button" data-action="reference-data-select-all" data-testid="reference-data-select-all">Select all</button>
            <button type="button" data-action="reference-data-clear-selection" data-testid="reference-data-clear-selection">Clear selection</button>
            <span id="reference-data-selected-count" data-testid="reference-data-selected-count">0 selected</span>
          </div>
          <dl class="summary-list compact" id="reference-data-registry-summary" data-testid="reference-data-registry-summary"></dl>
          <div class="table-wrap">
            <table class="results-grid" aria-label="Configured reference-data tables" data-testid="reference-data-configured-tables">
              <thead>
                <tr><th>Select</th><th>Schema</th><th>Table</th><th>Key columns</th><th>Ignored columns</th><th>Masked columns</th></tr>
              </thead>
              <tbody id="reference-data-configured-body">
                <tr><td colspan="6">Load registry status to show configured reference-data tables.</td></tr>
              </tbody>
            </table>
          </div>
          <details open>
            <summary>Setup guidance</summary>
            <p class="note">The registry file defines which tables are reference data. The table file contains expected rows. <code>key</code> identifies the row and <code>values</code> contains expected column values. <code>ignoredColumns</code> do not drive differences. <code>maskedColumns</code> are not exposed.</p>
            <p class="note">Registry file: <code>database/reference-data/dbstate.reference-data.yml</code></p>
            <pre id="reference-data-example-yaml" data-testid="reference-data-example-yaml">version: 1
tables:
  - schema: public
    name: country
    keyColumns:
      - country_id
    ignoredColumns:
      - last_update
    maskedColumns: []
</pre>
            <p class="note">Table row file: <code>database/reference-data/tables/public.country.yml</code></p>
            <p class="note"><strong>Canonical table file format</strong></p>
            <pre id="reference-data-table-example-yaml" data-testid="reference-data-table-example-yaml">version: 1
table: public.country
rows:
  - key:
      country_id: 1
    values:
      country_id: 1
      country: Afghanistan
      last_update: "2006-02-15 09:44:00+00"
</pre>
            <p class="note"><strong>Simplified supported format</strong> derives row keys from registry <code>keyColumns</code>.</p>
            <pre id="reference-data-table-flat-example-yaml" data-testid="reference-data-table-flat-example-yaml">version: 1
table: public.country
rows:
  - country_id: 1
    country: Afghanistan
    last_update: "2006-02-15 09:44:00+00"
            </pre>
          </details>
        </section>
        <section class="subsection" id="reference-data-export-panel" data-testid="reference-data-export-panel" hidden>
          <h3>Reference Data: Database to Repository</h3>
          <p class="note">Choose database tables that should be managed as reference data. DbState writes YAML files to the repository only. It does not modify PostgreSQL.</p>
          <div class="button-row">
            <button type="button" data-action="reference-data-load-database-tables" data-testid="reference-data-load-database-tables">Load database tables</button>
            <button type="button" data-action="reference-data-preview-yaml" data-testid="reference-data-preview-yaml">Preview Reference YAML</button>
            <button type="button" data-action="reference-data-write-yaml" data-testid="reference-data-write-yaml" disabled>Write Reference Data Files</button>
            <span id="reference-data-export-selected-count" data-testid="reference-data-export-selected-count">0 selected for export</span>
          </div>
          <label for="reference-data-export-search">Table search
            <input id="reference-data-export-search" type="text" autocomplete="off" placeholder="public.country">
          </label>
          <div class="table-wrap">
            <table class="results-grid" aria-label="Reference-data database tables" data-testid="reference-data-database-tables">
              <thead>
                <tr><th>Select</th><th>Schema</th><th>Table</th><th>Suggested key</th><th>Selected key count</th><th>Versioned columns</th><th>Masked columns</th><th>Rows</th></tr>
              </thead>
              <tbody id="reference-data-database-tables-body">
                <tr><td colspan="8">Load database tables to choose reference-data exports.</td></tr>
              </tbody>
            </table>
          </div>
          <section class="subsection" id="reference-data-export-detail" data-testid="reference-data-export-detail">
            <h4>Selected Table Detail</h4>
            <p class="note">Select key columns, versioned columns, and optional masked columns. Ignored columns are derived from columns not selected as key, versioned, or masked.</p>
            <dl class="summary-list compact" id="reference-data-export-detail-summary"></dl>
            <div class="table-wrap">
              <table class="results-grid" aria-label="Reference-data export columns" data-testid="reference-data-export-columns">
                <thead>
                  <tr><th>Column</th><th>Type</th><th>Nullable</th><th>PK</th><th>Unique</th><th>Key</th><th>Versioned</th><th>Masked</th></tr>
                </thead>
                <tbody id="reference-data-export-columns-body">
                  <tr><td colspan="8">Select a database table to configure columns.</td></tr>
                </tbody>
              </table>
            </div>
          </section>
          <label for="reference-data-write-confirmation">Typed confirmation
            <input id="reference-data-write-confirmation" type="text" autocomplete="off">
          </label>
          <p class="note">Type WRITE REFERENCE DATA FILES to enable repository YAML writes.</p>
          <dl class="summary-list compact" id="reference-data-export-summary" data-testid="reference-data-export-summary"></dl>
          <pre id="reference-data-export-preview" data-testid="reference-data-export-preview">Preview generated registry and table YAML here.</pre>
        </section>
        <div id="repository-sync-controls" class="subsection" hidden>
          <h3>Schema Compare: Database to Repository</h3>
          <p class="note">Preview reads PostgreSQL and the selected repository without writing files. Write repository changes writes only under the selected repository's database/objects/ paths, requires a clean working tree, and never changes PostgreSQL.</p>
          <label for="repository-write-confirmation">Typed confirmation
            <input id="repository-write-confirmation" type="text" autocomplete="off">
          </label>
          <p class="note">Type WRITE REPOSITORY FILES to enable the repository file write action.</p>
          <div class="button-row">
            <button type="button" data-action="repository-sync-preview" data-testid="preview-repository-sync">Preview Repository Sync</button>
            <button type="button" data-action="repository-sync-write" data-testid="write-repository-changes" disabled>Write Repository Changes</button>
          </div>
        </div>
      </section>

      <section class="workflow-panel" id="step-results">
        <div class="panel-heading">
          <h2>Results</h2>
          <p>Review comparison and planning results. Include selections are UI-only in this shell.</p>
        </div>
        <div class="results-context" aria-label="Results source and target context">
          <span>Source: <strong id="results-source">Repository desired state</strong></span>
          <span>Target: <strong id="results-target">PostgreSQL target</strong></span>
          <span>Operation: <strong id="results-operation">none</strong></span>
        </div>
        <div id="results-error-summary" class="results-error-summary" hidden></div>
        <div class="results-toolbar">
          <label for="object-type-filter">Object type
            <select id="object-type-filter" data-testid="results-object-type-filter">
              <option value="all">All</option>
              <option value="schema">Schema</option>
              <option value="table">Table</option>
            </select>
          </label>
          <label for="status-filter">Status
            <select id="status-filter" data-testid="results-status-filter">
              <option value="all">All</option>
              <option value="repoDifferent">repoDifferent</option>
              <option value="inSync">inSync</option>
              <option value="repoOnly">repoOnly</option>
              <option value="databaseOnly">databaseOnly</option>
              <option value="plannedCreate">plannedCreate</option>
              <option value="plannedUpdate">plannedUpdate</option>
              <option value="blocked">blocked</option>
              <option value="error">error</option>
              <option value="skipped">skipped</option>
              <option value="inspected">inspected</option>
            </select>
          </label>
          <span id="results-count" data-testid="results-visible-row-count">0 result rows</span>
          <span id="included-count" data-testid="results-included-count">0 included</span>
        </div>
        <div class="status-legend" aria-label="Status legend" data-testid="results-status-legend">
          <span><span class="status-badge status-insync">inSync</span> repository and database match</span>
          <span><span class="status-badge status-repodifferent">repoDifferent</span> exists in both but differs</span>
          <span><span class="status-badge status-repoonly">repoOnly</span> repository only</span>
          <span><span class="status-badge status-databaseonly">databaseOnly</span> PostgreSQL only</span>
          <span><span class="status-badge status-skipped">skipped</span> not safely comparable</span>
          <span><span class="status-badge status-blocked">blocked</span> blocked by safety or dependency rules</span>
          <span><span class="status-badge status-error">error</span> operation failed</span>
          <span><span class="status-badge status-inspected">inspected</span> read from PostgreSQL inspect output</span>
        </div>
        <div class="table-wrap">
          <table class="results-grid" aria-label="Comparison results grid" data-testid="results-table">
            <thead>
              <tr>
                <th>Include</th>
                <th>Object type</th>
                <th>Schema</th>
                <th>Object name</th>
                <th>Status</th>
                <th>Planned operation</th>
                <th>Warnings</th>
              </tr>
            </thead>
            <tbody id="results-body">
              <tr><td colspan="7">Run a compare or plan to populate results.</td></tr>
            </tbody>
          </table>
        </div>
        <section class="subsection" id="reference-data-row-detail" data-testid="reference-data-row-detail">
          <h3>Reference-Data Row Detail</h3>
          <p class="note">Read-only row comparison detail. Masked values are displayed as [masked]. Ignored columns do not cause differences.</p>
          <dl class="summary-list compact" id="reference-data-row-detail-summary"></dl>
        </section>
      </section>

      <section class="workflow-panel" id="step-object-diff">
        <div class="panel-heading">
          <h2>Object Diff</h2>
          <p>Select a result row to inspect available object-level details.</p>
        </div>
        <section class="subsection">
          <h3 id="selected-object-title">No object selected</h3>
          <dl class="summary-list compact" id="selected-object-summary"></dl>
        </section>
        <div class="object-diff-tabs" aria-label="Object Diff view mode">
          <button type="button" class="secondary-button active" data-diff-mode="fullContext">Full Context DDL</button>
          <button type="button" class="secondary-button" data-diff-mode="objectOnly">Object Only DDL</button>
          <button type="button" class="secondary-button" data-diff-mode="relatedObjects">Related Objects</button>
          <button type="button" class="secondary-button" data-diff-mode="rawDetails">Raw Details</button>
        </div>
        <div id="ddl-comparison-status" class="ddl-comparison-status ddl-unavailable">DDL unavailable</div>
        <div id="ddl-diff-view">
          <div class="diff-grid" aria-label="Object detail viewer">
            <section data-testid="object-diff-left">
              <h3 id="source-ddl-heading">Source DDL - Full Context DDL</h3>
              <p class="note">Source type: <strong id="source-type-label">Repository</strong></p>
              <pre id="source-detail" data-testid="object-diff-repository">Diff detail not available yet. DDL not available yet for this object.</pre>
            </section>
            <section data-testid="object-diff-right">
              <h3 id="target-ddl-heading">Target DDL - Full Context DDL</h3>
              <p class="note">Target type: <strong id="target-type-label">Database</strong></p>
              <pre id="target-detail" data-testid="object-diff-database">Diff detail not available yet. DDL not available yet for this object.</pre>
            </section>
          </div>
        </div>
        <div id="related-objects-view" class="related-objects-view" hidden>
          <section class="related-object-comparison-panel" aria-label="Related Object Comparison">
            <h3>Related Object Comparison</h3>
            <p class="note">Related object comparison can include Constraints, Indexes, Comments, and other object details when available.</p>
            <div class="table-wrap">
              <table class="results-grid related-object-comparison-table" aria-label="Related Object Comparison">
                <thead>
                  <tr><th>Type</th><th>Object</th><th>Status</th><th>Source detail</th><th>Target detail</th></tr>
                </thead>
                <tbody id="related-object-comparison-body">
                  <tr><td colspan="5">No related object comparison loaded.</td></tr>
                </tbody>
              </table>
            </div>
          </section>
          <div class="related-objects-grid related-objects-two-column">
          <section class="related-object-side-panel">
            <h3>Source Related Objects</h3>
            <div id="source-related-objects" class="related-object-list">No related object details loaded.</div>
          </section>
          <section class="related-object-side-panel">
            <h3>Target Related Objects</h3>
            <div id="target-related-objects" class="related-object-list">No related object details loaded.</div>
          </section>
          </div>
        </div>
        <section class="subsection">
          <h3>Selected JSON Item</h3>
          <pre id="selected-json" data-testid="selected-json-item">{}</pre>
        </section>
      </section>

      <section class="workflow-panel" id="step-warnings" data-testid="warnings-panel">
        <div class="panel-heading">
          <h2>Warnings</h2>
          <p>Dependency warnings, blocked items, deferred object types, and service errors appear here.</p>
        </div>
        <div id="warnings-list" class="issue-list" data-testid="warnings-list">No warnings yet.</div>
      </section>

      <section class="workflow-panel" id="step-release-plan">
        <div class="panel-heading">
          <h2>Release Plan</h2>
          <p>Generate reviewable release artifacts for Schema Compare: Repository to Database only.</p>
        </div>
        <div id="release-plan-not-applicable" class="issue-item" hidden></div>
        <div id="release-plan-content">
          <div class="form-grid">
            <label for="release-name">Release name
              <input id="release-name" type="text" autocomplete="off" spellcheck="false">
            </label>
            <label for="release-confirmation">Type GENERATE RELEASE ARTIFACTS
              <input id="release-confirmation" type="text" autocomplete="off" spellcheck="false">
            </label>
          </div>
          <div class="button-row">
            <button type="button" data-action="release-preview" data-testid="release-plan-dry-run">Dry-run Release Artifact</button>
            <button type="button" data-action="release-write" data-testid="generate-release-artifact" disabled>Generate Release Artifact</button>
          </div>
          <div class="release-card">
            <h3>Release Context</h3>
            <dl class="summary-list compact" id="release-context" data-testid="release-context"></dl>
          </div>
          <div class="release-card">
            <h3>Risk Summary</h3>
            <div id="release-risk-summary" data-testid="risk-summary">Risk level: Not generated in UI. Use Dry-run Release Artifact to generate risk JSON.</div>
          </div>
          <div class="release-card">
            <h3>Object Summary</h3>
            <dl class="summary-list compact" id="release-object-summary" data-testid="object-summary"></dl>
          </div>
          <div class="release-card">
            <h3>CLI command guidance</h3>
            <p class="note">The browser UI can dry-run and generate review artifacts through the local DbState Service. CLI commands are shown for fallback/manual use.</p>
            <pre id="release-dryrun-command">dbstate release postgres --all --name &lt;release-name&gt; --dry-run --format json</pre>
            <pre id="release-write-command">dbstate release postgres --all --name &lt;release-name&gt;</pre>
          </div>
          <div class="release-card">
            <h3>Release Candidates</h3>
            <p class="note">Release operation badges can include Review SQL, Additive ADD COLUMN, Manual Review, Blocked, Database Only, and Deferred. Generated SQL remains review-only; DbState does not execute SQL.</p>
            <div class="release-selection-controls" data-testid="release-candidate-selection-controls">
              <span id="release-selected-count" data-testid="release-selected-count">0 selected</span>
              <button type="button" data-action="release-select-all-eligible" data-testid="release-select-all-eligible">Select all eligible</button>
              <button type="button" data-action="release-clear-selection" data-testid="release-clear-selection">Clear selection</button>
              <span class="note">Selected candidates only are included in release artifact dry-run or generation.</span>
            </div>
            <div class="table-wrap">
              <table class="results-grid release-candidates-table" aria-label="Release candidates" data-testid="release-candidates">
                <thead>
                  <tr><th class="release-candidate-select-col">Select</th><th class="release-candidate-type-col">Object type</th><th class="release-candidate-schema-col">Schema</th><th class="release-candidate-name-col">Object name</th><th class="release-candidate-status-col">Status</th><th class="release-candidate-operation-col">Planned operation</th><th class="release-candidate-badge-col">Operation / safety</th><th class="release-candidate-explanation-col">Explanation</th><th class="release-candidate-reasons-col">Reasons</th><th class="release-candidate-warnings-col">Warnings</th></tr>
                </thead>
                <tbody id="release-candidates-body">
                  <tr><td colspan="10">No selected result rows yet.</td></tr>
                </tbody>
              </table>
            </div>
          </div>
          <div class="release-card">
            <h3>Dry-run / Generated Artifacts</h3>
            <div id="release-artifact-result" data-testid="generated-artifacts">No release artifact dry-run has been run yet.</div>
          </div>
          <div class="release-card" data-testid="release-artifact-preview">
            <h3>Artifact Preview</h3>
            <div id="release-artifact-preview-meta" class="note">Preview is available after Generate Release Artifact writes review files.</div>
            <pre id="release-artifact-preview-content" data-testid="release-artifact-preview-content">No artifact selected.</pre>
          </div>
          <div class="subsection">
            <h3>Reviewer Checklist</h3>
            <ol>
              <li>Review all REVIEW REQUIRED comments.</li>
              <li>Review blocked and skipped items.</li>
              <li>Review deferred object type warnings.</li>
              <li>Confirm no destructive SQL is present.</li>
              <li>Confirm object ordering is acceptable.</li>
              <li>Have a DBA or responsible engineer review before any manual execution outside DbState.</li>
            </ol>
          </div>
          <p class="note">Release artifacts are reviewable files under database/releases/. DbState does not execute SQL, apply database changes, mutate PostgreSQL, or stage, commit, push, pull, fetch, or tag Git changes.</p>
        </div>
      </section>

      <section class="workflow-panel" id="step-reports" data-testid="reports-panel">
        <div class="panel-heading">
          <h2>Reports / Raw JSON</h2>
          <p>Transparent redacted service output for review and troubleshooting.</p>
        </div>
        <div id="response-summary" class="response-summary">Run a workflow to see results.</div>
        <div class="button-row">
          <button type="button" data-action="copy-json" data-testid="raw-json-copy">Copy JSON</button>
          <span id="copy-json-status" class="note"></span>
        </div>
        <pre id="json-viewer" data-testid="raw-json-panel">{}</pre>
      </section>

      <section class="workflow-panel" id="step-about">
        <div class="panel-heading">
          <h2>About / Safety</h2>
          <p>DbState is local-first, Git/repo state based, and engine-first.</p>
        </div>
        <ul class="safety-list">
          <li>The browser UI is a thin workflow shell over the local service.</li>
          <li>PostgreSQL compare and inspect operations are read-only.</li>
          <li>Release artifacts are reviewable files under <code>database/releases/</code> and require explicit typed confirmation in the UI or CLI.</li>
          <li>No direct database apply exists.</li>
          <li>No generated SQL execution exists.</li>
          <li>Only controlled local file writes are exposed: repository object files and release artifact files. The UI never mutates PostgreSQL or Git history.</li>
          <li>Unsupported object types are deferred for later slices.</li>
          <li>Do not expose the local service publicly.</li>
        </ul>
      </section>
    </main>
  </div>

  <footer class="status-strip" aria-label="Operation status">
    <span>Last operation: <strong id="last-operation">none</strong></span>
    <span>Status: <strong id="last-status">not run</strong></span>
    <span>Warnings: <strong id="last-warnings">0</strong></span>
    <span>Errors: <strong id="last-errors">0</strong></span>
  </footer>

  <script src="/ui/app.js?v=slice22-beta-ui"></script>
</body>
</html>
"#;

const UI_CSS: &str = r#":root {
  color-scheme: light;
  --bg: #f3f6f8;
  --surface: #ffffff;
  --surface-alt: #eef3f6;
  --text: #17202a;
  --muted: #5b6875;
  --border: #d3dde5;
  --accent: #176b87;
  --accent-strong: #0f5369;
  --ok: #146c43;
  --warning: #7a4b00;
  --warning-bg: #fff4d6;
  --danger: #9f2d20;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-height: 100vh;
  background: var(--bg);
  color: var(--text);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  line-height: 1.45;
}

.app-header,
.context-strip,
.status-strip {
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}

.app-header {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  align-items: center;
  padding: 18px 24px;
}

.brand-block h1,
.brand-block p,
.panel-heading h2,
.panel-heading p {
  margin-top: 0;
}

.brand-block h1 {
  margin-bottom: 3px;
  font-size: 24px;
  letter-spacing: 0;
}

.brand-block p,
.panel-heading p,
.note {
  color: var(--muted);
}

.header-status,
.button-row,
.results-toolbar,
.object-filter-row,
.status-strip,
.context-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.status-pill {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 5px 10px;
  color: var(--muted);
  background: var(--surface);
  white-space: nowrap;
}

.status-pill.local-only {
  color: var(--ok);
  border-color: #b7dfc8;
}

.safety-strip {
  padding: 10px 24px;
  background: var(--warning-bg);
  color: var(--warning);
  border-bottom: 1px solid #eed28b;
}

.context-strip {
  padding: 9px 24px;
  color: var(--muted);
}

.app-layout {
  display: grid;
  grid-template-columns: 250px minmax(0, 1fr);
  min-height: calc(100vh - 176px);
}

.workflow-nav {
  padding: 16px;
  background: #e8eef2;
  border-right: 1px solid var(--border);
}

.workflow-step {
  width: 100%;
  margin-bottom: 6px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  padding: 9px 10px;
  text-align: left;
  font: inherit;
  cursor: pointer;
}

.workflow-step:hover,
.workflow-step.active {
  border-color: var(--border);
  background: var(--surface);
}

.workflow-step.active {
  color: var(--accent-strong);
  font-weight: 700;
}

.workflow-main {
  padding: 18px;
  min-width: 0;
}

.workflow-panel {
  display: none;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  padding: 18px;
}

.workflow-panel.active {
  display: block;
}

.panel-heading {
  margin-bottom: 16px;
  border-bottom: 1px solid var(--border);
  padding-bottom: 12px;
}

.panel-heading h2 {
  margin-bottom: 4px;
  font-size: 20px;
}

.subsection {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 14px;
  background: #fbfcfd;
  margin-bottom: 14px;
}

.subsection h3 {
  margin-top: 0;
}

.split-pane,
.diff-grid,
.form-grid {
  display: grid;
  gap: 14px;
}

.split-pane,
.diff-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.form-grid {
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  margin-bottom: 14px;
}

label {
  display: block;
  font-size: 13px;
  font-weight: 700;
}

input,
select {
  width: 100%;
  min-height: 34px;
  margin-top: 5px;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 9px;
  font: inherit;
  background: #ffffff;
}

button {
  border: 0;
  border-radius: 6px;
  background: var(--accent);
  color: #ffffff;
  padding: 8px 11px;
  font: inherit;
  cursor: pointer;
}

button:hover {
  background: var(--accent-strong);
}

.secondary-button {
  background: #e7eef3;
  color: var(--text);
  border: 1px solid var(--border);
}

.secondary-button:hover {
  background: #d8e3eb;
}

.directory-picker {
  margin: 14px 0;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 14px;
  background: #fbfcfd;
}

.directory-picker-header,
.directory-picker-controls {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  justify-content: space-between;
}

.directory-picker-header h3 {
  margin: 0;
}

.directory-picker-controls label {
  flex: 1 1 320px;
}

.directory-picker-error {
  margin: 10px 0;
  border-left: 4px solid var(--danger);
  background: #ffe9e6;
  color: #5d160f;
  padding: 8px 10px;
}

.directory-root-list,
.directory-list {
  display: grid;
  gap: 6px;
  margin-top: 12px;
}

.directory-root-list {
  grid-template-columns: repeat(auto-fit, minmax(120px, max-content));
}

.directory-entry {
  width: 100%;
  text-align: left;
  background: #ffffff;
  color: var(--text);
  border: 1px solid var(--border);
}

.directory-entry:hover {
  background: #edf7fb;
}

.object-filter-row {
  margin-bottom: 14px;
  color: var(--muted);
}

.object-filter-row label {
  font-weight: 600;
}

.results-toolbar {
  justify-content: space-between;
  margin: 10px 0;
}

.results-toolbar label {
  max-width: 220px;
}

.results-context,
.status-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.results-context {
  margin-bottom: 10px;
  color: var(--muted);
}

.results-error-summary {
  margin-bottom: 10px;
  border-left: 4px solid var(--danger);
  background: #ffe9e6;
  color: #5d160f;
  padding: 10px 12px;
}

.status-legend {
  margin-bottom: 10px;
  font-size: 12px;
  color: var(--muted);
}

.table-wrap {
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
}

.results-grid {
  width: 100%;
  border-collapse: collapse;
  min-width: 760px;
  background: #ffffff;
}

.results-grid th,
.results-grid td {
  border-bottom: 1px solid var(--border);
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
  font-size: 13px;
}

.results-grid th {
  background: var(--surface-alt);
  color: #25313c;
  white-space: nowrap;
}

.results-grid tr {
  cursor: pointer;
}

.results-grid tr.selected {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
  background: #edf7fb;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  border-radius: 999px;
  padding: 2px 8px;
  border: 1px solid #c9d2da;
  background: #eef2f5;
  color: #344451;
  font-weight: 700;
  white-space: nowrap;
}

.status-insync {
  border-color: #9bd6b5;
  background: #e7f7ee;
  color: #146c43;
}

.status-repodifferent,
.status-error {
  border-color: #ecaaa3;
  background: #ffe9e6;
  color: #9f2d20;
}

.status-repoonly {
  border-color: #a7c9f2;
  background: #e8f2ff;
  color: #195899;
}

.status-added,
.status-plannedcreate,
.status-created,
.status-databaseonly,
.status-planned,
.status-review {
  border-color: #e2c15f;
  background: #fff6d8;
  color: #775000;
}

.status-changed,
.status-plannedupdate,
.status-updated,
.status-unchanged,
.status-skipped,
.status-unknown {
  border-color: #c7ced5;
  background: #f1f3f5;
  color: #59636e;
}

.status-blocked {
  border-color: #c97c72;
  background: #f8d3ce;
  color: #7b1f16;
}

.status-inspected {
  border-color: #98c8d8;
  background: #e6f5fa;
  color: #176b87;
}

.ddl-comparison-status {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  margin: 0 0 12px;
  padding: 4px 10px;
  border: 1px solid #c7ced5;
  background: #f1f3f5;
  color: #59636e;
  font-weight: 700;
}

.ddl-similar {
  border-color: #9bd6b5;
  background: #e7f7ee;
  color: #146c43;
}

.ddl-different {
  border-color: #ecaaa3;
  background: #ffe9e6;
  color: #9f2d20;
}

.ddl-unavailable {
  border-color: #c7ced5;
  background: #f1f3f5;
  color: #59636e;
}

.beta-disabled-note {
  border-left: 4px solid #e2c15f;
  background: #fff8df;
  padding: 10px 12px;
  border-radius: 6px;
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(14, 24, 33, 0.55);
  display: grid;
  place-items: center;
  z-index: 1000;
}

.modal-backdrop[hidden] {
  display: none;
}

.modal-dialog {
  width: min(560px, calc(100vw - 32px));
  background: #ffffff;
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(14, 24, 33, 0.25);
  padding: 22px;
}

.modal-dialog h2 {
  margin-top: 0;
}

.release-card {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #ffffff;
  padding: 12px;
  margin: 12px 0;
}

.release-card h3 {
  margin-top: 0;
}

.release-selection-controls {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin: 8px 0 10px;
}

.release-candidates-table {
  table-layout: fixed;
  min-width: 1420px;
}

.release-candidates-table th,
.release-candidates-table td {
  white-space: normal;
  overflow-wrap: anywhere;
}

.release-candidate-select-col {
  width: 64px;
}

.release-candidate-type-col,
.release-candidate-schema-col,
.release-candidate-status-col,
.release-candidate-operation-col {
  width: 120px;
}

.release-candidate-name-col,
.release-candidate-badge-col {
  width: 150px;
}

.release-candidate-explanation-col {
  width: 280px;
}

.release-candidate-reasons-col {
  width: 300px;
}

.release-candidate-warnings-col {
  width: 240px;
}

.release-candidate-cell {
  white-space: normal;
  overflow-wrap: anywhere;
}

.release-operation-badge {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: #f5f7fa;
  color: var(--text);
  font-size: 11px;
  font-weight: 700;
  white-space: nowrap;
}

.release-operation-badge.blocked {
  background: #ffeceb;
  border-color: #f2b5b0;
}

.release-operation-badge.manualReview {
  background: #fff4d6;
  border-color: #e6c36a;
}

.release-operation-badge.reviewOnly {
  background: #e7f5ee;
  border-color: #9bd0b5;
}

.release-operation-badge.informational {
  background: #eef2f7;
  border-color: #c4ceda;
}

.diff-line-grid {
  display: grid;
  gap: 0;
  font-family: Consolas, "Liberation Mono", monospace;
  font-size: 12px;
  white-space: pre;
  overflow: auto;
  max-height: 520px;
  background: #111c28;
  color: #dbe7f3;
  border-radius: 6px;
  padding: 10px;
}

.diff-line-row {
  display: flex;
  min-height: 16px;
}

.diff-marker {
  flex: 0 0 2ch;
  width: 2ch;
  user-select: none;
}

.diff-line-text {
  flex: 1 1 auto;
  min-width: 0;
}

.diff-line-same {
  color: #ffffff;
}

.diff-line-different {
  color: #ff8f85;
}

.diff-line-source-only {
  color: #7ee2a8;
}

.diff-line-target-only {
  color: #ff8f85;
}

.object-diff-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}

.object-diff-tabs button.active {
  border-color: var(--accent);
  background: #dff1f6;
  color: var(--accent-strong);
  font-weight: 700;
}

.related-objects-view {
  display: grid;
  gap: 14px;
}

#related-objects-view[hidden] {
  display: none;
}

.related-objects-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  align-items: stretch;
  gap: 14px;
}

.related-objects-two-column {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.related-object-comparison-table {
  table-layout: fixed;
  min-width: 980px;
}

.related-object-comparison-table th,
.related-object-comparison-table td {
  white-space: normal;
  overflow-wrap: anywhere;
}

.related-object-side-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.related-object-list {
  display: grid;
  gap: 8px;
  align-content: start;
  flex: 1 1 auto;
}

.related-object-group {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #ffffff;
  padding: 10px;
}

.related-object-type-group {
  min-width: 0;
}

.related-object-type-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}

.related-object-type-table th,
.related-object-type-table td {
  border-top: 1px solid var(--border);
  padding: 6px 4px;
  text-align: left;
  vertical-align: top;
  white-space: normal;
  overflow-wrap: anywhere;
  font-size: 12px;
}

.related-object-type-table th {
  color: var(--muted);
  font-weight: 700;
}

.related-object-status {
  display: inline-block;
  margin-bottom: 4px;
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 1px 7px;
  background: #eef2f7;
  color: #344451;
  font-size: 11px;
  font-weight: 700;
}

.related-object-group h4 {
  margin: 0 0 6px;
  font-size: 13px;
}

.related-object-group ul {
  margin: 0;
  padding-left: 18px;
}

.summary-list {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 6px 12px;
  margin: 0;
}

.summary-list.compact {
  font-size: 13px;
}

dt {
  color: var(--muted);
}

.muted {
  color: var(--muted);
}

dd {
  margin: 0;
  min-width: 0;
  overflow-wrap: anywhere;
}

.issue-list {
  display: grid;
  gap: 8px;
}

.issue-item {
  border-left: 4px solid var(--warning);
  background: var(--warning-bg);
  padding: 10px 12px;
}

.issue-item.error {
  border-left-color: var(--danger);
  background: #ffe9e6;
}

.response-summary,
.release-summary {
  margin-bottom: 12px;
  color: var(--muted);
}

.safety-list {
  padding-left: 22px;
}

pre {
  overflow: auto;
  min-height: 160px;
  max-height: 420px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #17202a;
  color: #edf4f8;
  font-size: 12px;
}

.status-strip {
  justify-content: space-between;
  padding: 9px 18px;
  color: var(--muted);
}

@media (max-width: 900px) {
  .app-layout {
    grid-template-columns: 1fr;
  }

  .workflow-nav {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    border-right: 0;
    border-bottom: 1px solid var(--border);
  }

  .split-pane,
  .diff-grid,
  .related-objects-grid {
    grid-template-columns: 1fr;
  }

  .related-objects-two-column {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  .app-header {
    display: block;
  }

  .workflow-main,
  .app-header,
  .safety-strip,
  .context-strip {
    padding-left: 14px;
    padding-right: 14px;
  }
}
"#;

const UI_JS: &str = r#"(function () {
  const approvedEndpoints = {
    health: "/api/v1/health",
    repoStatus: "/api/v1/repo/status",
    initPlan: "/api/v1/init/plan",
    initWrite: "/api/v1/init/write",
    inspect: "/api/v1/postgres/inspect",
    compare: "/api/v1/postgres/compare",
    plan: "/api/v1/postgres/plan",
    referenceDataStatus: "/api/v1/reference-data/status",
    referenceDataDatabaseTables: "/api/v1/reference-data/database-tables",
    referenceDataExportPreview: "/api/v1/reference-data/export/preview",
    referenceDataExportWrite: "/api/v1/reference-data/export/write",
    dataCompare: "/api/v1/postgres/data-compare",
    objectDdl: "/api/v1/postgres/object-ddl",
    repositorySyncPreview: "/api/v1/postgres/repository-sync/preview",
    repositorySyncWrite: "/api/v1/postgres/repository-sync/write",
    releasePreview: "/api/v1/postgres/release/preview",
    releaseWrite: "/api/v1/postgres/release/write",
    releaseArtifactPreview: "/api/v1/releases/artifact-preview",
    workspaceRoots: "/api/v1/workspace/roots",
    workspaceListDirectories: "/api/v1/workspace/list-directories",
    workspaceValidate: "/api/v1/workspace/validate",
    profiles: "/api/v1/connections/profiles",
    connectionTest: "/api/v1/connections/test"
  };

  const state = {
    lastResponse: null,
    rows: [],
    visibleRows: [],
    inspectSchemas: [],
    inspectTables: [],
    inspectColumns: [],
    referenceDataTables: [],
    referenceDataConfiguredTables: [],
    referenceDataSelectedTables: new Set(),
    referenceDataDatabaseTables: [],
    referenceDataExportSelections: {},
    referenceDataExportActiveTable: "",
    profiles: [],
    directoryRoots: [],
    directoryCurrentPath: "",
    directoryParentPath: "",
    workspace: {
      isGitRepository: false,
      dbstateProjectStatus: "",
      gitRoot: ""
    },
    included: new Set(),
    releaseSelectedRefs: new Set(),
    releaseCandidateSignature: "",
    selectedIndex: -1,
    selectedObjectDdl: null,
    objectDiffMode: "fullContext",
    previousWorkflowMode: "inspect",
    releaseResponse: null,
    lastOperation: "none"
  };

  const jsonViewer = document.getElementById("json-viewer");
  const responseSummary = document.getElementById("response-summary");
  const servicePill = document.getElementById("service-pill");

  function byId(id) {
    return document.getElementById(id);
  }

  function showModal(title, message) {
    const modal = byId("app-modal");
    if (!modal) {
      responseSummary.textContent = title + ": " + message;
      return;
    }
    byId("app-modal-title").textContent = title;
    const body = byId("app-modal-body");
    body.innerHTML = "";
    textOrEmpty(message).split("\n").forEach(function (line) {
      const p = document.createElement("p");
      p.textContent = line || " ";
      body.appendChild(p);
    });
    modal.hidden = false;
    const closeButton = document.querySelector("[data-action='modal-close']");
    if (closeButton) {
      closeButton.focus();
    }
  }

  function closeModal() {
    const modal = byId("app-modal");
    if (modal) {
      modal.hidden = true;
    }
  }

  function isKnownNonGitWorkspace() {
    return state.workspace.dbstateProjectStatus === "notGitRepository" || state.workspace.isGitRepository === false && !!state.workspace.dbstateProjectStatus;
  }

  function showNotGitRepositoryModal() {
    showModal(
      "Not a Git Repository",
      "The selected directory is not a Git repository.\n\nDbState requires a local Git repository because Git is the source of truth for desired database state.\n\nInitialize Git in this folder first, or choose another folder."
    );
  }

  async function guardGitWorkspaceBefore(action) {
    if (isKnownNonGitWorkspace()) {
      showNotGitRepositoryModal();
      return false;
    }
    if (action === "workspace-status") {
      return true;
    }
    if (!state.workspace.dbstateProjectStatus) {
      const data = await requestJson(approvedEndpoints.workspaceValidate, attachWorkspacePath({}));
      state.lastResponse = data;
      jsonViewer.textContent = redactedJson(data);
      updateWorkspaceContext(data);
      updateSummary("workspace-summary", data);
      if (data && data.dbstateProjectStatus === "notGitRepository") {
        showNotGitRepositoryModal();
        return false;
      }
    }
    return true;
  }

  function value(id) {
    return byId(id).value.trim();
  }

  function resetSelect(select, defaultLabel) {
    select.innerHTML = "";
    const option = document.createElement("option");
    option.value = "";
    option.textContent = defaultLabel || "All";
    select.appendChild(option);
  }

  function appendOption(select, value, label) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label || value;
    select.appendChild(option);
  }

  function workspacePath() {
    return value("workspace-path");
  }

  function postgresUrl() {
    return value("postgres-url");
  }

  function attachWorkspacePath(body) {
    const path = workspacePath();
    if (path) {
      body.repositoryPath = path;
    }
    return body;
  }

  function updateInitWriteButton() {
    const button = document.querySelector("[data-action='init-write']");
    if (!button) {
      return;
    }
    const confirmed = value("init-confirmation") === "INITIALIZE DBSTATE PROJECT";
    const status = state.workspace.dbstateProjectStatus || "";
    const canInitialize = state.workspace.isGitRepository && status !== "completeDbStateStructure";
    button.disabled = !(confirmed && canInitialize);
  }

  function selectedConnectionMode() {
    return value("connection-mode") || "sessionUrl";
  }

  function attachConnection(body) {
    const mode = selectedConnectionMode();
    if (mode === "sessionUrl") {
      const url = postgresUrl();
      if (url) {
        body.postgresUrl = url;
      }
    } else if (mode === "profile") {
      const profileName = value("profile-select");
      const password = value("profile-password");
      if (profileName) {
        body.connection = { profileName: profileName };
        if (password) {
          body.connection.password = password;
        }
      }
    }
    return body;
  }

  function commaList(raw) {
    return raw.split(",").map(function (item) {
      return item.trim();
    }).filter(Boolean);
  }

  function buildScope() {
    const schema = value("compare-schema");
    const table = value("compare-table");
    if (table) {
      return { scope: "table", table: table };
    }
    if (schema) {
      return { scope: "schema", schema: schema };
    }
    return { scope: "all" };
  }

  function dataScope() {
    const scope = value("data-scope");
    if (scope === "all") {
      return { scope: "all" };
    }
    const selected = Array.from(state.referenceDataSelectedTables).sort();
    if (!selected.length) {
      throw new Error("Select at least one configured reference-data table.");
    }
    return { selectedTables: selected };
  }

  function repositorySyncWriteConfirmed() {
    return value("repository-write-confirmation") === "WRITE REPOSITORY FILES";
  }

  function standardOperationAllowed(actionLabel) {
    const mode = currentWorkflowMode();
    if (isSchemaRepositoryToDatabaseMode(mode) && actionLabel !== "Reference-data compare") {
      return true;
    }
    if (isReferenceDataRepositoryToDatabaseMode(mode) && actionLabel === "Reference-data compare") {
      return true;
    }
    clearOperationResults("Workflow mode changed. Run an operation to load results.");
    const message = isSchemaDatabaseToRepositoryMode(mode)
      ? (actionLabel === "Compare"
        ? "Compare is not available in Schema Compare: Database to Repository mode. Use Preview Repository Sync."
        : actionLabel + " is not available in Schema Compare: Database to Repository mode. Use Preview Repository Sync.")
      : actionLabel + " is not available in this reference-data workflow mode.";
    responseSummary.textContent = message;
    return false;
  }

  function repositorySyncBody(write) {
    const body = attachWorkspacePath(attachConnection(buildScope()));
    if (write) {
      body.confirmRepositoryWrite = true;
      body.confirmationText = value("repository-write-confirmation");
    }
    return body;
  }

  function redactedJson(input) {
    return JSON.stringify(input, null, 2)
      .replace(/postgres(?:ql)?:\/\/[^"\s]+/gi, "<redacted-postgres-url>")
      .replace(/"(password|token|secret|connectionString|connectionUrl|postgresUrl)"\s*:\s*"[^"]*"/gi, "\"$1\":\"<redacted>\"")
      .replace(/password[^",}]*/gi, "password=<redacted>")
      .replace(/token[^",}]*/gi, "token=<redacted>")
      .replace(/secret[^",}]*/gi, "secret=<redacted>");
  }

  function summarize(data) {
    const parts = [];
    parts.push(data.success ? "Success" : "Failed");
    if (data.command) {
      parts.push(data.command);
    }
    if (data.branch) {
      parts.push("branch " + data.branch);
    }
    if (data.workingTreeStatus) {
      parts.push("working tree " + data.workingTreeStatus);
    }
    if (Array.isArray(data.warnings) && data.warnings.length) {
      parts.push(data.warnings.length + " warning(s)");
    }
    if (Array.isArray(data.errors) && data.errors.length) {
      parts.push(data.errors.length + " error(s)");
    }
    return parts.join(" | ");
  }

  function updateStatus(label, data) {
    byId("last-operation").textContent = label;
    byId("last-status").textContent = data && data.success ? "success" : "failed";
    byId("last-warnings").textContent = Array.isArray(data && data.warnings) ? data.warnings.length : 0;
    byId("last-errors").textContent = Array.isArray(data && data.errors) ? data.errors.length : 0;
  }

  function updateSummary(id, entries) {
    const target = byId(id);
    target.innerHTML = "";
    Object.keys(entries).forEach(function (key) {
      const dt = document.createElement("dt");
      const dd = document.createElement("dd");
      dt.textContent = key;
      dd.textContent = entries[key] == null ? "" : String(entries[key]);
      target.appendChild(dt);
      target.appendChild(dd);
    });
  }

  function friendlyPath(path) {
    let text = textOrEmpty(path);
    text = text.replace(/\\/g, "/");
    text = text.replace(/^\/{2,3}\?\//, "");
    return text;
  }

  function setDirectoryPickerError(message) {
    const target = byId("directory-picker-error");
    target.textContent = message || "";
    target.hidden = !message;
  }

  function renderDirectoryRoots(roots) {
    const target = byId("directory-root-list");
    target.innerHTML = "";
    if (!Array.isArray(roots) || !roots.length) {
      target.textContent = "No browse roots are available.";
      return;
    }
    roots.forEach(function (root) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "secondary-button";
      button.textContent = root.name || root.path;
      button.addEventListener("click", function () {
        listDirectories(root.path);
      });
      target.appendChild(button);
    });
  }

  function renderDirectoryList(data) {
    const target = byId("directory-list");
    target.innerHTML = "";
    state.directoryCurrentPath = data.path || "";
    state.directoryParentPath = data.parentPath || "";
    byId("directory-picker-path").value = state.directoryCurrentPath;
    if (!Array.isArray(data.directories) || !data.directories.length) {
      target.textContent = "No child directories are available.";
      return;
    }
    data.directories.forEach(function (directory) {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "directory-entry";
      row.textContent = directory.name || directory.path;
      row.addEventListener("click", function () {
        listDirectories(directory.path);
      });
      target.appendChild(row);
    });
  }

  async function loadDirectoryRoots() {
    try {
      setDirectoryPickerError("");
      const data = await requestJson(approvedEndpoints.workspaceRoots, null);
      state.directoryRoots = Array.isArray(data.roots) ? data.roots : [];
      renderDirectoryRoots(state.directoryRoots);
      const preferredPath = workspacePath() || data.currentPath || (state.directoryRoots[0] && state.directoryRoots[0].path) || "";
      if (preferredPath) {
        await listDirectories(preferredPath);
      }
    } catch (error) {
      setDirectoryPickerError(error.message);
    }
  }

  async function listDirectories(path) {
    const selectedPath = (path || value("directory-picker-path") || workspacePath()).trim();
    if (!selectedPath) {
      setDirectoryPickerError("Choose a root or enter a local folder path.");
      return;
    }
    try {
      setDirectoryPickerError("");
      const data = await requestJson(approvedEndpoints.workspaceListDirectories, { path: selectedPath });
      if (data.success === false) {
        setDirectoryPickerError((data.errors || []).join(" ") || "Could not list directories.");
        return;
      }
      renderDirectoryList(data);
    } catch (error) {
      setDirectoryPickerError(error.message);
    }
  }

  async function selectDirectoryAsWorkspace() {
    const selectedPath = state.directoryCurrentPath || value("directory-picker-path");
    if (!selectedPath) {
      setDirectoryPickerError("Select a folder first.");
      return;
    }
    byId("workspace-path").value = selectedPath;
    byId("directory-picker").hidden = true;
    await run("Workspace validate", approvedEndpoints.workspaceValidate, attachWorkspacePath({}), { summaryId: "workspace-summary", step: "workspace" });
  }

  function openDirectoryPicker() {
    byId("directory-picker").hidden = false;
    byId("directory-picker-path").value = workspacePath();
    loadDirectoryRoots();
  }

  function updateWorkspaceContext(data) {
    if (!data) {
      return;
    }
    const repositoryPath = friendlyPath(data.repositoryPath);
    const gitRoot = friendlyPath(data.gitRoot);
    const workspaceDisplay = gitRoot || repositoryPath;
    if (workspaceDisplay) {
      byId("header-workspace").textContent = workspaceDisplay;
      byId("repository-workspace").textContent = workspaceDisplay;
      if (gitRoot) {
        byId("workspace-path").value = gitRoot;
      }
    }
    if (typeof data.isGitRepository === "boolean") {
      state.workspace.isGitRepository = data.isGitRepository;
    }
    if (gitRoot) {
      state.workspace.gitRoot = gitRoot;
    }
    if (gitRoot) {
      byId("repository-git-root").textContent = gitRoot;
    }
    if (data.branch) {
      byId("header-branch").textContent = data.branch;
      byId("repository-branch").textContent = data.branch;
    }
    if (data.workingTreeStatus) {
      byId("header-tree").textContent = data.workingTreeStatus;
      byId("repository-tree").textContent = data.workingTreeStatus;
    }
    if (data.dbstateProjectStatus) {
      byId("repository-project").textContent = data.dbstateProjectStatus;
      state.workspace.dbstateProjectStatus = data.dbstateProjectStatus;
    }
    if (typeof data.isDirty === "boolean") {
      byId("repository-dirty").textContent = data.isDirty ? "dirty" : "clean";
    }
    updateInitWriteButton();
  }

  function updateConnectionModePanels() {
    const mode = selectedConnectionMode();
    byId("session-url-panel").hidden = mode !== "sessionUrl";
    byId("profile-panel").hidden = mode !== "profile";
    byId("environment-panel").hidden = mode !== "environment";
  }

  function updateWorkflowModePanels() {
    const selectedMode = value("workflow-mode") || "schemaRepoToDatabase";
    const mode = selectedMode;
    state.previousWorkflowMode = mode;
    const layout = workflowLayout(mode);
    const schemaRepoToDatabase = isSchemaRepositoryToDatabaseMode(mode);
    const schemaDatabaseToRepository = isSchemaDatabaseToRepositoryMode(mode);
    const referenceDataRepoToDatabase = isReferenceDataRepositoryToDatabaseMode(mode);
    const referenceDataDatabaseToRepository = isReferenceDataDatabaseToRepositoryMode(mode);
    byId("repository-sync-controls").hidden = !schemaDatabaseToRepository;
    byId("reference-data-panel").hidden = !referenceDataRepoToDatabase;
    byId("reference-data-export-panel").hidden = !referenceDataDatabaseToRepository;
    document.querySelectorAll("[data-standard-operation-action]").forEach(function (button) {
      const action = button.dataset.action;
      if (action === "data-compare") {
        button.hidden = !referenceDataRepoToDatabase;
        button.disabled = !referenceDataRepoToDatabase;
      } else {
        button.hidden = !schemaRepoToDatabase;
        button.disabled = !schemaRepoToDatabase;
      }
    });
    byId("source-target-description").textContent = layout.description;
    byId("source-kind").textContent = layout.sourceKind;
    byId("target-kind").textContent = layout.targetKind;
    byId("source-type").textContent = layout.sourceType;
    byId("target-type").textContent = layout.targetType;
    placeSourceTargetContext(layout.sourceContext, layout.targetContext);
    updateRepositoryWriteButton();
    updateReleaseWriteButton();
    renderReleasePlan();
  }

  function workflowLayout(mode) {
    if (isSchemaDatabaseToRepositoryMode(mode)) {
      return {
        description: "DbState captures supported PostgreSQL database state into the selected repository after preview and explicit confirmation.",
        sourceKind: "PostgreSQL database",
        sourceType: "Database",
        sourceContext: "connection",
        targetKind: "Repository desired state",
        targetType: "Repository",
        targetContext: "repository"
      };
    }
    if (mode === "inspect") {
      return {
        description: "DbState reads PostgreSQL catalog state through read-only inspection.",
        sourceKind: "PostgreSQL database",
        sourceType: "Database",
        sourceContext: "connection",
        targetKind: "Read-only catalog view",
        targetType: "Catalog",
        targetContext: "catalog"
      };
    }
    if (isReferenceDataRepositoryToDatabaseMode(mode)) {
      return {
        description: "DbState compares configured repository reference data to PostgreSQL through read-only service operations.",
        sourceKind: "Repository reference-data",
        sourceType: "Repository",
        sourceContext: "repository",
        targetKind: "PostgreSQL target",
        targetType: "Database",
        targetContext: "connection"
      };
    }
    if (isReferenceDataDatabaseToRepositoryMode(mode)) {
      return {
        description: "DbState exports explicitly selected PostgreSQL table data into repository reference-data YAML files only after typed confirmation.",
        sourceKind: "PostgreSQL database",
        sourceType: "Database",
        sourceContext: "connection",
        targetKind: "Repository reference-data",
        targetType: "Repository",
        targetContext: "repository"
      };
    }
    return {
      description: "DbState compares repository desired state to PostgreSQL through read-only service operations.",
      sourceKind: "Repository desired state",
      sourceType: "Repository",
      sourceContext: "repository",
      targetKind: "PostgreSQL database",
      targetType: "Database",
      targetContext: "connection"
    };
  }

  function placeSourceTargetContext(sourceContext, targetContext) {
    const sourceContent = byId("source-content");
    const targetContent = byId("target-content");
    const repositoryContext = byId("repository-context");
    const connectionContext = byId("postgres-connection-context");
    const catalogContext = byId("catalog-context");

    catalogContext.hidden = true;
    [repositoryContext, connectionContext, catalogContext].forEach(function (element) {
      element.hidden = true;
    });

    function appendContext(name, target) {
      if (name === "repository") {
        repositoryContext.hidden = false;
        target.appendChild(repositoryContext);
      } else if (name === "connection") {
        connectionContext.hidden = false;
        target.appendChild(connectionContext);
      } else if (name === "catalog") {
        catalogContext.hidden = false;
        target.appendChild(catalogContext);
      }
    }

    appendContext(sourceContext, sourceContent);
    appendContext(targetContext, targetContent);
  }

  function updateRepositoryWriteButton() {
    const button = document.querySelector("[data-action='repository-sync-write']");
    if (button) {
      button.disabled = !repositorySyncWriteConfirmed();
    }
  }

  function updateProfileList(data) {
    if (!data || !Array.isArray(data.profiles)) {
      return;
    }
    state.profiles = data.profiles.slice().sort(function (left, right) {
      return textOrEmpty(left.name).localeCompare(textOrEmpty(right.name));
    });
    const select = byId("profile-select");
    const selected = select.value;
    resetSelect(select, "No profile selected");
    state.profiles.forEach(function (profile) {
      appendOption(select, profile.name, profile.name);
    });
    if (state.profiles.some(function (profile) { return profile.name === selected; })) {
      select.value = selected;
    }
    updateSelectedProfileDetails();
  }

  function selectedProfile() {
    const selected = value("profile-select");
    return state.profiles.find(function (profile) {
      return profile.name === selected;
    });
  }

  function updateSelectedProfileDetails() {
    const profile = selectedProfile();
    if (!profile) {
      updateSummary("profile-summary", {
        profile: "none",
        host: "",
        database: "",
        username: "",
        sslMode: ""
      });
      return;
    }
    updateSummary("profile-summary", {
      profile: profile.name,
      host: profile.host + ":" + profile.port,
      database: profile.database,
      username: profile.username,
      sslMode: profile.sslMode || ""
    });
    byId("profile-name").value = profile.name || "";
    byId("profile-host").value = profile.host || "";
    byId("profile-port").value = profile.port || 5432;
    byId("profile-database").value = profile.database || "";
    byId("profile-username").value = profile.username || "";
    byId("profile-sslmode").value = profile.sslMode || "prefer";
    byId("profile-description").value = profile.description || "";
  }

  function profileRequestBody() {
    return {
      name: value("profile-name"),
      host: value("profile-host"),
      port: Number(value("profile-port")),
      database: value("profile-database"),
      username: value("profile-username"),
      sslMode: value("profile-sslmode"),
      description: value("profile-description")
    };
  }

  function profilePath(name) {
    return approvedEndpoints.profiles + "/" + encodeURIComponent(name);
  }

  function updateCompareOptionLists(data) {
    if (!data || data.success === false || !Array.isArray(data.schemas) || !Array.isArray(data.tables)) {
      return;
    }
    state.inspectSchemas = data.schemas.map(function (schema) {
      return schema.name;
    }).filter(Boolean).sort();
    state.inspectTables = data.tables.map(function (table) {
      return {
        schema: table.schemaName,
        table: table.tableName,
        qualified: table.schemaName + "." + table.tableName
      };
    }).sort(function (left, right) {
      return left.qualified.localeCompare(right.qualified);
    });
    state.inspectColumns = Array.isArray(data.columns) ? data.columns.slice() : [];

    const schemaSelect = byId("compare-schema");
    const selectedSchema = schemaSelect.value;
    resetSelect(schemaSelect, "All");
    state.inspectSchemas.forEach(function (schema) {
      appendOption(schemaSelect, schema, schema);
    });
    if (state.inspectSchemas.indexOf(selectedSchema) >= 0) {
      schemaSelect.value = selectedSchema;
    }
    updateTableOptions();
  }

  function updateTableOptions() {
    const tableSelect = byId("compare-table");
    const selectedTable = tableSelect.value;
    const selectedSchema = value("compare-schema");
    resetSelect(tableSelect, "All");
    state.inspectTables.filter(function (table) {
      return !selectedSchema || table.schema === selectedSchema;
    }).forEach(function (table) {
      const label = selectedSchema ? table.table : table.qualified;
      appendOption(tableSelect, table.qualified, label);
    });
    if (Array.from(tableSelect.options).some(function (option) {
      return option.value === selectedTable;
    })) {
      tableSelect.value = selectedTable;
    }
  }

  function updateReferenceDataOptions(data) {
    if (data && Array.isArray(data.configuredTables)) {
      updateReferenceDataConfiguredTables(data);
      return;
    }
    if (!data || data.success === false || !Array.isArray(data.tableResults)) {
      return;
    }
    state.referenceDataTables = data.tableResults.map(function (table) {
      return table.tableName;
    }).filter(Boolean).sort();
    const tableSelect = byId("data-table");
    const selectedTable = tableSelect.value;
    resetSelect(tableSelect, "All");
    state.referenceDataTables.forEach(function (table) {
      appendOption(tableSelect, table, table);
    });
    if (state.referenceDataTables.indexOf(selectedTable) >= 0) {
      tableSelect.value = selectedTable;
    }
  }

  function updateReferenceDataConfiguredTables(data) {
    state.referenceDataConfiguredTables = Array.isArray(data.configuredTables)
      ? data.configuredTables.slice().sort(function (left, right) {
          return textOrEmpty(left.tableName).localeCompare(textOrEmpty(right.tableName));
        })
      : [];
    state.referenceDataTables = state.referenceDataConfiguredTables.map(function (table) {
      return table.tableName;
    });
    const configuredNames = new Set(state.referenceDataConfiguredTables.map(function (table) {
      return table.tableName;
    }));
    state.referenceDataSelectedTables = new Set(Array.from(state.referenceDataSelectedTables).filter(function (table) {
      return configuredNames.has(table);
    }));
    updateSummary("reference-data-registry-summary", {
      repositoryPathUsed: data.repositoryPathUsed || data.gitRoot || data.repositoryPath || "",
      registryPath: data.registryPath || "database/reference-data/dbstate.reference-data.yml",
      registryExists: data.registryExists === true,
      status: data.status || (data.success === false ? "invalid" : state.referenceDataConfiguredTables.length ? "ready" : "needs setup"),
      configuredTables: state.referenceDataConfiguredTables.length,
      warnings: Array.isArray(data.warnings) ? data.warnings.join("; ") : ""
    });
    renderReferenceDataConfiguredTables();
  }

  function renderReferenceDataConfiguredTables() {
    const body = byId("reference-data-configured-body");
    body.innerHTML = "";
    const tableSelect = byId("data-table");
    const selectedTable = tableSelect.value;
    resetSelect(tableSelect, "No configured tables loaded");
    if (!state.referenceDataConfiguredTables.length) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 6;
      td.textContent = "No configured reference-data tables. Add tables to database/reference-data/dbstate.reference-data.yml.";
      tr.appendChild(td);
      body.appendChild(tr);
      updateReferenceDataSelectedCount();
      return;
    }
    state.referenceDataConfiguredTables.forEach(function (table) {
      appendOption(tableSelect, table.tableName, table.tableName);
      const tr = document.createElement("tr");
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.setAttribute("data-testid", "reference-data-table-checkbox");
      checkbox.checked = state.referenceDataSelectedTables.has(table.tableName);
      checkbox.addEventListener("change", function () {
        if (checkbox.checked) {
          state.referenceDataSelectedTables.add(table.tableName);
        } else {
          state.referenceDataSelectedTables.delete(table.tableName);
        }
        updateReferenceDataSelectedCount();
      });
      [
        checkbox,
        table.schema || "",
        table.name || "",
        textOrEmpty(table.keyColumns),
        textOrEmpty(table.ignoredColumns),
        textOrEmpty(table.maskedColumns) || "[none]"
      ].forEach(function (value) {
        const td = document.createElement("td");
        if (value && value.nodeType) {
          td.appendChild(value);
        } else {
          td.textContent = textOrEmpty(value);
        }
        tr.appendChild(td);
      });
      body.appendChild(tr);
    });
    if (state.referenceDataTables.indexOf(selectedTable) >= 0) {
      tableSelect.value = selectedTable;
    }
    updateReferenceDataSelectedCount();
  }

  function updateReferenceDataDatabaseTables(data) {
    state.referenceDataDatabaseTables = Array.isArray(data.tables) ? data.tables.slice() : [];
    state.referenceDataDatabaseTables.sort(function (left, right) {
      return textOrEmpty(left.tableName).localeCompare(textOrEmpty(right.tableName));
    });
    state.referenceDataExportSelections = {};
    state.referenceDataExportActiveTable = "";
    renderReferenceDataDatabaseTables();
    renderReferenceDataExportDetail(null);
  }

  function defaultReferenceDataExportSelection(table) {
    const columns = Array.isArray(table.columns) ? table.columns : [];
    const suggested = Array.isArray(table.suggestedKeyColumns) && table.suggestedKeyColumns.length
      ? table.suggestedKeyColumns.slice()
      : columns.length ? [columns[0].name] : [];
    const versioned = columns.map(function (column) {
      return column.name;
    }).filter(function (column) {
      return suggested.indexOf(column) < 0;
    });
    return {
      schema: table.schema,
      name: table.name,
      keyColumns: suggested,
      versionedColumns: versioned,
      maskedColumns: []
    };
  }

  function selectedReferenceDataExportTables() {
    return Object.keys(state.referenceDataExportSelections).sort().map(function (tableName) {
      return state.referenceDataExportSelections[tableName];
    });
  }

  function renderReferenceDataDatabaseTables() {
    const body = byId("reference-data-database-tables-body");
    body.innerHTML = "";
    const filter = textOrEmpty(value("reference-data-export-search")).toLowerCase();
    const tables = state.referenceDataDatabaseTables.filter(function (table) {
      return !filter || textOrEmpty(table.tableName).toLowerCase().indexOf(filter) >= 0;
    });
    if (!tables.length) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 8;
      td.textContent = state.referenceDataDatabaseTables.length ? "No database tables match the filter." : "Load database tables to choose reference-data exports.";
      tr.appendChild(td);
      body.appendChild(tr);
      updateReferenceDataExportSelectedCount();
      return;
    }
    tables.forEach(function (table) {
      const tr = document.createElement("tr");
      const tableName = table.tableName;
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = !!state.referenceDataExportSelections[tableName];
      checkbox.setAttribute("data-testid", "reference-data-export-table-checkbox");
      checkbox.addEventListener("change", function () {
        if (checkbox.checked) {
          state.referenceDataExportSelections[tableName] = defaultReferenceDataExportSelection(table);
          state.referenceDataExportActiveTable = tableName;
          renderReferenceDataExportDetail(table);
        } else {
          delete state.referenceDataExportSelections[tableName];
          if (state.referenceDataExportActiveTable === tableName) {
            state.referenceDataExportActiveTable = "";
            renderReferenceDataExportDetail(null);
          }
        }
        updateReferenceDataExportSelectedCount();
        renderReferenceDataDatabaseTables();
      });
      const selectCell = document.createElement("td");
      selectCell.appendChild(checkbox);
      tr.appendChild(selectCell);
      const currentSelection = state.referenceDataExportSelections[tableName] || defaultReferenceDataExportSelection(table);
      [table.schema,
        table.name,
        (table.suggestedKeyColumns || []).join(", ") || "none",
        (currentSelection.keyColumns || []).length,
        (currentSelection.versionedColumns || []).length,
        (currentSelection.maskedColumns || []).length,
        table.rowCount == null ? "not counted" : table.rowCount
      ].forEach(function (value) {
        const td = document.createElement("td");
        td.textContent = textOrEmpty(value);
        tr.appendChild(td);
      });
      tr.addEventListener("click", function (event) {
        if (event.target !== checkbox) {
          state.referenceDataExportActiveTable = tableName;
          renderReferenceDataExportDetail(table);
        }
      });
      body.appendChild(tr);
    });
    updateReferenceDataExportSelectedCount();
  }

  function updateReferenceDataExportSelectedCount() {
    const count = selectedReferenceDataExportTables().length;
    byId("reference-data-export-selected-count").textContent = count + " selected for export";
    updateReferenceDataWriteButton();
  }

  function renderReferenceDataExportDetail(table) {
    const body = byId("reference-data-export-columns-body");
    body.innerHTML = "";
    if (!table) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 8;
      td.textContent = "Select a database table to configure columns.";
      tr.appendChild(td);
      body.appendChild(tr);
      updateSummary("reference-data-export-detail-summary", { table: "none" });
      return;
    }
    const selection = state.referenceDataExportSelections[table.tableName] || defaultReferenceDataExportSelection(table);
    const columns = Array.isArray(table.columns) ? table.columns : [];
    const selected = new Set([].concat(selection.keyColumns, selection.versionedColumns, selection.maskedColumns));
    const ignored = columns.map(function (column) {
      return column.name;
    }).filter(function (column) {
      return !selected.has(column);
    });
    updateSummary("reference-data-export-detail-summary", {
      table: table.tableName,
      keyColumns: selection.keyColumns.join(", ") || "none",
      versionedColumns: selection.versionedColumns.join(", ") || "none",
      ignoredColumns: ignored.join(", ") || "none",
      maskedColumns: selection.maskedColumns.join(", ") || "none"
    });
    columns.forEach(function (column) {
      const tr = document.createElement("tr");
      [column.name, column.dataType, column.nullable ? "yes" : "no", column.isPrimaryKey ? "yes" : "no", column.isUnique ? "yes" : "no"].forEach(function (value) {
        const td = document.createElement("td");
        td.textContent = textOrEmpty(value);
        tr.appendChild(td);
      });
      [["keyColumns", "Key"], ["versionedColumns", "Versioned"], ["maskedColumns", "Masked"]].forEach(function (entry) {
        const field = entry[0];
        const td = document.createElement("td");
        const checkbox = document.createElement("input");
        checkbox.type = "checkbox";
        checkbox.checked = selection[field].indexOf(column.name) >= 0;
        checkbox.setAttribute("data-testid", "reference-data-export-" + field);
        checkbox.addEventListener("change", function () {
          updateReferenceDataColumnSelection(table, column.name, field, checkbox.checked);
        });
        td.appendChild(checkbox);
        tr.appendChild(td);
      });
      body.appendChild(tr);
    });
  }

  function updateReferenceDataColumnSelection(table, columnName, field, checked) {
    const tableName = table.tableName;
    const selection = state.referenceDataExportSelections[tableName] || defaultReferenceDataExportSelection(table);
    ["keyColumns", "versionedColumns", "maskedColumns"].forEach(function (name) {
      selection[name] = selection[name].filter(function (value) {
        return value !== columnName;
      });
    });
    if (checked) {
      selection[field].push(columnName);
    }
    state.referenceDataExportSelections[tableName] = selection;
    renderReferenceDataExportDetail(table);
    updateReferenceDataExportSelectedCount();
  }

  function referenceDataExportBody(writeFiles) {
    const tables = selectedReferenceDataExportTables();
    if (!tables.length) {
      throw new Error("Select at least one database table to export as reference data.");
    }
    const body = attachWorkspacePath(attachConnection({ tables: tables }));
    if (writeFiles) {
      body.confirmReferenceDataWrite = true;
      body.confirmationText = value("reference-data-write-confirmation");
    }
    return body;
  }

  function updateReferenceDataWriteButton() {
    const button = document.querySelector("[data-action='reference-data-write-yaml']");
    if (!button) {
      return;
    }
    button.disabled = value("reference-data-write-confirmation") !== "WRITE REFERENCE DATA FILES" || selectedReferenceDataExportTables().length === 0;
  }

  function renderReferenceDataExportPreview(data) {
    const preview = byId("reference-data-export-preview");
    const chunks = [];
    if (data.registryYaml) {
      chunks.push('# database/reference-data/dbstate.reference-data.yml\n' + data.registryYaml);
    }
    if (Array.isArray(data.tablePreviews)) {
      data.tablePreviews.forEach(function (table) {
        chunks.push('# ' + table.file + '\n' + table.yaml);
      });
    }
    preview.textContent = chunks.length ? chunks.join('\n') : "Preview generated registry and table YAML here.";
    updateSummary("reference-data-export-summary", {
      success: data.success,
      filesCreated: Array.isArray(data.filesCreated) ? data.filesCreated.length : 0,
      filesUpdated: Array.isArray(data.filesUpdated) ? data.filesUpdated.length : 0,
      filesUnchanged: Array.isArray(data.filesUnchanged) ? data.filesUnchanged.length : 0,
      warnings: Array.isArray(data.warnings) ? data.warnings.join("; ") : ""
    });
  }

  function updateReferenceDataSelectedCount() {
    const count = state.referenceDataSelectedTables.size;
    byId("reference-data-selected-count").textContent = count + " selected";
  }

  function updateObjectTypeFilterOptions(rows, label) {
    const select = byId("object-type-filter");
    const selected = select.value || "all";
    resetSelect(select, "All");
    select.options[0].value = "all";
    appendOption(select, "schema", "Schema");
    appendOption(select, "table", "Table");
    ["extension", "enum", "sequence", "index", "view", "constraint", "function", "trigger", "grant", "rlsPolicy"].forEach(function (type) {
      if (rows.some(function (row) { return row.objectType === type; })) {
        appendOption(select, type, type.charAt(0).toUpperCase() + type.slice(1));
      }
    });
    const mode = value("workflow-mode");
    const hasReferenceData = isReferenceDataRepositoryToDatabaseMode(mode) || isReferenceDataDatabaseToRepositoryMode(mode) || label === "Reference-data compare" || rows.some(function (row) {
      return row.objectType === "referenceDataTable" || row.objectType === "referenceDataRow";
    });
    if (hasReferenceData) {
      appendOption(select, "referenceData", "Reference data");
    }
    const hasUnknownOrSkipped = rows.some(function (row) {
      return row.objectType === "unknown" || row.status === "skipped";
    });
    if (hasUnknownOrSkipped) {
      appendOption(select, "unknown", "Unknown or Skipped");
    }
    if (Array.from(select.options).some(function (option) {
      return option.value === selected;
    })) {
      select.value = selected;
    }
  }

  function updateStatusFilterOptions(rows) {
    const select = byId("status-filter");
    if (!select) {
      return;
    }
    const selected = select.value || "all";
    const preferred = [
      "repoDifferent",
      "inSync",
      "repoOnly",
      "databaseOnly",
      "plannedCreate",
      "plannedUpdate",
      "blocked",
      "error",
      "skipped",
      "inspected"
    ];
    resetSelect(select, "All");
    select.options[0].value = "all";
    preferred.forEach(function (status) {
      if (rows.some(function (row) { return row.status === status; })) {
        appendOption(select, status, status);
      }
    });
    rows.map(function (row) { return row.status; }).filter(Boolean).sort().forEach(function (status) {
      if (!Array.from(select.options).some(function (option) { return option.value === status; })) {
        appendOption(select, status, status);
      }
    });
    if (Array.from(select.options).some(function (option) { return option.value === selected; })) {
      select.value = selected;
    }
  }

  function showStep(step) {
    document.querySelectorAll(".workflow-step").forEach(function (button) {
      button.classList.toggle("active", button.dataset.step === step);
    });
    document.querySelectorAll(".workflow-panel").forEach(function (panel) {
      panel.classList.toggle("active", panel.id === "step-" + step);
    });
  }

  function rowRef(row, index) {
    return row.objectRef || [row.objectType, row.schema, row.name, index].join(":");
  }

  function textOrEmpty(value) {
    if (value == null) {
      return "";
    }
    if (Array.isArray(value)) {
      return value.join(", ");
    }
    return String(value);
  }

  function splitIdentity(value) {
    const text = textOrEmpty(value);
    const withoutType = text.indexOf(":") > -1 ? text.split(":").slice(1).join(":") : text;
    const dot = withoutType.indexOf(".");
    if (dot > -1) {
      return {
        schema: withoutType.slice(0, dot),
        name: withoutType.slice(dot + 1)
      };
    }
    return {
      schema: "",
      name: withoutType
    };
  }

  function functionIdentitySlug(identityArguments) {
    const text = textOrEmpty(identityArguments).trim();
    if (!text) {
      return "no_args";
    }
    return text.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "").slice(0, 80) || "args";
  }

  function fileNameWithoutSql(path) {
    const normalized = textOrEmpty(path).replace(/\\/g, "/");
    const fileName = normalized.split("/").filter(Boolean).pop() || normalized;
    return fileName.endsWith(".sql") ? fileName.slice(0, -4) : fileName;
  }

  function identityFromPath(path) {
    const normalized = textOrEmpty(path).replace(/\\/g, "/");
    const fileBase = fileNameWithoutSql(normalized);
    if (normalized.indexOf("database/objects/schemas/") >= 0) {
      return {
        objectType: "schema",
        schema: fileBase,
        name: fileBase
      };
    }
    if (normalized.indexOf("database/objects/tables/") >= 0) {
      const dot = fileBase.indexOf(".");
      if (dot > 0) {
        return {
          objectType: "table",
          schema: fileBase.slice(0, dot),
          name: fileBase.slice(dot + 1)
        };
      }
      return {
        objectType: "table",
        schema: "",
        name: fileBase
      };
    }
    if (normalized.indexOf("database/objects/extensions/") >= 0) {
      return {
        objectType: "extension",
        schema: "",
        name: fileBase
      };
    }
    if (normalized.indexOf("database/objects/enums/") >= 0) {
      const dot = fileBase.indexOf(".");
      if (dot > 0) {
        return {
          objectType: "enum",
          schema: fileBase.slice(0, dot),
          name: fileBase.slice(dot + 1)
        };
      }
    }
    if (normalized.indexOf("database/objects/sequences/") >= 0) {
      const dot = fileBase.indexOf(".");
      if (dot > 0) {
        return {
          objectType: "sequence",
          schema: fileBase.slice(0, dot),
          name: fileBase.slice(dot + 1)
        };
      }
    }
    if (normalized.indexOf("database/objects/indexes/") >= 0) {
      const parts = fileBase.split(".");
      if (parts.length >= 3) {
        return {
          objectType: "index",
          schema: parts[0],
          name: parts.slice(2).join("."),
          parentName: parts[1]
        };
      }
    }
    if (normalized.indexOf("database/objects/views/") >= 0) {
      const dot = fileBase.indexOf(".");
      if (dot > 0) {
        return {
          objectType: "view",
          schema: fileBase.slice(0, dot),
          name: fileBase.slice(dot + 1)
        };
      }
    }
    if (normalized.indexOf("database/objects/materialized-views/") >= 0) {
      const dot = fileBase.indexOf(".");
      if (dot > 0) {
        return {
          objectType: "materializedView",
          schema: fileBase.slice(0, dot),
          name: fileBase.slice(dot + 1)
        };
      }
    }
    if (normalized.indexOf("database/objects/functions/") >= 0) {
      const parts = fileBase.split(".");
      if (parts.length >= 3) {
        return {
          objectType: "function",
          schema: parts[0],
          name: parts.slice(1).join(".")
        };
      }
    }
    if (normalized.indexOf("database/objects/triggers/") >= 0) {
      const parts = fileBase.split(".");
      if (parts.length >= 3) {
        return {
          objectType: "trigger",
          schema: parts[0],
          name: parts.slice(1).join("."),
          parentName: parts[1]
        };
      }
    }
    if (normalized.indexOf("database/objects/constraints/") >= 0) {
      const parts = fileBase.split(".");
      if (parts.length >= 3) {
        return {
          objectType: "constraint",
          schema: parts[0],
          name: parts.slice(2).join("."),
          parentName: parts[1]
        };
      }
    }
    if (normalized.indexOf("database/objects/grants/") >= 0) {
      const parts = fileBase.split(".");
      const folder = normalized.indexOf("database/objects/grants/schemas/") >= 0 ? "schema"
        : normalized.indexOf("database/objects/grants/tables/") >= 0 ? "table"
        : normalized.indexOf("database/objects/grants/views/") >= 0 ? "view"
        : normalized.indexOf("database/objects/grants/materialized-views/") >= 0 ? "materializedView"
        : normalized.indexOf("database/objects/grants/sequences/") >= 0 ? "sequence"
        : normalized.indexOf("database/objects/grants/functions/") >= 0 ? "function"
        : "grant";
      if (folder === "schema" && parts.length === 2) {
        return {
          objectType: "grant",
          schema: parts[0],
          name: folder + "." + parts[0] + "." + parts[1],
          parentName: folder
        };
      }
      if (folder === "function" && parts.length === 4) {
        return {
          objectType: "grant",
          schema: parts[0],
          name: folder + "." + parts.join("."),
          parentName: folder
        };
      }
      if (parts.length === 3) {
        return {
          objectType: "grant",
          schema: parts[0],
          name: folder + "." + parts.join("."),
          parentName: folder
        };
      }
    }
    if (normalized.indexOf("database/objects/rls-policies/") >= 0) {
      const parts = fileBase.split(".");
      if (parts.length >= 3) {
        return {
          objectType: "rlsPolicy",
          schema: parts[0],
          name: parts.slice(1).join("."),
          parentName: parts[1]
        };
      }
    }
    return null;
  }

  function identityFromObjectRef(objectRef) {
    const text = textOrEmpty(objectRef);
    if (text.indexOf("schema:") === 0) {
      const schema = text.slice("schema:".length);
      return {
        objectType: "schema",
        schema: schema,
        name: schema
      };
    }
    if (text.indexOf("table:") === 0) {
      const identity = splitIdentity(text);
      return {
        objectType: "table",
        schema: identity.schema,
        name: identity.name
      };
    }
    if (text.indexOf("extension:") === 0) {
      const name = text.slice("extension:".length);
      return { objectType: "extension", schema: "", name: name };
    }
    if (text.indexOf("enum:") === 0) {
      const identity = splitIdentity(text);
      return { objectType: "enum", schema: identity.schema, name: identity.name };
    }
    if (text.indexOf("sequence:") === 0) {
      const identity = splitIdentity(text);
      return { objectType: "sequence", schema: identity.schema, name: identity.name };
    }
    if (text.indexOf("index:") === 0) {
      const identity = text.slice("index:".length);
      const parts = identity.split(".");
      if (parts.length >= 3) {
        return { objectType: "index", schema: parts[0], name: parts.slice(2).join("."), parentName: parts[1] };
      }
    }
    if (text.indexOf("view:") === 0) {
      const identity = splitIdentity(text);
      return { objectType: "view", schema: identity.schema, name: identity.name };
    }
    if (text.indexOf("materializedView:") === 0) {
      const identity = splitIdentity(text);
      return { objectType: "materializedView", schema: identity.schema, name: identity.name };
    }
    if (text.indexOf("constraint:") === 0) {
      const identity = text.slice("constraint:".length);
      const parts = identity.split(".");
      if (parts.length >= 3) {
        return { objectType: "constraint", schema: parts[0], name: parts.slice(2).join("."), parentName: parts[1] };
      }
    }
    if (text.indexOf("function:") === 0) {
      const identity = text.slice("function:".length);
      const parts = identity.split(".");
      if (parts.length >= 3) {
        return { objectType: "function", schema: parts[0], name: parts.slice(1).join(".") };
      }
    }
    if (text.indexOf("trigger:") === 0) {
      const identity = text.slice("trigger:".length);
      const parts = identity.split(".");
      if (parts.length >= 3) {
        return { objectType: "trigger", schema: parts[0], name: parts.slice(1).join("."), parentName: parts[1] };
      }
    }
    if (text.indexOf("grant:") === 0) {
      const identity = text.slice("grant:".length);
      const parts = identity.split(".");
      if (parts.length >= 3) {
        return { objectType: "grant", schema: parts[1] || "", name: identity, parentName: parts[0] };
      }
    }
    if (text.indexOf("rlsPolicy:") === 0) {
      const identity = text.slice("rlsPolicy:".length);
      const parts = identity.split(".");
      if (parts.length >= 3) {
        return { objectType: "rlsPolicy", schema: parts[0], name: parts.slice(1).join("."), parentName: parts[1] };
      }
    }
    return null;
  }

  function identityFromTableName(tableName) {
    const identity = splitIdentity(tableName);
    return {
      objectType: "table",
      schema: identity.schema,
      name: identity.name
    };
  }

  function normalizeObjectType(value) {
    const text = textOrEmpty(value);
    if (text === "reference row") {
      return "referenceDataRow";
    }
    if (text === "reference table") {
      return "referenceDataTable";
    }
    if (["schema", "table", "column", "extension", "enum", "sequence", "index", "view", "materializedView", "constraint", "function", "trigger", "grant", "rlsPolicy", "referenceDataTable", "referenceDataRow"].indexOf(text) >= 0) {
      return text;
    }
    return text || "unknown";
  }

  function statusClass(status) {
    return "status-" + textOrEmpty(status).toLowerCase().replace(/[^a-z0-9]+/g, "");
  }

  function rowFromItem(item, status, fallbackType) {
    const raw = typeof item === "object" && item !== null ? item : { value: item };
    const objectRef = raw.objectRef || raw.objectName || raw.tableName || raw.relativePath || raw.value || "";
    const pathIdentity = identityFromPath(raw.relativePath || objectRef);
    const refIdentity = identityFromObjectRef(objectRef);
    const tableIdentity = raw.tableName ? identityFromTableName(raw.tableName) : null;
    const identity = pathIdentity || refIdentity || tableIdentity || splitIdentity(objectRef || raw.name || "");
    const objectType = normalizeObjectType(raw.objectType || identity.objectType || fallbackType || "unknown");
    const repositorySyncStatuses = ["added", "changed", "unchanged", "plannedCreate", "plannedUpdate", "created", "updated"];
    const isRepositorySync = repositorySyncStatuses.indexOf(status) >= 0;
    const operationByStatus = {
      added: "planned repository create",
      changed: "planned repository update",
      unchanged: "none",
      plannedCreate: "planned repository create",
      plannedUpdate: "planned repository update",
      created: "repository file created",
      updated: "repository file updated"
    };
    return {
      objectRef: objectRef || objectType + ":" + (raw.name || status),
      objectType: objectType,
      schema: raw.schema || raw.schemaName || identity.schema || "",
      name: raw.name || raw.table || identity.name || fileNameWithoutSql(raw.relativePath) || status,
      status: raw.compareClassification || raw.classification || status,
      operation: raw.planIntent || raw.plannedOperation || operationByStatus[status] || (status === "inspected" ? "" : ""),
      operationKind: raw.operationKind || "",
      operationLabel: raw.operationLabel || "",
      safetyBadge: raw.safetyBadge || "",
      safetyLevel: raw.safetyLevel || "",
      operationExplanation: raw.operationExplanation || "",
      operationReasons: Array.isArray(raw.operationReasons) ? raw.operationReasons : [],
      warnings: raw.warnings || raw.dependencyWarnings || [],
      source: raw.source || (isRepositorySync ? "PostgreSQL database" : "repository"),
      target: raw.relativePath || raw.target || raw.value || (isRepositorySync ? "repository desired-state file" : "postgresql"),
      relativePath: raw.relativePath || (textOrEmpty(raw.value).indexOf("database/objects/") === 0 ? raw.value : ""),
      raw: raw
    };
  }

  function appendObjectList(rows, data, field, status, type) {
    if (!Array.isArray(data[field])) {
      return;
    }
    data[field].forEach(function (item) {
      rows.push(rowFromItem(item, status, type));
    });
  }

  function referenceTableStatus(table) {
    const counts = table && table.rowCounts ? table.rowCounts : {};
    if ((counts.repoDifferent || 0) > 0) {
      return "repoDifferent";
    }
    if ((counts.repoOnly || 0) > 0) {
      return "repoOnly";
    }
    if ((counts.databaseOnly || 0) > 0) {
      return "databaseOnly";
    }
    if ((counts.skipped || 0) > 0 || (Array.isArray(table.errors) && table.errors.length)) {
      return "skipped";
    }
    return "inSync";
  }

  function rowsFromResponse(data, label) {
    const rows = [];
    appendObjectList(rows, data, "inSync", "inSync", "unknown");
    appendObjectList(rows, data, "repoDifferent", "repoDifferent", "unknown");
    appendObjectList(rows, data, "repoOnly", "repoOnly", "unknown");
    appendObjectList(rows, data, "databaseOnly", "databaseOnly", "unknown");
    appendObjectList(rows, data, "skipped", "skipped", "unknown");
    appendObjectList(rows, data, "planItems", "planned", "unknown");
    appendObjectList(rows, data, "blockedItems", "blocked", "unknown");
    appendObjectList(rows, data, "addedFiles", "added", "unknown");
    appendObjectList(rows, data, "changedFiles", "changed", "unknown");
    appendObjectList(rows, data, "unchangedFiles", "unchanged", "unknown");
    appendObjectList(rows, data, "skippedFiles", "skipped", "unknown");
    appendObjectList(rows, data, "plannedCreates", "plannedCreate", "unknown");
    appendObjectList(rows, data, "plannedUpdates", "plannedUpdate", "unknown");
    appendObjectList(rows, data, "createdFiles", "created", "unknown");
    appendObjectList(rows, data, "updatedFiles", "updated", "unknown");

    if (label === "Inspect" && data.success !== false) {
      if (Array.isArray(data.schemas)) {
        data.schemas.forEach(function (schema) {
          rows.push({
            objectRef: "schema:" + schema.name,
            objectType: "schema",
            schema: schema.name,
            name: schema.name,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: schema
          });
        });
      }
      if (Array.isArray(data.tables)) {
        data.tables.forEach(function (table) {
          rows.push({
            objectRef: "table:" + table.schemaName + "." + table.tableName,
            objectType: "table",
            schema: table.schemaName,
            name: table.tableName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: table
          });
        });
      }
      if (Array.isArray(data.extensions)) {
        data.extensions.forEach(function (extension) {
          rows.push({
            objectRef: "extension:" + extension.extensionName,
            objectType: "extension",
            schema: extension.schemaName || "",
            name: extension.extensionName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: extension
          });
        });
      }
      if (Array.isArray(data.enums)) {
        data.enums.forEach(function (item) {
          rows.push({
            objectRef: "enum:" + item.schemaName + "." + item.enumName,
            objectType: "enum",
            schema: item.schemaName,
            name: item.enumName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.sequences)) {
        data.sequences.forEach(function (item) {
          rows.push({
            objectRef: "sequence:" + item.schemaName + "." + item.sequenceName,
            objectType: "sequence",
            schema: item.schemaName,
            name: item.sequenceName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.indexes)) {
        data.indexes.forEach(function (item) {
          rows.push({
            objectRef: "index:" + item.schemaName + "." + item.tableName + "." + item.indexName,
            objectType: "index",
            schema: item.schemaName,
            name: item.indexName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.views)) {
        data.views.forEach(function (item) {
          rows.push({
            objectRef: "view:" + item.schemaName + "." + item.viewName,
            objectType: "view",
            schema: item.schemaName,
            name: item.viewName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.materializedViews)) {
        data.materializedViews.forEach(function (item) {
          rows.push({
            objectRef: "materializedView:" + item.schemaName + "." + item.materializedViewName,
            objectType: "materializedView",
            schema: item.schemaName,
            name: item.materializedViewName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.constraints)) {
        data.constraints.forEach(function (item) {
          rows.push({
            objectRef: "constraint:" + item.schemaName + "." + item.tableName + "." + item.constraintName,
            objectType: "constraint",
            schema: item.schemaName,
            name: item.constraintName,
            parentName: item.tableName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.functions)) {
        data.functions.forEach(function (item) {
          const signature = functionIdentitySlug(item.identityArguments || "");
          rows.push({
            objectRef: "function:" + item.schemaName + "." + item.functionName + "." + signature,
            objectType: "function",
            schema: item.schemaName,
            name: item.functionName + "." + signature,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.triggers)) {
        data.triggers.forEach(function (item) {
          rows.push({
            objectRef: "trigger:" + item.schemaName + "." + item.relationName + "." + item.triggerName,
            objectType: "trigger",
            schema: item.schemaName,
            name: item.relationName + "." + item.triggerName,
            parentName: item.relationName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.grants)) {
        data.grants.forEach(function (item) {
          const granteeToken = String(item.grantee || "").toUpperCase() === "PUBLIC" ? "public" : item.grantee;
          const signature = item.targetKind === "function" ? functionIdentitySlug(item.identityArguments || "") : "";
          const targetIdentity = item.targetKind === "schema"
            ? item.schemaName + "." + granteeToken
            : item.targetKind === "function"
              ? item.schemaName + "." + item.objectName + "." + signature + "." + granteeToken
              : item.schemaName + "." + item.objectName + "." + granteeToken;
          rows.push({
            objectRef: "grant:" + item.targetKind + "." + targetIdentity,
            objectType: "grant",
            schema: item.schemaName,
            name: item.targetKind + "." + targetIdentity,
            parentName: item.targetKind,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
      if (Array.isArray(data.rlsPolicies)) {
        data.rlsPolicies.forEach(function (item) {
          rows.push({
            objectRef: "rlsPolicy:" + item.schemaName + "." + item.tableName + "." + item.policyName,
            objectType: "rlsPolicy",
            schema: item.schemaName,
            name: item.tableName + "." + item.policyName,
            parentName: item.tableName,
            status: "inspected",
            operation: "",
            warnings: [],
            source: "PostgreSQL inspect",
            target: "Read-only catalog view",
            raw: item
          });
        });
      }
    }

    if (Array.isArray(data.tableResults)) {
      data.tableResults.forEach(function (table) {
        const tableIdentity = identityFromTableName(table.tableName);
        rows.push({
          objectRef: "referenceDataTable:" + table.tableName,
          objectType: "referenceDataTable",
          schema: tableIdentity.schema,
          name: tableIdentity.name,
          status: referenceTableStatus(table),
          operation: "read-only compare",
          warnings: table.warnings || [],
          source: "repository reference-data",
          target: "postgresql",
          raw: table
        });
        if (Array.isArray(table.rowResults)) {
          table.rowResults.forEach(function (row) {
            const rawRow = Object.assign({}, row, {
              keyColumns: table.keyColumns || [],
              ignoredColumns: table.ignoredColumns || [],
              maskedColumns: row.maskedColumns || table.maskedColumns || [],
              tableCounts: table.rowCounts || {}
            });
            rows.push({
              objectRef: "referenceDataRow:" + table.tableName + ":" + textOrEmpty(row.rowKey),
              objectType: "referenceDataRow",
              schema: tableIdentity.schema,
              name: tableIdentity.name + " " + textOrEmpty(row.rowKey),
              status: row.classification || "row",
              operation: "read-only compare",
              warnings: row.warnings || [],
              source: "repository reference-data",
              target: "postgresql",
              raw: rawRow
            });
          });
        }
      });
    }
    rows.forEach(function (row) {
      row.resultOperation = label;
      row.producingWorkflowMode = workflowModeForOperation(label);
      row.workflowMode = row.producingWorkflowMode;
    });
    return rows;
  }

  function workflowModeForOperation(label) {
    if (label === "Inspect") {
      return "inspect";
    }
    if (label === "Database to Repository Preview" || label === "Database to Repository Write") {
      return "schemaDatabaseToRepository";
    }
    if (label === "Reference-data compare") {
      return "referenceDataRepoToDatabase";
    }
    if (label === "Reference-data database tables" || label === "Reference-data export preview" || label === "Reference-data export write") {
      return "referenceDataDatabaseToRepository";
    }
    return "schemaRepoToDatabase";
  }

  function rowMatchesObjectTypeFilter(row) {
    const filter = value("object-type-filter");
    if (filter === "all") {
      return true;
    }
    if (filter === "referenceData") {
      return row.objectType === "referenceDataTable" || row.objectType === "referenceDataRow";
    }
    if (filter === "unknown") {
      return row.objectType === "unknown" || row.status === "skipped";
    }
    return row.objectType === filter;
  }

  function rowMatchesStatusFilter(row) {
    const filter = value("status-filter");
    return !filter || filter === "all" || row.status === filter;
  }

  function rowMatchesFilter(row) {
    return rowMatchesObjectTypeFilter(row) && rowMatchesStatusFilter(row);
  }

  function statusSortRank(status) {
    const ranks = {
      repoDifferent: 0,
      plannedUpdate: 1,
      plannedCreate: 2,
      repoOnly: 3,
      databaseOnly: 4,
      blocked: 5,
      error: 6,
      skipped: 7,
      inspected: 8,
      inSync: 9
    };
    return Object.prototype.hasOwnProperty.call(ranks, status) ? ranks[status] : 20;
  }

  function compareResultRows(left, right) {
    const statusCompare = statusSortRank(left.status) - statusSortRank(right.status);
    if (statusCompare !== 0) {
      return statusCompare;
    }
    return [left.objectType, left.schema, left.name].join(".").localeCompare([right.objectType, right.schema, right.name].join("."));
  }

  function renderResults(rows, keepUnderlying) {
    const body = byId("results-body");
    body.innerHTML = "";
    if (!keepUnderlying) {
      state.rows = rows;
    }
    const visibleRows = state.rows.filter(function (row) {
      return rowMatchesFilter(row) && rowMatchesCurrentWorkflow(row);
    }).slice().sort(compareResultRows);
    state.visibleRows = visibleRows;
    if (state.selectedIndex < 0 || state.selectedIndex >= visibleRows.length) {
      state.selectedIndex = visibleRows.length ? 0 : -1;
    }
    visibleRows.forEach(function (row, visibleIndex) {
      const sourceIndex = state.rows.indexOf(row);
      const ref = rowRef(row, sourceIndex);
      if (!state.included.has(ref)) {
        state.included.add(ref);
      }
      const tr = document.createElement("tr");
      tr.dataset.index = String(visibleIndex);
      if (textOrEmpty(row.name).toLowerCase() === "actor") {
        tr.setAttribute("data-testid", "results-row-actor");
      }
      tr.classList.toggle("selected", visibleIndex === state.selectedIndex);
      const include = document.createElement("input");
      include.type = "checkbox";
      include.setAttribute("data-testid", "results-include-checkbox");
      include.checked = state.included.has(ref);
      include.addEventListener("change", function (event) {
        event.stopPropagation();
        if (include.checked) {
          state.included.add(ref);
        } else {
          state.included.delete(ref);
        }
        renderReleasePlan();
        updateResultCounts();
      });

      [
        include,
        row.objectType,
        row.schema,
        row.name,
        statusBadge(row.status),
        row.operation || "",
        Array.isArray(row.warnings) ? row.warnings.length : textOrEmpty(row.warnings),
      ].forEach(function (value, cellIndex) {
        const td = document.createElement("td");
        if (visibleIndex === 0 && cellIndex === 0) {
          td.setAttribute("data-testid", "results-row-first-selectable");
        }
        if (value instanceof HTMLElement) {
          td.appendChild(value);
        } else {
          td.textContent = textOrEmpty(value);
        }
        tr.appendChild(td);
      });

      tr.addEventListener("click", function () {
        state.selectedIndex = visibleIndex;
        renderResults(state.rows, true);
        renderSelectedObject();
        showStep(row.objectType === "referenceDataRow" ? "results" : "object-diff");
      });
      body.appendChild(tr);
    });

    if (!visibleRows.length) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 7;
      td.textContent = "No object rows are available in the latest response. Use Reports / Raw JSON for the full service response.";
      tr.appendChild(td);
      body.appendChild(tr);
    }

    updateResultCounts();
    renderSelectedObject();
    renderReleasePlan();
  }

  function statusBadge(status) {
    const span = document.createElement("span");
    span.className = "status-badge " + statusClass(status);
    span.textContent = textOrEmpty(status) || "unknown";
    return span;
  }

  function updateResultCounts() {
    const visibleRefs = state.visibleRows.map(function (row) {
      return rowRef(row, state.rows.indexOf(row));
    });
    const visibleIncluded = visibleRefs.filter(function (ref) {
      return state.included.has(ref);
    }).length;
    byId("results-count").textContent = state.visibleRows.length + " visible of " + state.rows.length + " row(s)";
    byId("included-count").textContent = visibleIncluded + " included in filter";
  }

  function clearObjectDiffDetails(message) {
    state.selectedObjectDdl = null;
    byId("source-detail").className = "";
    byId("target-detail").className = "";
    byId("source-detail").textContent = message || "DDL not available yet for this object.";
    byId("target-detail").textContent = message || "DDL not available yet for this object.";
    byId("related-object-comparison-body").innerHTML = "<tr><td colspan=\"5\">No related object comparison loaded.</td></tr>";
    byId("source-related-objects").textContent = "No related object details loaded.";
    byId("target-related-objects").textContent = "No related object details loaded.";
    byId("selected-json").textContent = "{}";
    updateDdlComparisonStatus(null, null);
  }

  function setObjectDiffMode(mode) {
    state.objectDiffMode = mode || "fullContext";
    document.querySelectorAll("[data-diff-mode]").forEach(function (button) {
      button.classList.toggle("active", button.dataset.diffMode === state.objectDiffMode);
    });
    renderLoadedObjectDiff();
  }

  function objectDdlSection(detail, mode) {
    if (!detail) {
      return {};
    }
    if (mode === "objectOnly") {
      return detail.objectOnly || {
        repositoryDdl: detail.repositoryDdl || "",
        databaseDdl: detail.databaseDdl || ""
      };
    }
    if (mode === "fullContext") {
      return detail.fullContext || detail.objectOnly || {
        repositoryDdl: detail.repositoryDdl || "",
        databaseDdl: detail.databaseDdl || ""
      };
    }
    return {};
  }

  function ddlByDirection(section, direction) {
    const repositoryDdl = section.repositoryDdl || "";
    const databaseDdl = section.databaseDdl || "";
    const ddlBySide = { repository: repositoryDdl, database: databaseDdl, "": "" };
    return {
      sourceDdl: ddlBySide[direction.sourceDdlSide] || "",
      targetDdl: ddlBySide[direction.targetDdlSide] || ""
    };
  }

  function relatedObjectsForSide(detail, side) {
    const related = detail && detail.relatedObjects ? detail.relatedObjects : {};
    return Array.isArray(related[side]) ? related[side] : [];
  }

  function relatedObjectGroup(item) {
    return textOrEmpty(item && item.group) || "Details";
  }

  function relatedObjectName(item) {
    return textOrEmpty(item && item.name) || "Not available in Private Beta";
  }

  function relatedObjectDetail(item) {
    if (!item) {
      return "";
    }
    const name = relatedObjectName(item);
    const detail = textOrEmpty(item.detail);
    if (name === "Not available in Private Beta") {
      return detail || name;
    }
    return [name, detail].filter(Boolean).join(" - ") || "Not available in Private Beta";
  }

  function relatedObjectIdentity(item) {
    return relatedObjectGroup(item) + "::" + relatedObjectName(item);
  }

  function relatedObjectComparisonStatus(sourceDetail, targetDetail) {
    if (!sourceDetail && !targetDetail) {
      return "Not available";
    }
    if (sourceDetail && !targetDetail) {
      return "Source only";
    }
    if (!sourceDetail && targetDetail) {
      return "Target only";
    }
    if (sourceDetail === "Not available in Private Beta" && targetDetail === "Not available in Private Beta") {
      return "Not available";
    }
    return sourceDetail === targetDetail ? "In sync" : "Different";
  }

  function relatedObjectComparisonRows(detail, direction) {
    const sourceItems = relatedObjectsForSide(detail, direction.sourceDdlSide);
    const targetItems = relatedObjectsForSide(detail, direction.targetDdlSide);
    const rowsByKey = {};
    function add(side, item) {
      const key = relatedObjectIdentity(item);
      if (!rowsByKey[key]) {
        rowsByKey[key] = {
          group: relatedObjectGroup(item),
          name: relatedObjectName(item),
          sourceDetail: "",
          targetDetail: ""
        };
      }
      rowsByKey[key][side + "Detail"] = relatedObjectDetail(item);
    }
    sourceItems.forEach(function (item) {
      add("source", item);
    });
    targetItems.forEach(function (item) {
      add("target", item);
    });
    const rows = Object.keys(rowsByKey).sort().map(function (key) {
      const row = rowsByKey[key];
      row.status = relatedObjectComparisonStatus(row.sourceDetail, row.targetDetail);
      return row;
    });
    if (!rows.length) {
      rows.push({
        group: "Details",
        name: "Not available in Private Beta",
        status: "Not available",
        sourceDetail: "Not available in Private Beta",
        targetDetail: "Not available in Private Beta"
      });
    }
    return rows;
  }

  function renderRelatedObjectComparison(rows) {
    const body = byId("related-object-comparison-body");
    body.innerHTML = "";
    rows.forEach(function (row) {
      const tr = document.createElement("tr");
      [row.group, row.name, row.status, row.sourceDetail || "--", row.targetDetail || "--"].forEach(function (value, index) {
        const td = document.createElement("td");
        td.textContent = value;
        if (index === 2) {
          td.className = "related-object-status-cell";
        }
        tr.appendChild(td);
      });
      body.appendChild(tr);
    });
  }

  function relatedObjectRowsByGroup(rows) {
    const groups = {};
    rows.forEach(function (row) {
      const group = row.group || "Details";
      if (!groups[group]) {
        groups[group] = [];
      }
      groups[group].push(row);
    });
    return Object.keys(groups).sort().map(function (group) {
      return {
        group: group,
        rows: groups[group].sort(function (left, right) {
          return String(left.name).localeCompare(String(right.name));
        })
      };
    });
  }

  function renderRelatedObjectList(targetId, rows, side) {
    const target = byId(targetId);
    target.innerHTML = "";
    const sideLabel = side === "source" ? "source" : "target";
    relatedObjectRowsByGroup(rows).forEach(function (group) {
      const wrapper = document.createElement("div");
      wrapper.className = "related-object-group related-object-type-group";
      const heading = document.createElement("h4");
      heading.textContent = group.group;
      wrapper.appendChild(heading);
      const table = document.createElement("table");
      table.className = "related-object-type-table";
      const thead = document.createElement("thead");
      const header = document.createElement("tr");
      ["Object", "Status", "Detail"].forEach(function (label) {
        const th = document.createElement("th");
        th.textContent = label;
        header.appendChild(th);
      });
      thead.appendChild(header);
      table.appendChild(thead);
      const tbody = document.createElement("tbody");
      group.rows.forEach(function (row) {
        const detailValue = row[side + "Detail"] || "";
        const tr = document.createElement("tr");
        const objectCell = document.createElement("td");
        objectCell.textContent = detailValue ? row.name : "--";
        tr.appendChild(objectCell);
        const statusCell = document.createElement("td");
        const status = document.createElement("span");
        status.className = "related-object-status";
        status.textContent = row.status;
        statusCell.appendChild(status);
        tr.appendChild(statusCell);
        const detailCell = document.createElement("td");
        detailCell.textContent = detailValue || ("No " + sideLabel + " " + group.group + " available.");
        tr.appendChild(detailCell);
        tbody.appendChild(tr);
      });
      table.appendChild(tbody);
      wrapper.appendChild(table);
      target.appendChild(wrapper);
    });
  }

  function ddlLines(value) {
    const text = value == null ? "" : String(value).replace(/\r\n/g, "\n").replace(/\r/g, "\n");
    if (!text) {
      return [];
    }
    return text.split("\n").map(function (line) {
      return line.replace(/\s+$/g, "");
    });
  }

  function alignedLineDiff(sourceText, targetText) {
    const source = ddlLines(sourceText);
    const target = ddlLines(targetText);
    const rows = [];
    const dp = Array(source.length + 1).fill(null).map(function () {
      return Array(target.length + 1).fill(0);
    });
    for (let i = source.length - 1; i >= 0; i -= 1) {
      for (let j = target.length - 1; j >= 0; j -= 1) {
        dp[i][j] = source[i] === target[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
      }
    }
    let i = 0;
    let j = 0;
    while (i < source.length || j < target.length) {
      if (i < source.length && j < target.length && source[i] === target[j]) {
        rows.push({ source: source[i], target: target[j], type: "same" });
        i += 1;
        j += 1;
      } else if (j < target.length && (i >= source.length || dp[i][j + 1] >= dp[i + 1][j])) {
        rows.push({ source: "", target: target[j], type: "targetOnly" });
        j += 1;
      } else if (i < source.length) {
        rows.push({ source: source[i], target: "", type: "sourceOnly" });
        i += 1;
      }
    }
    return rows;
  }

  function diffClass(type) {
    if (type === "same") {
      return "diff-line-same";
    }
    if (type === "targetOnly") {
      return "diff-line-target-only";
    }
    if (type === "sourceOnly") {
      return "diff-line-source-only";
    }
    return "diff-line-different";
  }

  function diffTitle(type, side) {
    if (type === "same") {
      return "Matched line";
    }
    if (type === "targetOnly") {
      return side === "target" ? "Target-only line. Missing from source." : "Blank counterpart for target-only line.";
    }
    if (type === "sourceOnly") {
      return side === "source" ? "Source-only line. Missing from target." : "Blank counterpart for source-only line.";
    }
    return "Different line";
  }


  function appendVisualDiffLine(target, marker, lineText) {
    const markerElement = document.createElement("span");
    markerElement.className = "diff-marker";
    markerElement.setAttribute("aria-hidden", "true");
    markerElement.title = marker ? "Display-only diff marker" : "";
    markerElement.textContent = marker;

    const textElement = document.createElement("span");
    textElement.className = "diff-line-text";
    textElement.textContent = lineText;

    target.appendChild(markerElement);
    target.appendChild(textElement);
  }
  function renderDdlLineDiff(sourceTarget, targetTarget, sourceDdl, targetDdl) {
    sourceTarget.innerHTML = "";
    targetTarget.innerHTML = "";
    sourceTarget.className = "diff-line-grid";
    targetTarget.className = "diff-line-grid";
    const rows = alignedLineDiff(sourceDdl, targetDdl);
    if (!rows.length) {
      sourceTarget.textContent = "DDL not available yet for this object.";
      targetTarget.textContent = "DDL not available yet for this object.";
      return;
    }
    rows.forEach(function (row) {
      const sourceLine = document.createElement("div");
      const targetLine = document.createElement("div");
      const className = "diff-line-row " + diffClass(row.type);
      sourceLine.className = className;
      targetLine.className = className;
      sourceLine.title = diffTitle(row.type, "source");
      targetLine.title = diffTitle(row.type, "target");
      sourceLine.setAttribute("aria-label", sourceLine.title);
      targetLine.setAttribute("aria-label", targetLine.title);
      appendVisualDiffLine(sourceLine, row.type === "sourceOnly" && row.source ? "+" : "", row.source || " ");
      appendVisualDiffLine(targetLine, row.type === "targetOnly" && row.target ? "-" : "", row.target || " ");
      sourceTarget.appendChild(sourceLine);
      targetTarget.appendChild(targetLine);
    });
  }

  function renderLoadedObjectDiff() {
    const row = state.visibleRows[state.selectedIndex];
    if (!row || !state.selectedObjectDdl) {
      return;
    }
    const direction = directionForResultRow(row);
    const mode = state.objectDiffMode || "fullContext";
    const showRelated = mode === "relatedObjects";
    const showRaw = mode === "rawDetails";
    byId("ddl-diff-view").hidden = showRelated || showRaw;
    byId("related-objects-view").hidden = !showRelated;
    byId("selected-json").parentElement.hidden = false;

    if (showRelated) {
      byId("ddl-comparison-status").textContent = "DDL unavailable";
      byId("ddl-comparison-status").className = "ddl-comparison-status ddl-unavailable";
      const rows = relatedObjectComparisonRows(state.selectedObjectDdl, direction);
      renderRelatedObjectComparison(rows);
      renderRelatedObjectList("source-related-objects", rows, "source");
      renderRelatedObjectList("target-related-objects", rows, "target");
      byId("selected-json").textContent = redactedJson({
        selected: objectDiffDisplayPayload(row, direction),
        objectDdl: state.selectedObjectDdl
      });
      return;
    }

    if (showRaw) {
      byId("ddl-diff-view").hidden = true;
      byId("related-objects-view").hidden = true;
      byId("ddl-comparison-status").textContent = "DDL unavailable";
      byId("ddl-comparison-status").className = "ddl-comparison-status ddl-unavailable";
      byId("selected-json").textContent = redactedJson({
        selected: objectDiffDisplayPayload(row, direction),
        objectDdl: state.selectedObjectDdl
      });
      return;
    }

    const label = mode === "objectOnly" ? "Object Only DDL" : "Full Context DDL";
    byId("source-ddl-heading").textContent = "Source DDL - " + label;
    byId("target-ddl-heading").textContent = "Target DDL - " + label;
    const section = objectDdlSection(state.selectedObjectDdl, mode);
    const ddl = ddlByDirection(section, direction);
    const unavailable = "DDL not available yet for this object.";
    if (!ddl.sourceDdl && !ddl.targetDdl) {
      byId("source-detail").className = "";
      byId("target-detail").className = "";
      byId("source-detail").textContent = unavailable;
      byId("target-detail").textContent = unavailable;
    } else {
      renderDdlLineDiff(byId("source-detail"), byId("target-detail"), ddl.sourceDdl || "", ddl.targetDdl || "");
    }
    updateDdlComparisonStatus(ddl.sourceDdl, ddl.targetDdl);
    byId("selected-json").textContent = redactedJson({
      selected: objectDiffDisplayPayload(row, direction),
      objectDdl: state.selectedObjectDdl
    });
  }

  function renderSelectedObject() {
    const row = state.visibleRows[state.selectedIndex];
    const direction = directionForResultRow(row);
    renderReferenceDataRowDetail(row);
    setObjectDiffDdlTestIds(direction);
    byId("source-type-label").textContent = direction.sourceType;
    byId("target-type-label").textContent = direction.targetType;
    if (!row) {
      byId("selected-object-title").textContent = "No object selected";
      byId("selected-object-summary").innerHTML = "";
      clearObjectDiffDetails("DDL not available yet for this object.");
      return;
    }
    if (!rowMatchesCurrentWorkflow(row)) {
      const message = "Selected result belongs to a different workflow. Run the current workflow again.";
      byId("selected-object-title").textContent = "No object selected";
      updateSummary("selected-object-summary", {
        message: message
      });
      clearObjectDiffDetails("DDL not available yet for this object.");
      return;
    }
    byId("selected-object-title").textContent = row.objectType + ": " + row.name;
    updateSummary("selected-object-summary", {
      objectType: row.objectType,
      schema: row.schema,
      objectName: row.name,
      status: row.status,
      operation: row.resultOperation || row.operation || "review",
      warnings: Array.isArray(row.warnings) ? row.warnings.length : textOrEmpty(row.warnings),
      source: direction.sourceLabel,
      sourceType: direction.sourceType,
      target: direction.targetLabel,
      targetType: direction.targetType
    });
    state.selectedObjectDdl = null;
    setObjectDiffMode(state.objectDiffMode || "fullContext");
    byId("source-detail").textContent = "Loading DDL detail...";
    byId("target-detail").textContent = "Loading DDL detail...";
    updateDdlComparisonStatus(null, null);
    byId("selected-json").textContent = redactedJson(objectDiffDisplayPayload(row, direction));
    loadSelectedObjectDdl(row, direction);
  }

  function renderReferenceDataRowDetail(row) {
    const target = byId("reference-data-row-detail-summary");
    if (!target) {
      return;
    }
    target.innerHTML = "";
    if (!row || row.objectType !== "referenceDataRow") {
      updateSummary("reference-data-row-detail-summary", {
        selectedRow: "none",
        guidance: "Select a reference-data row result to inspect read-only row metadata."
      });
      return;
    }
    const raw = row.raw || {};
    const maskedColumns = Array.isArray(raw.maskedColumns) ? raw.maskedColumns : [];
    const changedColumns = Array.isArray(raw.changedColumns) ? raw.changedColumns : [];
    const ignoredColumns = Array.isArray(raw.ignoredColumns) ? raw.ignoredColumns : [];
    updateSummary("reference-data-row-detail-summary", {
      table: raw.tableName || row.schema + "." + row.name,
      keyValues: raw.rowKey || "",
      classification: raw.classification || row.status,
      changedColumns: changedColumns.length ? changedColumns.join(", ") : "[none]",
      repositoryValue: maskedColumns.length ? "[masked]" : "not exposed in UI response",
      databaseValue: maskedColumns.length ? "[masked]" : "not exposed in UI response",
      maskedColumns: maskedColumns.length ? maskedColumns.join(", ") : "[none]",
      ignoredColumns: ignoredColumns.length ? ignoredColumns.join(", ") + " ignored for comparison" : "[none]"
    });
  }

  function objectDiffDisplayPayload(row, direction) {
    const payload = Object.assign({}, row.raw || row);
    payload.operation = row.resultOperation || row.operation || payload.operation || "review";
    payload.producingWorkflowMode = row.producingWorkflowMode || workflowModeForOperation(payload.operation);
    payload.source = direction.sourceLabel;
    payload.sourceType = direction.sourceType;
    payload.target = direction.targetLabel;
    payload.targetType = direction.targetType;
    return payload;
  }

  function setObjectDiffDdlTestIds(direction) {
    const sourceDetail = byId("source-detail");
    const targetDetail = byId("target-detail");
    sourceDetail.setAttribute("data-testid", direction.sourceDdlSide === "database" ? "object-diff-database" : "object-diff-repository");
    targetDetail.setAttribute("data-testid", direction.targetDdlSide === "repository" ? "object-diff-repository" : "object-diff-database");
  }

  function directionForResultRow(row) {
    let mode = row && row.producingWorkflowMode ? row.producingWorkflowMode : "";
    if (!mode && row && row.workflowMode) {
      mode = row.workflowMode;
    }
    const operation = row && row.resultOperation ? row.resultOperation : state.lastOperation;
    if (!mode) {
      mode = workflowModeForOperation(operation);
    }
    if (isSchemaRepositoryToDatabaseMode(mode) && row && isDatabaseToRepositoryRow(row)) {
      mode = "schemaDatabaseToRepository";
    }
    return directionForWorkflowMode(mode);
  }

  function isDatabaseToRepositoryMode(mode) {
    return isSchemaDatabaseToRepositoryMode(mode);
  }

  function isSchemaRepositoryToDatabaseMode(mode) {
    return mode === "schemaRepoToDatabase" || mode === "compare";
  }

  function isSchemaDatabaseToRepositoryMode(mode) {
    return mode === "schemaDatabaseToRepository" || mode === "databaseToRepository" || mode === "dbToRepo";
  }

  function isReferenceDataRepositoryToDatabaseMode(mode) {
    return mode === "referenceDataRepoToDatabase" || mode === "data";
  }

  function isReferenceDataDatabaseToRepositoryMode(mode) {
    return mode === "referenceDataDatabaseToRepository";
  }

  function currentWorkflowMode() {
    return value("workflow-mode") || "schemaRepoToDatabase";
  }

  function workflowModesMatch(rowMode, selectedMode) {
    if (rowMode === "inspect" && isSchemaRepositoryToDatabaseMode(selectedMode)) {
      return true;
    }
    if (isSchemaRepositoryToDatabaseMode(rowMode) && isSchemaRepositoryToDatabaseMode(selectedMode)) {
      return true;
    }
    if (isSchemaDatabaseToRepositoryMode(rowMode) && isSchemaDatabaseToRepositoryMode(selectedMode)) {
      return true;
    }
    if (isReferenceDataRepositoryToDatabaseMode(rowMode) && isReferenceDataRepositoryToDatabaseMode(selectedMode)) {
      return true;
    }
    if (isReferenceDataDatabaseToRepositoryMode(rowMode) && isReferenceDataDatabaseToRepositoryMode(selectedMode)) {
      return true;
    }
    return textOrEmpty(rowMode || "schemaRepoToDatabase") === textOrEmpty(selectedMode || "schemaRepoToDatabase");
  }

  function rowMatchesCurrentWorkflow(row) {
    return rowMatchesWorkflowMode(row, currentWorkflowMode());
  }

  function rowMatchesWorkflowMode(row, selectedMode) {
    if (!row) {
      return false;
    }
    const rowMode = row.producingWorkflowMode || row.workflowMode || workflowModeForOperation(row.resultOperation || row.operation || state.lastOperation);
    return workflowModesMatch(rowMode, selectedMode);
  }

  function isDatabaseToRepositoryRow(row) {
    const operation = textOrEmpty(row.resultOperation);
    const repositorySyncStatuses = ["added", "changed", "unchanged", "plannedCreate", "plannedUpdate", "created", "updated"];
    return operation.indexOf("Database to Repository") === 0 || repositorySyncStatuses.indexOf(row.status) >= 0 || textOrEmpty(row.source) === "PostgreSQL database";
  }

  function directionForWorkflowMode(mode) {
    if (mode === "inspect") {
      return {
        sourceLabel: "PostgreSQL database",
        sourceType: "Database",
        targetLabel: "Read-only catalog view",
        targetType: "Catalog",
        sourceDdlSide: "database",
        targetDdlSide: ""
      };
    }
    if (isSchemaDatabaseToRepositoryMode(mode)) {
      return {
        sourceLabel: "PostgreSQL database",
        sourceType: "Database",
        targetLabel: "Repository desired state",
        targetType: "Repository",
        sourceDdlSide: "database",
        targetDdlSide: "repository"
      };
    }
    if (isReferenceDataRepositoryToDatabaseMode(mode)) {
      return {
        sourceLabel: "Repository reference-data",
        sourceType: "Repository",
        targetLabel: "PostgreSQL target",
        targetType: "Database",
        sourceDdlSide: "repository",
        targetDdlSide: "database"
      };
    }
    if (isReferenceDataDatabaseToRepositoryMode(mode)) {
      return {
        sourceLabel: "PostgreSQL database",
        sourceType: "Database",
        targetLabel: "Repository reference-data",
        targetType: "Repository",
        sourceDdlSide: "database",
        targetDdlSide: "repository"
      };
    }
    return {
      sourceLabel: "Repository desired state",
      sourceType: "Repository",
      targetLabel: "PostgreSQL database",
      targetType: "Database",
      sourceDdlSide: "repository",
      targetDdlSide: "database"
    };
  }

  function normalizeDdlForComparison(ddl) {
    if (!ddl) {
      return "";
    }
    return String(ddl).replace(/\r\n/g, "\n").replace(/\r/g, "\n").replace(/\n{3,}/g, "\n\n").trim();
  }

  function updateDdlComparisonStatus(sourceDdl, targetDdl) {
    const status = byId("ddl-comparison-status");
    status.className = "ddl-comparison-status";
    const source = normalizeDdlForComparison(sourceDdl);
    const target = normalizeDdlForComparison(targetDdl);
    if (!source || !target) {
      status.textContent = "DDL unavailable";
      status.classList.add("ddl-unavailable");
      return;
    }
    if (source === target) {
      status.textContent = "Similar";
      status.classList.add("ddl-similar");
    } else {
      status.textContent = "Different";
      status.classList.add("ddl-different");
    }
  }

  function quoteIdentifier(identifier) {
    return "\"" + textOrEmpty(identifier).replace(/"/g, "\"\"") + "\"";
  }

  function clientSchemaDdl(row) {
    const schema = row.schema || row.name;
    if (!schema) {
      return "";
    }
    return "-- DbState PostgreSQL desired-state object\n-- Object type: schema\n-- Object name: " + schema + "\n\nCREATE SCHEMA " + quoteIdentifier(schema) + ";\n";
  }

  function clientTableDdl(row) {
    const columns = columnsForSelectedTable(row);
    if (!columns.length) {
      return "";
    }
    const lines = [];
    lines.push("-- DbState PostgreSQL desired-state object");
    lines.push("-- Object type: table");
    lines.push("-- Object name: " + row.schema + "." + row.name);
    lines.push("");
    lines.push("CREATE TABLE " + quoteIdentifier(row.schema) + "." + quoteIdentifier(row.name) + " (");
    columns.forEach(function (column, index) {
      const comma = index + 1 === columns.length ? "" : ",";
      const nullable = column.isNullable === false ? " NOT NULL" : "";
      lines.push("    " + quoteIdentifier(column.columnName) + " " + textOrEmpty(column.dataType) + nullable + comma);
    });
    lines.push(");");
    return lines.join("\n") + "\n";
  }

  function clientDatabaseDdl(row) {
    if (row.objectType === "schema") {
      return clientSchemaDdl(row);
    }
    if (row.objectType === "table") {
      return clientTableDdl(row);
    }
    return "";
  }

  function objectDdlRequest(row) {
    let objectName = row.name || "";
    if (row.objectType === "index") {
      const tableName = row.parentName || (row.raw && row.raw.tableName) || "";
      if (tableName && objectName.indexOf(".") < 0) {
        objectName = tableName + "." + objectName;
      }
    }
    if (row.objectType === "constraint") {
      const tableName = row.parentName || (row.raw && row.raw.tableName) || "";
      if (tableName && objectName.indexOf(".") < 0) {
        objectName = tableName + "." + objectName;
      }
    }
    return attachWorkspacePath(attachConnection({
      objectType: row.objectType,
      schema: row.schema || "",
      objectName: objectName,
      relativePath: row.relativePath || ""
    }));
  }

  async function loadSelectedObjectDdl(row, direction) {
    if (row.objectType === "referenceDataRow" || row.objectType === "referenceDataTable") {
      state.selectedObjectDdl = {
        repositoryDdl: "",
        databaseDdl: "",
        objectOnly: {},
        fullContext: {},
        relatedObjects: { repository: [], database: [] },
        warnings: ["Reference-data compare rows are read-only data results, not DDL objects."]
      };
      renderLoadedObjectDiff();
      return;
    }
    let detail = {};
    try {
      detail = await requestJson(approvedEndpoints.objectDdl, objectDdlRequest(row));
    } catch (error) {
      detail = { repositoryDdl: "", databaseDdl: "", objectOnly: {}, fullContext: {}, relatedObjects: {}, warnings: [error.message || "DDL detail request failed."] };
    }

    if (!detail.databaseDdl && (row.objectType === "schema" || row.objectType === "table")) {
      detail.databaseDdl = clientDatabaseDdl(row);
    }
    if (!detail.objectOnly) {
      detail.objectOnly = {};
    }
    if (!detail.objectOnly.databaseDdl && detail.databaseDdl) {
      detail.objectOnly.databaseDdl = detail.databaseDdl;
    }
    if (!detail.objectOnly.repositoryDdl && detail.repositoryDdl) {
      detail.objectOnly.repositoryDdl = detail.repositoryDdl;
    }
    if (!detail.fullContext) {
      detail.fullContext = {};
    }
    if (!detail.fullContext.databaseDdl && detail.objectOnly.databaseDdl) {
      detail.fullContext.databaseDdl = detail.objectOnly.databaseDdl;
    }
    if (!detail.fullContext.repositoryDdl && detail.objectOnly.repositoryDdl) {
      detail.fullContext.repositoryDdl = detail.objectOnly.repositoryDdl;
    }
    if (!detail.relatedObjects) {
      detail.relatedObjects = { repository: [], database: [] };
    }
    state.selectedObjectDdl = detail;
    renderLoadedObjectDiff();
  }

  function objectDiffDirectionRegressionFixture() {
    const staleRow = {
      producingWorkflowMode: "databaseToRepository",
      workflowMode: "databaseToRepository",
      resultOperation: "Database to Repository Preview",
      sourceType: "Repository",
      targetType: "Database",
      status: "changed",
      source: "Repository desired state",
      target: "PostgreSQL database",
      repositoryDdl: "-- repo ddl",
      databaseDdl: "-- db ddl"
    };
    const direction = directionForResultRow(staleRow);
    const displayPayload = objectDiffDisplayPayload(staleRow, direction);
    const ddlBySide = { repository: staleRow.repositoryDdl, database: staleRow.databaseDdl, "": "" };
    const sourceDdl = ddlBySide[direction.sourceDdlSide] || "";
    const targetDdl = ddlBySide[direction.targetDdlSide] || "";
    return {
      operation: staleRow.resultOperation,
      source: displayPayload.source,
      sourceType: displayPayload.sourceType,
      target: displayPayload.target,
      targetType: displayPayload.targetType,
      sourceDdl: sourceDdl,
      targetDdl: targetDdl,
      renderedText: [
        "Operation: " + displayPayload.operation,
        "source " + displayPayload.source,
        "sourceType " + displayPayload.sourceType,
        "target " + displayPayload.target,
        "targetType " + displayPayload.targetType,
        "Source type: " + direction.sourceType,
        "Target type: " + direction.targetType,
        "Source DDL " + sourceDdl,
        "Target DDL " + targetDdl
      ].join("\n")
    };
  }

  function staleCompareResultInDatabaseToRepositoryFixture() {
    const staleCompareRow = {
      producingWorkflowMode: "compare",
      workflowMode: "compare",
      resultOperation: "Compare",
      operation: "Compare",
      objectType: "table",
      schema: "core",
      name: "parking_sessions",
      source: "Repository desired state",
      sourceType: "Repository",
      target: "PostgreSQL database",
      targetType: "Database"
    };
    const selectedMode = "databaseToRepository";
    const canRender = rowMatchesWorkflowMode(staleCompareRow, selectedMode);
    return {
      selectedWorkflowMode: selectedMode,
      rowOperation: staleCompareRow.operation,
      rowProducingWorkflowMode: staleCompareRow.producingWorkflowMode,
      canRender: canRender,
      message: canRender ? "" : "Selected result belongs to a different workflow. Run the current workflow again.",
      visibleRows: canRender ? 1 : 0
    };
  }

  if (typeof window !== "undefined") {
    window.dbstateUiTestHooks = {
      objectDiffDirectionRegressionFixture: objectDiffDirectionRegressionFixture,
      staleCompareResultInDatabaseToRepositoryFixture: staleCompareResultInDatabaseToRepositoryFixture
    };
  }

  function columnsForSelectedTable(row) {
    if (!row || row.objectType !== "table") {
      return [];
    }
    return state.inspectColumns.filter(function (column) {
      return column.schemaName === row.schema && column.tableName === row.name;
    }).sort(function (left, right) {
      return (left.ordinalPosition || 0) - (right.ordinalPosition || 0);
    });
  }

  function projectStructureGuidance(message) {
    const text = textOrEmpty(message);
    if (text.indexOf("DbState PostgreSQL project structure is incomplete") >= 0 || text.indexOf("Run dbstate init first") >= 0) {
      return text + " This workspace is a Git repository but not yet an initialized DbState project. Run dbstate init from this workspace, then export or sync desired state before comparing.";
    }
    return text;
  }

  function renderWarnings(data) {
    const warnings = [];
    function addMany(values, label, kind) {
      if (!Array.isArray(values)) {
        return;
      }
      values.forEach(function (item) {
        const text = typeof item === "string" ? projectStructureGuidance(item) : redactedJson(item);
        warnings.push({
          kind: kind || "warning",
          text: label + ": " + text
        });
      });
    }
    addMany(data.warnings, "Warning");
    addMany(data.errors, "Error", "error");
    addMany(data.dependencyWarnings, "Dependency warning");
    addMany(data.blockedItems, "Blocked item", "error");
    addMany(data.deferredObjectTypes, "Deferred object type");

    const target = byId("warnings-list");
    target.innerHTML = "";
    if (!warnings.length) {
      target.textContent = "No warnings yet.";
      return;
    }
    warnings.forEach(function (warning) {
      const item = document.createElement("div");
      item.className = "issue-item" + (warning.kind === "error" ? " error" : "");
      item.textContent = warning.text;
      target.appendChild(item);
    });
  }

  function updateResultsContext(label, data) {
    byId("results-operation").textContent = label || "none";
    if (!label || label === "none") {
      byId("results-source").textContent = "none";
      byId("results-target").textContent = "none";
      return;
    }
    if (label === "Inspect") {
      byId("results-source").textContent = "PostgreSQL inspect";
      byId("results-target").textContent = "Read-only catalog view";
      return;
    }
    if (label === "Database to Repository Preview" || label === "Database to Repository Write") {
      byId("results-source").textContent = "PostgreSQL database";
      byId("results-target").textContent = data && data.repositoryPath ? "Repository desired state: " + data.repositoryPath : "Repository desired state";
      return;
    }
    if (label === "Reference-data compare") {
      byId("results-source").textContent = "Repository reference-data";
      byId("results-target").textContent = "PostgreSQL target";
      return;
    }
    byId("results-source").textContent = data && data.repositoryPath ? "Repository desired state: " + data.repositoryPath : "Repository desired state";
    byId("results-target").textContent = data && data.databaseType ? data.databaseType + " target" : "PostgreSQL target";
  }

  function clearOperationResults(reason) {
    state.lastResponse = null;
    state.lastOperation = "none";
    state.rows = [];
    state.visibleRows = [];
    state.included = new Set();
    state.selectedIndex = -1;
    state.selectedObjectDdl = null;
    state.objectDiffMode = "fullContext";
    responseSummary.textContent = reason || "Select an operation to run.";
    jsonViewer.textContent = "{}";
    byId("last-operation").textContent = "none";
    byId("last-status").textContent = "not run";
    byId("last-warnings").textContent = "0";
    byId("last-errors").textContent = "0";
    updateResultsContext("none", {});
    renderErrorSummary({ success: true, warnings: [], errors: [] });
    renderWarnings({ warnings: [], errors: [] });
    updateObjectTypeFilterOptions([], "none");
    renderResults([]);
    setObjectDiffMode("fullContext");
  }

  function renderErrorSummary(data) {
    const target = byId("results-error-summary");
    if (data && data.success === false && Array.isArray(data.errors) && data.errors.length) {
      target.hidden = false;
      target.textContent = projectStructureGuidance(data.errors.join(" "));
      return;
    }
    target.hidden = true;
    target.textContent = "";
  }

  function releaseName() {
    return value("release-name");
  }

  function releaseConfirmed() {
    return value("release-confirmation") === "GENERATE RELEASE ARTIFACTS";
  }

  function updateReleaseWriteButton() {
    const button = document.querySelector("[data-action='release-write']");
    const preview = document.querySelector("[data-action='release-preview']");
    const selectedCount = selectedReleaseObjectRefs().length;
    if (preview) {
      preview.disabled = currentWorkflowMode() !== "compare" || !releaseName() || selectedCount === 0;
    }
    if (button) {
      button.disabled = currentWorkflowMode() !== "compare" || !releaseName() || !releaseConfirmed() || selectedCount === 0;
    }
  }

  function releaseBody(write) {
    const body = attachWorkspacePath(attachConnection(buildScope()));
    body.releaseName = releaseName();
    const selected = selectedReleaseObjectRefs();
    if (!selected.length) {
      throw new Error("Select at least one release candidate.");
    }
    body.selectedObjectRefs = selected;
    body.include = selected.slice();
    body.exclude = [];
    if (write) {
      body.confirmReleaseArtifacts = true;
      body.confirmationText = value("release-confirmation");
    }
    return body;
  }

  function renderReleaseArtifactResult(data) {
    const target = byId("release-artifact-result");
    if (!target) {
      return;
    }
    target.innerHTML = "";
    const summary = document.createElement("dl");
    summary.className = "summary-list compact";
    const createdArtifacts = Array.isArray(data.createdArtifacts) ? data.createdArtifacts : [];
    const plannedArtifacts = Array.isArray(data.plannedArtifacts) ? data.plannedArtifacts : [];
    const artifacts = createdArtifacts.length ? createdArtifacts : plannedArtifacts;
    updateSummaryElement(summary, {
      success: data.success,
      releaseName: data.releaseName || releaseName(),
      dryRun: data.dryRun,
      riskLevel: data.riskLevel || "unknown",
      artifacts: artifacts.length
    });
    target.appendChild(summary);

    if (data.repositoryContext && data.repositoryContext.isDirty || data.isDirty) {
      appendReleaseMessageList(
        target,
        "Working tree must be clean before generating release artifacts",
        ["Release artifacts cannot be generated while the working tree is dirty. Commit or stash repository changes first, then run Generate Release Artifact."],
        "warning"
      );
    }
    appendReleaseMessageList(target, "Errors", data.errors, "error");
    appendReleaseMessageList(target, "Warnings", data.warnings, "warning");
    if (Array.isArray(data.riskReasons) && data.riskReasons.length) {
      appendReleaseMessageList(target, "Risk reasons", data.riskReasons, "warning");
    }

    const riskSummary = byId("release-risk-summary");
    if (riskSummary) {
      riskSummary.textContent = "Risk level: " + textOrEmpty(data.riskLevel || "unknown");
      if (data.repositoryContext && data.repositoryContext.isDirty || data.isDirty) {
        riskSummary.textContent += ". Working tree is dirty; commit or stash changes before generating release artifacts.";
      }
    }

    if (artifacts.length) {
      const list = document.createElement("ul");
      artifacts.forEach(function (artifact) {
        const item = document.createElement("li");
        const path = textOrEmpty(artifact);
        const pathSpan = document.createElement("span");
        pathSpan.textContent = path;
        item.appendChild(pathSpan);
        if (createdArtifacts.length) {
          item.appendChild(document.createTextNode(" "));
          const preview = document.createElement("button");
          preview.type = "button";
          preview.textContent = "Preview";
          preview.setAttribute("data-action", "release-artifact-preview");
          preview.setAttribute("data-artifact-path", path);
          item.appendChild(preview);
        }
        list.appendChild(item);
      });
      target.appendChild(list);
      if (!createdArtifacts.length) {
        const note = document.createElement("p");
        note.className = "note";
        note.textContent = "Preview is available after Generate Release Artifact writes review files.";
        target.appendChild(note);
      }
    }
  }

  async function previewReleaseArtifact(artifactPath) {
    const meta = byId("release-artifact-preview-meta");
    const content = byId("release-artifact-preview-content");
    if (!artifactPath) {
      throw new Error("Artifact path is required.");
    }
    meta.textContent = "Loading " + artifactPath + "...";
    content.textContent = "";
    const data = await requestJson(approvedEndpoints.releaseArtifactPreview, {
      repositoryPath: workspacePath(),
      artifactPath: artifactPath
    });
    state.lastResponse = data;
    state.lastOperation = "Release artifact preview";
    responseSummary.textContent = "Release artifact preview: " + summarize(data);
    jsonViewer.textContent = redactedJson(data);
    updateStatus("Release artifact preview", data);
    renderErrorSummary(data);
    renderWarnings(data);
    meta.textContent = [
      "File: " + textOrEmpty(data.fileName),
      "Type: " + textOrEmpty(data.artifactType),
      "Path: " + textOrEmpty(data.artifactPath),
      data.truncated ? "Preview truncated at 1 MiB." : ""
    ].filter(Boolean).join(" | ");
    content.textContent = textOrEmpty(data.content);
  }

  function appendReleaseMessageList(target, title, values, kind) {
    if (!Array.isArray(values) || !values.length) {
      return;
    }
    const box = document.createElement("div");
    box.className = "issue-item" + (kind === "error" ? " error" : "");
    const heading = document.createElement("strong");
    heading.textContent = title;
    box.appendChild(heading);
    const list = document.createElement("ul");
    values.forEach(function (value) {
      const item = document.createElement("li");
      item.textContent = textOrEmpty(value);
      list.appendChild(item);
    });
    box.appendChild(list);
    target.appendChild(box);
  }

  function updateSummaryElement(target, values) {
    target.innerHTML = "";
    Object.keys(values).forEach(function (key) {
      const dt = document.createElement("dt");
      const dd = document.createElement("dd");
      dt.textContent = key;
      dd.textContent = textOrEmpty(values[key]);
      target.appendChild(dt);
      target.appendChild(dd);
    });
  }

  function releaseCandidateRowsFromResponse(data) {
    if (!data) {
      return [];
    }
    const rows = [];
    function append(values, fallbackStatus) {
      if (!Array.isArray(values)) {
        return;
      }
      values.forEach(function (item) {
        const identity = splitIdentity(item.objectRef);
        rows.push({
          objectRef: item.objectRef || "",
          objectType: item.objectType || "",
          schema: identity.schema,
          name: identity.name,
          status: item.compareClassification || fallbackStatus,
          operation: item.planIntent || "",
          operationKind: item.operationKind || "",
          operationLabel: item.operationLabel || "",
          safetyBadge: item.safetyBadge || item.operationLabel || "",
          safetyLevel: item.safetyLevel || "",
          operationExplanation: item.operationExplanation || "",
          operationReasons: Array.isArray(item.operationReasons) ? item.operationReasons : [],
          blocked: item.blocked || fallbackStatus === "blocked",
          warnings: item.warnings || []
        });
      });
    }
    append(data.planItems, "planned");
    append(data.blockedItems, "blocked");
    return rows;
  }

  function releaseCandidateObjectRef(row, index) {
    return textOrEmpty(row.objectRef);
  }

  function releaseCandidateEligible(row) {
    return !!row.objectRef && !row.blocked && row.operation !== "blocked" && row.safetyLevel !== "blocked";
  }

  function releaseCandidateRows() {
    const metadata = {};
    releaseCandidateRowsFromResponse(state.releaseResponse).forEach(function (row) {
      if (row.objectRef) {
        metadata[row.objectRef] = row;
      }
    });
    const baseRows = state.rows.filter(function (row, index) {
      return state.included.has(rowRef(row, index));
    }).map(function (row, index) {
      const objectRef = releaseCandidateObjectRef(row, index);
      const enriched = Object.assign({}, row, { objectRef: objectRef }, metadata[objectRef] || {});
      if (metadata[objectRef]) {
        enriched.schema = metadata[objectRef].schema || row.schema;
        enriched.name = metadata[objectRef].name || row.name;
        enriched.objectType = metadata[objectRef].objectType || row.objectType;
      }
      return enriched;
    });
    if (baseRows.length) {
      return baseRows;
    }
    return releaseCandidateRowsFromResponse(state.releaseResponse);
  }

  function syncReleaseSelectionWithCandidates(rows) {
    const signature = rows.map(function (row, index) {
      return releaseCandidateObjectRef(row, index);
    }).sort().join("|");
    if (signature !== state.releaseCandidateSignature) {
      state.releaseCandidateSignature = signature;
      state.releaseSelectedRefs = new Set();
      rows.forEach(function (row, index) {
        const ref = releaseCandidateObjectRef(row, index);
        if (ref && releaseCandidateEligible(row)) {
          state.releaseSelectedRefs.add(ref);
        }
      });
    }
  }

  function selectedReleaseObjectRefs() {
    return Array.from(state.releaseSelectedRefs).sort();
  }

  function updateReleaseSelectionSummary(rows) {
    const selected = selectedReleaseObjectRefs().length;
    const eligible = rows.filter(releaseCandidateEligible).length;
    const target = byId("release-selected-count");
    if (target) {
      target.textContent = selected + " selected of " + eligible + " eligible";
    }
  }

  function renderReleasePlan() {
    const mode = currentWorkflowMode();
    const notApplicable = byId("release-plan-not-applicable");
    const content = byId("release-plan-content");
    if (!isSchemaRepositoryToDatabaseMode(mode)) {
      content.hidden = true;
      notApplicable.hidden = false;
      if (isSchemaDatabaseToRepositoryMode(mode)) {
        notApplicable.textContent = "Release Plan is not used for Schema Compare: Database to Repository. This workflow writes selected PostgreSQL object definitions into repository files under database/objects/. Use Results → Write Selected Repository Changes.";
      } else if (isReferenceDataRepositoryToDatabaseMode(mode)) {
        notApplicable.textContent = "Release Plan is not used for Reference Data Compare: Repository to Database. Reference-data compare is read-only and does not generate DML.";
      } else {
        notApplicable.textContent = "Release Plan is not used for Reference Data Compare: Database to Repository. This workflow writes selected YAML files under database/reference-data/ after typed confirmation.";
      }
      return;
    }
    content.hidden = false;
    notApplicable.hidden = true;
    const includedRows = releaseCandidateRows();
    syncReleaseSelectionWithCandidates(includedRows);
    updateReleaseSelectionSummary(includedRows);
    updateSummary("release-context", {
      workflowMode: "Schema Compare: Repository to Database",
      source: "Repository desired state",
      target: "PostgreSQL database",
      latestResult: state.lastOperation || "none",
      releaseName: releaseName() || "<required>"
    });

    const statusCounts = {};
    const typeCounts = {};
    includedRows.forEach(function (row) {
      const status = textOrEmpty(row.status || "unknown");
      const type = textOrEmpty(row.objectType || "unknown");
      statusCounts[status] = (statusCounts[status] || 0) + 1;
      typeCounts[type] = (typeCounts[type] || 0) + 1;
    });
    const summary = { selectedRows: includedRows.length };
    Object.keys(statusCounts).sort().forEach(function (status) {
      summary["status " + status] = statusCounts[status];
    });
    Object.keys(typeCounts).sort().forEach(function (type) {
      summary["type " + type] = typeCounts[type];
    });
    updateSummary("release-object-summary", summary);

    const name = releaseName() || "<release-name>";
    byId("release-dryrun-command").textContent = "dbstate release postgres --all --name " + name + " --dry-run --format json";
    byId("release-write-command").textContent = "dbstate release postgres --all --name " + name;

    const body = byId("release-candidates-body");
    body.innerHTML = "";
    if (!includedRows.length) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 10;
      td.textContent = "No selected result rows yet. Run Schema Compare: Repository to Database or Plan, then check rows in Results.";
      tr.appendChild(td);
      body.appendChild(tr);
    } else {
      includedRows.slice(0, 200).forEach(function (row, index) {
        const tr = document.createElement("tr");
        const objectRef = releaseCandidateObjectRef(row, index);
        const eligible = releaseCandidateEligible(row);
        const selectCell = document.createElement("td");
        selectCell.className = "release-candidate-cell release-candidate-select-cell";
        const checkbox = document.createElement("input");
        checkbox.type = "checkbox";
        checkbox.setAttribute("data-action", "release-candidate-select");
        checkbox.setAttribute("data-object-ref", objectRef);
        checkbox.setAttribute("data-testid", "release-candidate-checkbox");
        checkbox.checked = eligible && state.releaseSelectedRefs.has(objectRef);
        checkbox.disabled = !eligible;
        selectCell.appendChild(checkbox);
        tr.appendChild(selectCell);
        const values = [
          row.objectType,
          row.schema,
          row.name,
          row.status,
          row.operation || row.planIntent || "review"
        ];
        const valueClasses = [
          "release-candidate-type-cell",
          "release-candidate-schema-cell",
          "release-candidate-name-cell",
          "release-candidate-status-cell",
          "release-candidate-operation-cell"
        ];
        values.forEach(function (value, valueIndex) {
          const td = document.createElement("td");
          td.className = "release-candidate-cell " + valueClasses[valueIndex];
          td.textContent = textOrEmpty(value);
          tr.appendChild(td);
        });
        const badgeCell = document.createElement("td");
        badgeCell.className = "release-candidate-cell release-candidate-badge-cell";
        const badge = document.createElement("span");
        badge.className = "release-operation-badge " + textOrEmpty(row.safetyLevel);
        badge.textContent = textOrEmpty(row.safetyBadge || row.operationLabel || "Manual Review");
        badgeCell.appendChild(badge);
        tr.appendChild(badgeCell);
        const explanation = document.createElement("td");
        explanation.className = "release-candidate-cell release-candidate-explanation-cell";
        explanation.textContent = textOrEmpty(row.operationExplanation || "Review selected object before artifact generation. DbState remains review-only.");
        tr.appendChild(explanation);
        const reasons = document.createElement("td");
        reasons.className = "release-candidate-cell release-candidate-reasons-cell";
        reasons.textContent = Array.isArray(row.operationReasons) && row.operationReasons.length
          ? row.operationReasons.join("; ")
          : "";
        tr.appendChild(reasons);
        const warningCell = document.createElement("td");
        warningCell.className = "release-candidate-cell release-candidate-warnings-cell";
        warningCell.textContent = Array.isArray(row.warnings) ? row.warnings.length : textOrEmpty(row.warnings);
        tr.appendChild(warningCell);
        body.appendChild(tr);
      });
      if (includedRows.length > 200) {
        const tr = document.createElement("tr");
        const td = document.createElement("td");
        td.colSpan = 10;
        td.textContent = "Additional in-sync rows summarized only: " + (includedRows.length - 200);
        tr.appendChild(td);
        body.appendChild(tr);
      }
    }
    updateReleaseWriteButton();
  }

  async function requestJson(endpoint, body, method) {
    const verb = method || (body == null ? "GET" : "POST");
    const options = { method: verb };
    if (body != null) {
      options.headers = { "Content-Type": "application/json" };
      options.body = JSON.stringify(body);
    }
    const response = await fetch(endpoint, options);
    const text = await response.text();
    let data;
    try {
      data = JSON.parse(text);
    } catch (error) {
      throw new Error("Service returned a non-JSON response with HTTP " + response.status + ".");
    }
    data.httpStatus = response.status;
    return data;
  }

  async function copyRedactedJson() {
    const text = jsonViewer.textContent || "{}";
    const status = byId("copy-json-status");
    try {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = text;
        textarea.setAttribute("readonly", "readonly");
        textarea.style.position = "fixed";
        textarea.style.left = "-9999px";
        document.body.appendChild(textarea);
        textarea.select();
        if (!document.execCommand("copy")) {
          throw new Error("Clipboard API unavailable.");
        }
        document.body.removeChild(textarea);
      }
      status.textContent = "Copied";
    } catch (error) {
      status.textContent = "Copy failed";
    }
  }

  async function run(label, endpoint, body, options) {
    const config = options || {};
    responseSummary.textContent = "Running " + label + "...";
    showStep(config.step || "reports");
    try {
      const data = await requestJson(endpoint, body, config.method);
      state.lastResponse = data;
      state.lastOperation = label;
      responseSummary.textContent = label + ": " + summarize(data);
      jsonViewer.textContent = redactedJson(data);
      updateStatus(label, data);
      updateWorkspaceContext(data);
      updateResultsContext(label, data);
      renderErrorSummary(data);
      renderWarnings(data);
      if (config.releaseResult) {
        state.releaseResponse = data;
        renderReleaseArtifactResult(data);
        renderReleasePlan();
      }
      if (config.profiles || Array.isArray(data.profiles)) {
        updateProfileList(data);
      }
      state.included = new Set();
      state.selectedIndex = -1;
      if (label === "Inspect") {
        updateCompareOptionLists(data);
      }
      if (label === "Reference-data compare" || label === "Reference-data status") {
        updateReferenceDataOptions(data);
      }
      if (label === "Reference-data database tables") {
        updateReferenceDataDatabaseTables(data);
      }
      if (label === "Reference-data YAML preview" || label === "Reference-data YAML write") {
        renderReferenceDataExportPreview(data);
      }
      if (label === "Reference-data YAML write" && data.success) {
        requestJson(approvedEndpoints.referenceDataStatus, attachWorkspacePath({})).then(updateReferenceDataOptions).catch(function () {});
      }
      const rows = rowsFromResponse(data, label);
      updateObjectTypeFilterOptions(rows, label);
      updateStatusFilterOptions(rows);
      renderResults(rows);
      if (config.summaryId) {
        updateSummary(config.summaryId, {
          success: data.success,
          status: data.httpStatus,
          repository: data.repositoryPath || "",
          gitRoot: data.gitRoot || "",
          branch: data.branch || "",
          tree: data.workingTreeStatus || "",
          project: data.dbstateProjectStatus || "",
          missingPaths: Array.isArray(data.missingPaths) ? data.missingPaths.length : 0,
          warnings: Array.isArray(data.warnings) ? data.warnings.length : 0,
          errors: Array.isArray(data.errors) ? data.errors.length : 0
        });
      }
      if (label === "Health") {
        servicePill.textContent = data.success ? "Service healthy" : "Service issue";
      }
    } catch (error) {
      const message = error && error.message ? error.message : "Unknown service error.";
      const data = { success: false, errors: [message] };
      state.lastResponse = data;
      state.lastOperation = label;
      responseSummary.textContent = label + ": " + message;
      jsonViewer.textContent = redactedJson(data);
      updateStatus(label, data);
      updateResultsContext(label, data);
      renderErrorSummary(data);
      renderWarnings(data);
      const rows = rowsFromResponse(data, label);
      updateObjectTypeFilterOptions(rows, label);
      updateStatusFilterOptions(rows);
      renderResults(rows);
      if (label === "Health") {
        servicePill.textContent = "Service not reachable";
      }
    }
  }

  document.querySelectorAll(".workflow-step").forEach(function (button) {
    button.addEventListener("click", function () {
      showStep(button.dataset.step);
    });
  });

  document.getElementById("object-type-filter").addEventListener("change", function () {
    state.selectedIndex = -1;
    renderResults(state.rows, true);
  });

  document.getElementById("status-filter").addEventListener("change", function () {
    state.selectedIndex = -1;
    renderResults(state.rows, true);
  });

  document.querySelectorAll("[data-diff-mode]").forEach(function (button) {
    button.addEventListener("click", function () {
      setObjectDiffMode(button.dataset.diffMode);
    });
  });

  document.getElementById("compare-schema").addEventListener("change", function () {
    updateTableOptions();
  });

  document.getElementById("data-table").addEventListener("change", function () {
    const table = value("data-table");
    state.referenceDataSelectedTables = new Set();
    if (table) {
      state.referenceDataSelectedTables.add(table);
      byId("data-scope").value = "selected";
    }
    renderReferenceDataConfiguredTables();
  });

  document.getElementById("workflow-mode").addEventListener("change", function () {
    updateWorkflowModePanels();
    clearOperationResults("Workflow mode changed. Run the selected operation again.");
  });

  document.getElementById("repository-write-confirmation").addEventListener("input", updateRepositoryWriteButton);

  document.getElementById("connection-mode").addEventListener("change", function () {
    updateConnectionModePanels();
    if (selectedConnectionMode() === "profile") {
      run("Connection profiles", approvedEndpoints.profiles, null, { step: "source-target", profiles: true });
    }
  });

  document.getElementById("profile-select").addEventListener("change", updateSelectedProfileDetails);

  document.querySelector("[data-action='profiles-refresh']").addEventListener("click", function () {
    run("Connection profiles", approvedEndpoints.profiles, null, { step: "source-target", profiles: true });
  });

  document.querySelector("[data-action='profile-save']").addEventListener("click", function () {
    try {
      const body = profileRequestBody();
      const existing = state.profiles.some(function (profile) {
        return profile.name === body.name;
      });
      const endpoint = existing ? profilePath(body.name) : approvedEndpoints.profiles;
      const method = existing ? "PUT" : "POST";
      run("Save connection profile", endpoint, body, { step: "source-target", profiles: true, method: method });
    } catch (error) {
      responseSummary.textContent = "Save connection profile: " + error.message;
    }
  });

  document.querySelector("[data-action='profile-delete']").addEventListener("click", function () {
    const profileName = value("profile-select") || value("profile-name");
    if (!profileName) {
      responseSummary.textContent = "Delete connection profile: select a profile first.";
      return;
    }
    run("Delete connection profile", profilePath(profileName), null, { step: "source-target", profiles: true, method: "DELETE" });
  });

  document.querySelector("[data-action='connection-test']").addEventListener("click", function () {
    run("Connection test", approvedEndpoints.connectionTest, attachConnection({}), { step: "source-target" });
  });

  document.querySelector("[data-action='workspace-browse']").addEventListener("click", openDirectoryPicker);

  document.querySelector("[data-action='directory-close']").addEventListener("click", function () {
    byId("directory-picker").hidden = true;
  });

  document.querySelector("[data-action='directory-roots']").addEventListener("click", loadDirectoryRoots);

  document.querySelector("[data-action='directory-refresh']").addEventListener("click", function () {
    listDirectories();
  });

  document.querySelector("[data-action='directory-up']").addEventListener("click", function () {
    if (state.directoryParentPath) {
      listDirectories(state.directoryParentPath);
    } else {
      setDirectoryPickerError("No parent folder is available.");
    }
  });

  document.querySelector("[data-action='directory-select']").addEventListener("click", function () {
    selectDirectoryAsWorkspace();
  });

  document.querySelector("[data-action='health']").addEventListener("click", function () {
    run("Health", approvedEndpoints.health, null, { summaryId: "workspace-summary", step: "workspace" });
  });

  document.querySelector("[data-action='workspace-status']").addEventListener("click", async function () {
    const data = await requestJson(approvedEndpoints.workspaceValidate, attachWorkspacePath({}));
    state.lastResponse = data;
    jsonViewer.textContent = redactedJson(data);
    updateStatus("Workspace validate", data);
    updateWorkspaceContext(data);
    updateSummary("workspace-summary", data);
    if (data && data.dbstateProjectStatus === "notGitRepository") {
      showNotGitRepositoryModal();
    }
  });

  document.querySelector("[data-action='repo-status']").addEventListener("click", async function () {
    if (!(await guardGitWorkspaceBefore("repo-status"))) {
      return;
    }
    run("Repository status", approvedEndpoints.repoStatus, attachWorkspacePath({}), { summaryId: "workspace-summary", step: "workspace" });
  });

  document.querySelector("[data-action='init-plan']").addEventListener("click", async function () {
    if (!(await guardGitWorkspaceBefore("init-plan"))) {
      return;
    }
    run("Init plan", approvedEndpoints.initPlan, attachWorkspacePath({ dryRun: true }), { summaryId: "workspace-summary", step: "workspace" });
  });

  byId("init-confirmation").addEventListener("input", updateInitWriteButton);

  document.querySelector("[data-action='init-write']").addEventListener("click", async function () {
    if (!(await guardGitWorkspaceBefore("init-write"))) {
      return;
    }
    const body = attachWorkspacePath({
      confirmInitializeProject: true,
      confirmationText: value("init-confirmation")
    });
    run("Initialize DbState Project", approvedEndpoints.initWrite, body, { summaryId: "workspace-summary", step: "workspace" });
  });

  document.querySelector("[data-action='inspect']").addEventListener("click", function () {
    try {
      if (!standardOperationAllowed("Inspect")) {
        return;
      }
      const body = attachWorkspacePath(attachConnection(buildScope()));
      run("Inspect", approvedEndpoints.inspect, body, { step: "results" });
    } catch (error) {
      responseSummary.textContent = "Inspect: " + error.message;
    }
  });

  document.querySelector("[data-action='compare']").addEventListener("click", function () {
    try {
      if (!standardOperationAllowed("Compare")) {
        return;
      }
      run("Compare", approvedEndpoints.compare, attachWorkspacePath(attachConnection(buildScope())), { step: "results" });
    } catch (error) {
      responseSummary.textContent = "Compare: " + error.message;
    }
  });

  document.querySelector("[data-action='plan']").addEventListener("click", function () {
    try {
      if (!standardOperationAllowed("Plan")) {
        return;
      }
      const body = attachWorkspacePath(attachConnection(buildScope()));
      body.include = [];
      body.exclude = [];
      run("Plan", approvedEndpoints.plan, body, { step: "results" });
    } catch (error) {
      responseSummary.textContent = "Plan: " + error.message;
    }
  });

  document.querySelector("[data-action='data-compare']").addEventListener("click", function () {
    try {
      if (!standardOperationAllowed("Reference-data compare")) {
        return;
      }
      run("Reference-data compare", approvedEndpoints.dataCompare, attachWorkspacePath(attachConnection(dataScope())), { step: "results" });
    } catch (error) {
      responseSummary.textContent = "Reference-data compare: " + error.message;
    }
  });

  document.querySelector("[data-action='reference-data-status']").addEventListener("click", function () {
    run("Reference-data status", approvedEndpoints.referenceDataStatus, attachWorkspacePath({}), { step: "compare-options" });
  });

  document.querySelector("[data-action='reference-data-select-all']").addEventListener("click", function () {
    state.referenceDataSelectedTables = new Set(state.referenceDataConfiguredTables.map(function (table) {
      return table.tableName;
    }));
    byId("data-scope").value = "selected";
    renderReferenceDataConfiguredTables();
  });

  document.querySelector("[data-action='reference-data-clear-selection']").addEventListener("click", function () {
    state.referenceDataSelectedTables = new Set();
    renderReferenceDataConfiguredTables();
  });

  byId("reference-data-export-search").addEventListener("input", renderReferenceDataDatabaseTables);
  byId("reference-data-write-confirmation").addEventListener("input", updateReferenceDataWriteButton);

  document.querySelector("[data-action='reference-data-load-database-tables']").addEventListener("click", function () {
    try {
      run("Reference-data database tables", approvedEndpoints.referenceDataDatabaseTables, attachWorkspacePath(attachConnection({})), { step: "compare-options" });
    } catch (error) {
      responseSummary.textContent = "Reference-data database tables: " + error.message;
    }
  });

  document.querySelector("[data-action='reference-data-preview-yaml']").addEventListener("click", function () {
    try {
      run("Reference-data YAML preview", approvedEndpoints.referenceDataExportPreview, referenceDataExportBody(false), { step: "compare-options" });
    } catch (error) {
      responseSummary.textContent = "Reference-data YAML preview: " + error.message;
    }
  });

  document.querySelector("[data-action='reference-data-write-yaml']").addEventListener("click", function () {
    try {
      if (value("reference-data-write-confirmation") !== "WRITE REFERENCE DATA FILES") {
        throw new Error("Type WRITE REFERENCE DATA FILES before writing reference-data YAML files.");
      }
      run("Reference-data YAML write", approvedEndpoints.referenceDataExportWrite, referenceDataExportBody(true), { step: "compare-options" });
    } catch (error) {
      responseSummary.textContent = "Reference-data YAML write: " + error.message;
    }
  });

  document.querySelector("[data-action='repository-sync-preview']").addEventListener("click", function () {
    try {
      run("Database to Repository Preview", approvedEndpoints.repositorySyncPreview, repositorySyncBody(false), { step: "results" });
    } catch (error) {
      responseSummary.textContent = "Database to Repository Preview: " + error.message;
    }
  });

  document.querySelector("[data-action='repository-sync-write']").addEventListener("click", function () {
    try {
      if (!repositorySyncWriteConfirmed()) {
        throw new Error("Type WRITE REPOSITORY FILES before writing repository files.");
      }
      run("Database to Repository Write", approvedEndpoints.repositorySyncWrite, repositorySyncBody(true), { step: "results" });
    } catch (error) {
      responseSummary.textContent = "Database to Repository Write: " + error.message;
    }
  });



  byId("release-name").addEventListener("input", function () {
    renderReleasePlan();
    updateReleaseWriteButton();
  });

  byId("release-confirmation").addEventListener("input", updateReleaseWriteButton);

  document.querySelector("[data-action='release-preview']").addEventListener("click", function () {
    try {
      if (currentWorkflowMode() !== "compare") {
        renderReleasePlan();
        return;
      }
      if (!releaseName()) {
        throw new Error("Release name is required before dry-run.");
      }
      run("Release artifact dry-run", approvedEndpoints.releasePreview, releaseBody(false), { step: "release-plan", releaseResult: true });
    } catch (error) {
      responseSummary.textContent = "Release artifact dry-run: " + error.message;
    }
  });

  document.querySelector("[data-action='release-write']").addEventListener("click", function () {
    try {
      if (currentWorkflowMode() !== "compare") {
        renderReleasePlan();
        return;
      }
      if (!releaseName()) {
        throw new Error("Release name is required before generating release artifacts.");
      }
      if (!releaseConfirmed()) {
        throw new Error("Type GENERATE RELEASE ARTIFACTS before generating release artifacts.");
      }
      run("Generate Release Artifact", approvedEndpoints.releaseWrite, releaseBody(true), { step: "release-plan", releaseResult: true });
    } catch (error) {
      responseSummary.textContent = "Generate Release Artifact: " + error.message;
    }
  });

  byId("release-candidates-body").addEventListener("change", function (event) {
    const checkbox = event.target.closest("[data-action='release-candidate-select']");
    if (!checkbox) {
      return;
    }
    const objectRef = checkbox.getAttribute("data-object-ref") || "";
    if (!objectRef) {
      return;
    }
    if (checkbox.checked) {
      state.releaseSelectedRefs.add(objectRef);
    } else {
      state.releaseSelectedRefs.delete(objectRef);
    }
    updateReleaseSelectionSummary(releaseCandidateRows());
    updateReleaseWriteButton();
  });

  document.querySelector("[data-action='release-select-all-eligible']").addEventListener("click", function () {
    const rows = releaseCandidateRows();
    rows.forEach(function (row, index) {
      const objectRef = releaseCandidateObjectRef(row, index);
      if (releaseCandidateEligible(row)) {
        state.releaseSelectedRefs.add(objectRef);
      }
    });
    renderReleasePlan();
  });

  document.querySelector("[data-action='release-clear-selection']").addEventListener("click", function () {
    state.releaseSelectedRefs = new Set();
    renderReleasePlan();
    responseSummary.textContent = "Select at least one release candidate.";
  });

  byId("release-artifact-result").addEventListener("click", function (event) {
    const button = event.target.closest("[data-action='release-artifact-preview']");
    if (!button) {
      return;
    }
    try {
      const artifactPath = button.getAttribute("data-artifact-path") || "";
      previewReleaseArtifact(artifactPath).catch(function (error) {
        responseSummary.textContent = "Release artifact preview: " + error.message;
      });
    } catch (error) {
      responseSummary.textContent = "Release artifact preview: " + error.message;
    }
  });

  document.querySelector("[data-action='copy-json']").addEventListener("click", copyRedactedJson);

  document.querySelector("[data-action='modal-close']").addEventListener("click", closeModal);
  document.addEventListener("keydown", function (event) {
    if (event.key === "Escape") {
      closeModal();
    }
  });

  updateConnectionModePanels();
  updateWorkflowModePanels();
  run("Health", approvedEndpoints.health, null, { summaryId: "workspace-summary", step: "workspace" });
}());
"#;

pub fn index_html() -> &'static str {
    UI_HTML
}

pub fn app_css() -> &'static str {
    UI_CSS
}

pub fn app_js() -> &'static str {
    UI_JS
}

pub fn ui_html() -> &'static str {
    index_html()
}

pub fn ui_css() -> &'static str {
    app_css()
}

pub fn ui_js() -> &'static str {
    app_js()
}
