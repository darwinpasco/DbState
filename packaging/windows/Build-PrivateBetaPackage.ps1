[CmdletBinding()]
param(
    [string]$Version = "v0.1.0-private-beta.1",
    [Parameter(Mandatory = $true)][string]$InstallerPath,
    [string]$OutputRoot = "dist/private-beta"
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
    $scriptDirectory = Split-Path -Parent $PSCommandPath
    return (Resolve-Path (Join-Path $scriptDirectory "..\..")).Path
}

function Resolve-RepoRelativePath {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$Path
    )

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return (Join-Path $RepoRoot $Path)
}

function New-CleanDirectory {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }

    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Assert-PrivateBetaPackageIsSafe {
    param([Parameter(Mandatory = $true)][string]$PackageDirectory)

    $forbiddenNames = @(
        ".git",
        "src",
        "Cargo.toml",
        "Cargo.lock",
        "tests",
        ".env",
        "connection-profiles.json",
        "target",
        "staging"
    )

    $forbiddenExtensions = @(".rs", ".ps1")
    $items = Get-ChildItem -LiteralPath $PackageDirectory -Recurse -Force

    foreach ($item in $items) {
        if ($forbiddenNames -contains $item.Name) {
            throw "Forbidden item found in private beta package: $($item.FullName)"
        }

        if (-not $item.PSIsContainer -and ($forbiddenExtensions -contains $item.Extension)) {
            throw "Forbidden source/script file found in private beta package: $($item.FullName)"
        }

        $normalized = $item.FullName.Replace("/", "\")
        if ($normalized -like "*\packaging\windows\staging\*") {
            throw "Installer staging content was copied into the private beta package: $($item.FullName)"
        }
    }

    $testerFacingFiles = Get-ChildItem -LiteralPath $PackageDirectory -Recurse -File -Force |
        Where-Object { $_.Extension -in @(".md", ".txt", ".json", ".yml", ".yaml") }

    foreach ($file in $testerFacingFiles) {
        $content = Get-Content -LiteralPath $file.FullName -Raw -ErrorAction Stop
        if ($content -match "D:\\") {
            throw "Tester-facing file contains a Drive D path: $($file.FullName)"
        }
    }
}

function Write-PrivateBetaReadme {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Version,
        [Parameter(Mandatory = $true)][string]$InstallerName
    )

    @"
# DbState PostgreSQL Private Beta Package

Package version:

```text
$Version
```

Installer:

```text
$InstallerName
```

## Install

1. Run the installer.
2. Use the default install folder:

```text
C:\Program Files\DbState
```

3. Start `Start DbState Local Service` from the Start Menu.
4. Open:

```text
http://127.0.0.1:4587/
```

5. Follow the ParkingDemo walkthrough using:

```text
C:\DbState\ParkingDemo
```

Do not use production, UAT, staging, shared databases, or customer data for private beta testing.

## Included Docs

- `docs/48-slice-18-golden-path-private-beta-walkthrough.md`
- `docs/49-private-beta-feedback-template.md`
- `docs/52-private-beta-known-limitations.md`
- `docs/53-private-beta-installer-distribution.md`
- `docs/54-private-beta-smoke-test-matrix.md`

## Safety Reminders

DbState does not:

- execute SQL
- execute generated SQL
- apply changes to a database
- mutate PostgreSQL
- auto-stage, commit, push, pull, fetch, or tag Git changes
- store passwords
- store full PostgreSQL URLs

Release artifacts are review-only.

## Feedback

Use `docs/49-private-beta-feedback-template.md`.

Feedback channel:

```text
<feedback-channel-to-be-filled-by-Darwin>
```

Redact passwords, full PostgreSQL URLs, production data, customer data, tokens, certificates, and secrets before sending feedback.
"@ | Set-Content -LiteralPath $Path -Encoding UTF8
}

$repoRoot = Resolve-RepoRoot
$resolvedInstallerPath = Resolve-RepoRelativePath -RepoRoot $repoRoot -Path $InstallerPath
$resolvedOutputRoot = Resolve-RepoRelativePath -RepoRoot $repoRoot -Path $OutputRoot

if (-not (Test-Path -LiteralPath $resolvedInstallerPath -PathType Leaf)) {
    throw @"
Installer was not found:
  $resolvedInstallerPath

Build the installer first, or pass -InstallerPath to the generated setup executable.
Example:
  .\packaging\windows\Build-PrivateBetaPackage.ps1 -Version v0.1.0-private-beta.1 -InstallerPath .\packaging\windows\out\DbState-PostgreSQL-v0.1.0-alpha.3-setup.exe
"@
}

$packageDirectory = Join-Path $resolvedOutputRoot $Version
$docsDirectory = Join-Path $packageDirectory "docs"
$installerFileName = "DbState-PostgreSQL-$Version-setup.exe"
$installerDestination = Join-Path $packageDirectory $installerFileName
$hashDestination = Join-Path $packageDirectory "$installerFileName.sha256.txt"
$readmeDestination = Join-Path $packageDirectory "README-private-beta.md"

Write-Host "DbState private beta distribution package"
Write-Host "Repository: $repoRoot"
Write-Host "Version: $Version"
Write-Host "Installer: $resolvedInstallerPath"

New-CleanDirectory -Path $packageDirectory
New-Item -ItemType Directory -Force -Path $docsDirectory | Out-Null

Copy-Item -LiteralPath $resolvedInstallerPath -Destination $installerDestination

$hash = Get-FileHash -LiteralPath $installerDestination -Algorithm SHA256
"$($hash.Hash)  $installerFileName" | Set-Content -LiteralPath $hashDestination -Encoding ASCII

$docNames = @(
    "48-slice-18-golden-path-private-beta-walkthrough.md",
    "49-private-beta-feedback-template.md",
    "52-private-beta-known-limitations.md",
    "53-private-beta-installer-distribution.md",
    "54-private-beta-smoke-test-matrix.md"
)

foreach ($docName in $docNames) {
    $source = Join-Path (Join-Path $repoRoot "docs\postgresql-v0.1") $docName
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required tester-facing doc was not found: $source"
    }

    Copy-Item -LiteralPath $source -Destination (Join-Path $docsDirectory $docName)
}

Write-PrivateBetaReadme -Path $readmeDestination -Version $Version -InstallerName $installerFileName

Assert-PrivateBetaPackageIsSafe -PackageDirectory $packageDirectory

Write-Host "Private beta package created:"
Write-Host "  $packageDirectory"
Write-Host "SHA256:"
Write-Host "  $($hash.Hash)"
