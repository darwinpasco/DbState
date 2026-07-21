[CmdletBinding()]
param(
    [string]$Configuration = "release",
    [string]$InstallerVersion = "v0.1.0-private-beta.2",
    [string]$InnoSetupCompiler
)

$ErrorActionPreference = "Stop"

function Resolve-RepoRoot {
    $scriptDirectory = Split-Path -Parent $PSCommandPath
    return (Resolve-Path (Join-Path $scriptDirectory "..\..")).Path
}

function New-CleanDirectory {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Find-InnoSetupCompiler {
    param([string]$ExplicitPath)

    if ($ExplicitPath) {
        if (Test-Path -LiteralPath $ExplicitPath) {
            return (Resolve-Path -LiteralPath $ExplicitPath).Path
        }
        throw "Inno Setup compiler was not found at '$ExplicitPath'."
    }

    if ($env:ISCC_EXE -and (Test-Path -LiteralPath $env:ISCC_EXE)) {
        return (Resolve-Path -LiteralPath $env:ISCC_EXE).Path
    }

    $candidates = @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    return $null
}

function Find-Cargo {
    $command = Get-Command cargo -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $candidate = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path -LiteralPath $candidate) {
        return (Resolve-Path -LiteralPath $candidate).Path
    }

    throw "Cargo was not found. Install Rust, or make cargo.exe available on PATH before running this packaging script."
}

function Assert-StagingIsSafe {
    param([Parameter(Mandatory = $true)][string]$StagingDirectory)

    $forbiddenNames = @(
        "src",
        ".git",
        "Cargo.toml",
        "Cargo.lock",
        "tests",
        "debug",
        "connection-profiles.json",
        ".env"
    )

    $forbiddenExtensions = @(".rs", ".ps1")
    $items = Get-ChildItem -LiteralPath $StagingDirectory -Recurse -Force

    foreach ($item in $items) {
        if ($forbiddenNames -contains $item.Name) {
            throw "Forbidden item staged for installer: $($item.FullName)"
        }
        if (-not $item.PSIsContainer -and ($forbiddenExtensions -contains $item.Extension)) {
            throw "Forbidden source/script file staged for installer: $($item.FullName)"
        }
        if ($item.FullName -like "*\target\debug\*") {
            throw "Debug build output staged for installer: $($item.FullName)"
        }
    }

    $textFiles = Get-ChildItem -LiteralPath $StagingDirectory -Recurse -File -Force |
        Where-Object { $_.Extension -in @(".txt", ".md", ".json", ".yml", ".yaml", ".config", ".ini") }

    foreach ($file in $textFiles) {
        $content = Get-Content -LiteralPath $file.FullName -Raw -ErrorAction Stop
        if ($content -match "postgres://.*:.*@") {
            throw "Potential PostgreSQL credential URL found in staged file: $($file.FullName)"
        }
    }
}

$repoRoot = Resolve-RepoRoot
$packagingRoot = Join-Path $repoRoot "packaging\windows"
$stagingDirectory = Join-Path $packagingRoot "staging"
$outputDirectory = Join-Path $packagingRoot "out"
$issPath = Join-Path $packagingRoot "DbState.iss"
$binaryPath = Join-Path $repoRoot "target\$Configuration\dbstate.exe"
$cargoExe = Find-Cargo
$cargoVersionLine = Select-String -Path (Join-Path $repoRoot "Cargo.toml") -Pattern '^version\s*=' | Select-Object -First 1
$cargoPackageVersion = if ($cargoVersionLine -and $cargoVersionLine.Line -match '"([^"]+)"') {
    $Matches[1]
} else {
    "unknown"
}

Write-Host "DbState Windows installer packaging"
Write-Host "Repository: $repoRoot"
Write-Host "Version: $InstallerVersion"
Write-Host "Cargo: $cargoExe"

Push-Location $repoRoot
try {
    & $cargoExe build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release failed."
    }
}
finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $binaryPath)) {
    throw "Release binary was not found at $binaryPath."
}

New-CleanDirectory -Path $stagingDirectory
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null

Copy-Item -LiteralPath $binaryPath -Destination (Join-Path $stagingDirectory "dbstate.exe")
Copy-Item -LiteralPath (Join-Path $packagingRoot "README-installer.md") -Destination (Join-Path $stagingDirectory "README.txt")

$licenseCandidates = @("LICENSE.txt", "LICENSE.md", "LICENSE", "COPYING")
foreach ($license in $licenseCandidates) {
    $candidate = Join-Path $repoRoot $license
    if (Test-Path -LiteralPath $candidate) {
        Copy-Item -LiteralPath $candidate -Destination (Join-Path $stagingDirectory "LICENSE.txt")
        break
    }
}

@"
DbState PostgreSQL private beta
Installer version: $InstallerVersion
Cargo package version: $cargoPackageVersion
Build configuration: $Configuration
"@ | Set-Content -LiteralPath (Join-Path $stagingDirectory "VERSION.txt") -Encoding UTF8

Assert-StagingIsSafe -StagingDirectory $stagingDirectory

$iscc = Find-InnoSetupCompiler -ExplicitPath $InnoSetupCompiler
if (-not $iscc) {
    Write-Error @"
Inno Setup compiler was not found.

Install Inno Setup 6, or set ISCC_EXE to the full path of ISCC.exe, then rerun:
  .\packaging\windows\Build-WindowsInstaller.ps1

Staging validation succeeded at:
  $stagingDirectory
"@
    exit 1
}

& $iscc "/DAppVersion=$InstallerVersion" "/DSourceDir=$stagingDirectory" "/DOutputDir=$outputDirectory" $issPath
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compiler failed."
}

$installerPath = Join-Path $outputDirectory "DbState-PostgreSQL-$InstallerVersion-setup.exe"
Write-Host "Installer created: $installerPath"
