# Slice 19 Windows Installer Packaging

Slice 19 adds Windows private beta installer packaging for DbState PostgreSQL v0.1.

The installer packages only the compiled DbState binary and beta-facing installer assets. It does not distribute source code, Git history, Cargo project files, tests, internal development docs, local configuration, connection profiles, database workspaces, generated release artifacts, or secrets.

DbState remains local-first and safety-first:

- DbState does not execute SQL.
- DbState does not execute generated SQL.
- DbState does not apply changes to a database.
- DbState does not mutate PostgreSQL.
- DbState does not stage, commit, push, pull, fetch, or tag Git changes.
- Release artifacts remain review-only.

## Packaging Files

```text
packaging/windows/Build-WindowsInstaller.ps1
packaging/windows/DbState.iss
packaging/windows/README-installer.md
```

Generated outputs are ignored by Git:

```text
packaging/windows/staging/
packaging/windows/out/
dist/
*.exe
*.msi
```

## Prerequisites

Required:

- Windows PowerShell.
- Rust toolchain for building the release binary.
- Inno Setup 6 for compiling the installer.

The packaging script can stage and validate files without Inno Setup, then fails cleanly with install instructions if `ISCC.exe` is missing.

## Build Command

From the repository root:

```powershell
cd C:\SourceCodes\DbState
.\packaging\windows\Build-WindowsInstaller.ps1
```

Optional explicit Inno Setup path:

```powershell
.\packaging\windows\Build-WindowsInstaller.ps1 `
  -InnoSetupCompiler "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

The script:

1. Runs `cargo build --release`.
2. Creates a clean staging folder.
3. Copies only allowed installer payload files.
4. Generates `VERSION.txt`.
5. Validates that source files, secrets, and forbidden project files were not staged.
6. Invokes Inno Setup if available.

## Installer Output

Expected output:

```text
packaging/windows/out/DbState-PostgreSQL-v0.1.0-alpha.3-setup.exe
```

Do not commit generated installer binaries.

Darwin should create the `v0.1.0-alpha.3` tag manually only after Slice 19 is merged and post-merge installer smoke passes.

## Installed Files

Default install directory:

```text
C:\Program Files\DbState
```

Installed payload:

- `dbstate.exe`
- `README.txt`
- `VERSION.txt`
- `LICENSE.txt`, only if a license file exists at packaging time

Intentionally excluded:

- `src\`
- `.git\`
- `Cargo.toml`
- `Cargo.lock`
- `tests\`
- internal PostgreSQL planning docs
- Rust source files
- PowerShell packaging scripts
- `target\debug\`
- local config
- connection profiles
- `.env`
- database workspace files
- generated local artifacts
- personal local paths

## Start Menu Shortcuts

The installer creates a Start Menu folder:

```text
DbState
```

Shortcuts:

- Start DbState Local Service
- Open DbState UI
- DbState Help

Start DbState Local Service runs:

```powershell
dbstate.exe serve --host 127.0.0.1 --port 4587
```

The shortcut may open a console window. This is acceptable for private beta.

Open DbState UI opens:

```text
http://127.0.0.1:4587/
```

The local service must be running before the UI opens.

## PATH Behavior

Slice 19 does not modify the system PATH. Users can run the installed binary by using:

```powershell
C:\Program Files\DbState\dbstate.exe --help
```

PATH modification can be revisited after private beta feedback.

## Uninstall Behavior

Use Windows Apps or Control Panel to uninstall.

The installer removes installed application files. `%APPDATA%\DbState` may remain if the tester created non-secret connection profiles. Connection profiles must not contain passwords or full PostgreSQL URLs.

## Manual Smoke Checklist

1. Build the installer.
2. Install the generated setup `.exe`.
3. Confirm `C:\Program Files\DbState\dbstate.exe` exists.
4. Open Start Menu shortcut `Start DbState Local Service`.
5. Confirm service prints:
   - local URL
   - local-only binding
   - no generated SQL execution or database apply
6. Open `http://127.0.0.1:4587/`.
7. Confirm UI loads.
8. Confirm health endpoint works:

```powershell
Invoke-RestMethod http://127.0.0.1:4587/health
```

9. Create a disposable ParkingDemo database using the Slice 18 walkthrough.
10. Select workspace `C:\DbState\ParkingDemo`.
11. Run Inspect.
12. Confirm no source files were installed.
13. Uninstall from Windows Apps or Control Panel.
14. Confirm the install folder is removed or contains only expected remnants.
15. Confirm `%APPDATA%\DbState` may remain only for non-secret profiles, if created.

## Private Beta Limitations

- No Windows Service registration.
- No auto-update.
- No code signing in Slice 19.
- No MSI or WiX packaging.
- No telemetry.
- No hosted service mode.
- No authentication or user accounts.
- No source code distribution through the installer.
- No SQL execution.
- No direct database apply.
- No PostgreSQL mutation.
- No Git automation.

## Validation

Run normal validation:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
cargo build --release
docker build -t dbstate-postgres:dev .
```

Run packaging validation:

```powershell
.\packaging\windows\Build-WindowsInstaller.ps1
```

If Inno Setup is missing, the script should fail after staging validation with actionable install instructions.
