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
            <option value="inspect">PostgreSQL Inspect Only</option>
            <option value="compare">Repository to Database Compare</option>
            <option value="databaseToRepository">Database to Repository Compare</option>
            <option value="data">Reference-Data Compare</option>
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
            <select id="data-scope" disabled>
              <option value="all">All configured tables</option>
              <option value="table">Table</option>
            </select>
          </label>
          <label for="data-table">Reference-data table
            <select id="data-table" disabled>
              <option value="">All</option>
            </select>
          </label>
        </div>
        <p class="note">Run Inspect first to populate schema and table lists.</p>
        <p class="note beta-disabled-note">Disabled in current beta. Include/exclude filters and reference-data compare are out-of-scope for this beta version.</p>
        <div class="object-filter-row" aria-label="Object type filters">
          <label><input type="checkbox" checked disabled> schemas</label>
          <label><input type="checkbox" checked disabled> tables</label>
          <label><input type="checkbox" disabled> indexes future</label>
          <label><input type="checkbox" disabled> views future</label>
          <label><input type="checkbox" disabled> functions future</label>
          <label><input type="checkbox" disabled> triggers future</label>
          <label><input type="checkbox" disabled> grants future</label>
        </div>
        <div class="button-row">
          <button type="button" data-action="inspect" data-standard-operation-action>Inspect</button>
          <button type="button" data-action="compare" data-standard-operation-action>Run Compare</button>
          <button type="button" data-action="plan" data-standard-operation-action>Run Plan</button>
          <button type="button" data-action="data-compare" data-standard-operation-action disabled>Run Reference Data Compare</button>
        </div>
        <div id="repository-sync-controls" class="subsection" hidden>
          <h3>Database to Repository Compare</h3>
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
        <div id="related-objects-view" class="related-objects-grid" hidden>
          <section>
            <h3>Source Related Objects</h3>
            <div id="source-related-objects" class="related-object-list">No related object details loaded.</div>
          </section>
          <section>
            <h3>Target Related Objects</h3>
            <div id="target-related-objects" class="related-object-list">No related object details loaded.</div>
          </section>
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
          <p>Generate reviewable release artifacts for Repository to Database Compare only.</p>
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
            <div class="table-wrap">
              <table class="results-grid" aria-label="Release candidates" data-testid="release-candidates">
                <thead>
                  <tr><th>Object type</th><th>Schema</th><th>Object name</th><th>Status</th><th>Planned operation</th><th>Warnings</th></tr>
                </thead>
                <tbody id="release-candidates-body">
                  <tr><td colspan="6">No selected result rows yet.</td></tr>
                </tbody>
              </table>
            </div>
          </div>
          <div class="release-card">
            <h3>Dry-run / Generated Artifacts</h3>
            <div id="release-artifact-result" data-testid="generated-artifacts">No release artifact dry-run has been run yet.</div>
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

.related-objects-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.related-object-list {
  display: grid;
  gap: 8px;
}

.related-object-group {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #ffffff;
  padding: 10px;
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
    dataCompare: "/api/v1/postgres/data-compare",
    objectDdl: "/api/v1/postgres/object-ddl",
    repositorySyncPreview: "/api/v1/postgres/repository-sync/preview",
    repositorySyncWrite: "/api/v1/postgres/repository-sync/write",
    releasePreview: "/api/v1/postgres/release/preview",
    releaseWrite: "/api/v1/postgres/release/write",
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
    const body = { scope: scope };
    if (scope === "table") {
      const table = value("data-table");
      if (!table) {
        throw new Error("Reference-data table scope requires a configured table selection.");
      }
      body.table = table;
    }
    return body;
  }

  function repositorySyncWriteConfirmed() {
    return value("repository-write-confirmation") === "WRITE REPOSITORY FILES";
  }

  function standardOperationAllowed(actionLabel) {
    if (!isDatabaseToRepositoryMode(currentWorkflowMode())) {
      return true;
    }
    clearOperationResults("Workflow mode changed. Run an operation to load results.");
    const message = actionLabel === "Compare"
      ? "Compare is not available in Database to Repository mode. Use Preview Repository Sync."
      : actionLabel + " is not available in Database to Repository mode. Use Preview Repository Sync.";
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
    const select = byId("workflow-mode");
    const selectedMode = value("workflow-mode") || "inspect";
    if (selectedMode === "data") {
      showModal(
        "Reference-Data Compare Is Out of Scope",
        "Reference-data compare is out-of-scope of the current beta version.\n\nThis beta focuses on PostgreSQL schema/object workflows: inspect, database-to-repository capture, repository-to-database compare, Object Diff, and release artifact review."
      );
      select.value = state.previousWorkflowMode || "inspect";
    }
    const mode = value("workflow-mode") || "inspect";
    state.previousWorkflowMode = mode;
    const layout = workflowLayout(mode);
    byId("repository-sync-controls").hidden = !isDatabaseToRepositoryMode(mode);
    document.querySelectorAll("[data-standard-operation-action]").forEach(function (button) {
      const disabledForDbToRepo = isDatabaseToRepositoryMode(mode);
      const isDataCompare = button.dataset.action === "data-compare";
      button.hidden = disabledForDbToRepo;
      button.disabled = disabledForDbToRepo || isDataCompare;
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
    if (isDatabaseToRepositoryMode(mode)) {
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
    if (mode === "data") {
      return {
        description: "DbState compares configured repository reference data to PostgreSQL through read-only service operations.",
        sourceKind: "Repository configured reference data",
        sourceType: "Repository",
        sourceContext: "repository",
        targetKind: "PostgreSQL database",
        targetType: "Database",
        targetContext: "connection"
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

  function updateObjectTypeFilterOptions(rows, label) {
    const select = byId("object-type-filter");
    const selected = select.value || "all";
    resetSelect(select, "All");
    select.options[0].value = "all";
    appendOption(select, "schema", "Schema");
    appendOption(select, "table", "Table");
    ["extension", "enum", "sequence", "index", "view"].forEach(function (type) {
      if (rows.some(function (row) { return row.objectType === type; })) {
        appendOption(select, type, type.charAt(0).toUpperCase() + type.slice(1));
      }
    });
    const hasReferenceData = value("workflow-mode") === "data" || label === "Reference-data compare" || rows.some(function (row) {
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
    if (["schema", "table", "column", "extension", "enum", "sequence", "index", "view", "referenceDataTable", "referenceDataRow"].indexOf(text) >= 0) {
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
    }

    if (Array.isArray(data.tableResults)) {
      data.tableResults.forEach(function (table) {
        const tableIdentity = identityFromTableName(table.tableName);
        rows.push({
          objectRef: "referenceDataTable:" + table.tableName,
          objectType: "referenceDataTable",
          schema: tableIdentity.schema,
          name: tableIdentity.name,
          status: "inspected",
          operation: "compare only",
          warnings: table.warnings || [],
          source: "repository reference-data",
          target: "postgresql",
          raw: table
        });
        if (Array.isArray(table.rowResults)) {
          table.rowResults.forEach(function (row) {
            rows.push({
              objectRef: "referenceDataRow:" + table.tableName + ":" + textOrEmpty(row.rowKey),
              objectType: "referenceDataRow",
              schema: tableIdentity.schema,
              name: tableIdentity.name + " " + textOrEmpty(row.rowKey),
              status: row.classification || "row",
              operation: "compare only",
              warnings: row.warnings || [],
              source: "repository reference-data",
              target: "postgresql",
              raw: row
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
      return "databaseToRepository";
    }
    if (label === "Reference-data compare") {
      return "data";
    }
    return "compare";
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
        showStep("object-diff");
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

  function groupRelatedObjects(items) {
    const groups = {};
    if (!items.length) {
      groups["Details"] = ["Not available in Private Beta"];
      return groups;
    }
    items.forEach(function (item) {
      const group = item.group || "Details";
      const value = item.name === "Not available in Private Beta" ? item.name : ([item.name, item.detail].filter(Boolean).join(" - ") || "Not available in Private Beta");
      if (!groups[group]) {
        groups[group] = [];
      }
      groups[group].push(value);
    });
    return groups;
  }

  function renderRelatedObjectList(targetId, items) {
    const target = byId(targetId);
    target.innerHTML = "";
    const groups = groupRelatedObjects(items);
    Object.keys(groups).sort().forEach(function (group) {
      const wrapper = document.createElement("div");
      wrapper.className = "related-object-group";
      const heading = document.createElement("h4");
      heading.textContent = group;
      wrapper.appendChild(heading);
      const list = document.createElement("ul");
      groups[group].forEach(function (value) {
        const item = document.createElement("li");
        item.textContent = value;
        list.appendChild(item);
      });
      wrapper.appendChild(list);
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
      renderRelatedObjectList("source-related-objects", relatedObjectsForSide(state.selectedObjectDdl, direction.sourceDdlSide));
      renderRelatedObjectList("target-related-objects", relatedObjectsForSide(state.selectedObjectDdl, direction.targetDdlSide));
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
    if (mode === "compare" && row && isDatabaseToRepositoryRow(row)) {
      mode = "databaseToRepository";
    }
    return directionForWorkflowMode(mode);
  }

  function isDatabaseToRepositoryMode(mode) {
    return mode === "databaseToRepository" || mode === "dbToRepo";
  }

  function currentWorkflowMode() {
    return value("workflow-mode") || "inspect";
  }

  function workflowModesMatch(rowMode, selectedMode) {
    if (isDatabaseToRepositoryMode(rowMode) && isDatabaseToRepositoryMode(selectedMode)) {
      return true;
    }
    return textOrEmpty(rowMode || "inspect") === textOrEmpty(selectedMode || "inspect");
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
    if (isDatabaseToRepositoryMode(mode)) {
      return {
        sourceLabel: "PostgreSQL database",
        sourceType: "Database",
        targetLabel: "Repository desired state",
        targetType: "Repository",
        sourceDdlSide: "database",
        targetDdlSide: "repository"
      };
    }
    if (mode === "data") {
      return {
        sourceLabel: "Repository configured reference data",
        sourceType: "Repository",
        targetLabel: "PostgreSQL database",
        targetType: "Database",
        sourceDdlSide: "repository",
        targetDdlSide: "database"
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
    return attachWorkspacePath(attachConnection({
      objectType: row.objectType,
      schema: row.schema || "",
      objectName: objectName,
      relativePath: row.relativePath || ""
    }));
  }

  async function loadSelectedObjectDdl(row, direction) {
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
    if (button) {
      button.disabled = currentWorkflowMode() !== "compare" || !releaseName() || !releaseConfirmed();
    }
  }

  function releaseBody(write) {
    const body = attachWorkspacePath(attachConnection(buildScope()));
    body.releaseName = releaseName();
    body.include = [];
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
    const artifacts = Array.isArray(data.createdArtifacts) && data.createdArtifacts.length
      ? data.createdArtifacts
      : Array.isArray(data.plannedArtifacts) ? data.plannedArtifacts : [];
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
        item.textContent = textOrEmpty(artifact);
        list.appendChild(item);
      });
      target.appendChild(list);
    }
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

  function renderReleasePlan() {
    const mode = currentWorkflowMode();
    const notApplicable = byId("release-plan-not-applicable");
    const content = byId("release-plan-content");
    if (mode !== "compare") {
      content.hidden = true;
      notApplicable.hidden = false;
      notApplicable.textContent = isDatabaseToRepositoryMode(mode)
        ? "Release Plan is not used for Database to Repository Compare. This workflow writes selected PostgreSQL object definitions into repository files under database/objects/. Use Results → Write Selected Repository Changes."
        : "Release artifacts are generated from Repository to Database Compare. Run Repository to Database Compare first.";
      return;
    }
    content.hidden = false;
    notApplicable.hidden = true;
    const includedRows = state.rows.filter(function (row, index) {
      return state.included.has(rowRef(row, index));
    });
    updateSummary("release-context", {
      workflowMode: "Repository to Database Compare",
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
      td.colSpan = 6;
      td.textContent = "No selected result rows yet. Run Repository to Database Compare or Plan, then check rows in Results.";
      tr.appendChild(td);
      body.appendChild(tr);
    } else {
      includedRows.slice(0, 200).forEach(function (row) {
        const tr = document.createElement("tr");
        [
          row.objectType,
          row.schema,
          row.name,
          row.status,
          row.operation || row.planIntent || "review",
          Array.isArray(row.warnings) ? row.warnings.length : textOrEmpty(row.warnings)
        ].forEach(function (value) {
          const td = document.createElement("td");
          td.textContent = textOrEmpty(value);
          tr.appendChild(td);
        });
        body.appendChild(tr);
      });
      if (includedRows.length > 200) {
        const tr = document.createElement("tr");
        const td = document.createElement("td");
        td.colSpan = 6;
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
      }
      if (config.profiles || Array.isArray(data.profiles)) {
        updateProfileList(data);
      }
      state.included = new Set();
      state.selectedIndex = -1;
      if (label === "Inspect") {
        updateCompareOptionLists(data);
      }
      if (label === "Reference-data compare") {
        updateReferenceDataOptions(data);
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
    showModal(
      "Reference-Data Compare Is Out of Scope",
      "Reference-data compare is out-of-scope of the current beta version.\n\nThis beta focuses on PostgreSQL schema/object workflows: inspect, database-to-repository capture, repository-to-database compare, Object Diff, and release artifact review."
    );
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
