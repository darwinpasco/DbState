# DbState PostgreSQL Private Beta Installer

This installer packages the DbState PostgreSQL private beta CLI, local Service API, and static browser UI into a Windows install folder.

DbState runs locally. The browser UI is served by:

```powershell
dbstate.exe serve --host 127.0.0.1 --port 4587
```

Open the UI at:

```text
http://127.0.0.1:4587/
```

## Safety

DbState does not:

- Execute SQL.
- Execute generated SQL.
- Apply changes to a database.
- Mutate PostgreSQL.
- Auto-stage, commit, push, pull, or fetch Git changes.
- Store passwords.
- Store full PostgreSQL URLs.

Connection profiles store only non-secret metadata. Use disposable or approved read-only PostgreSQL targets for private beta testing.

## Installed Files

The installer places files under:

```text
C:\Program Files\DbState
```

Expected files:

- `dbstate.exe`
- `README.txt`
- `VERSION.txt`
- `LICENSE.txt`, only if a repository license file is available at packaging time

The installer does not include source code, Git history, Cargo files, internal development docs, tests, local workspaces, connection profiles, or secrets.

## Start Menu

The installer creates a `DbState` Start Menu folder with:

- Start DbState Local Service
- Open DbState UI
- DbState Help

Start the local service before opening the UI.

## Private Beta Walkthrough

Use the ParkingDemo walkthrough for testing:

```text
docs/postgresql-v0.1/48-slice-18-golden-path-private-beta-walkthrough.md
```

Recommended tester workspace:

```text
C:\DbState\ParkingDemo
```

## Uninstall

Uninstall DbState from Windows Apps or Control Panel.

If you created non-secret connection profiles, `%APPDATA%\DbState` may remain. That folder should not contain passwords or full PostgreSQL URLs.
