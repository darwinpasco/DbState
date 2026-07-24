param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Status", "Apply", "Reset")]
    [string]$Action
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ApprovedHosts = @("localhost", "127.0.0.1")
$ApprovedDatabase = "pagila"
$TargetSchema = "public"
$TargetTable = "country"
$TargetColumn = "iso_code"
$TargetColumnType = "character varying"
$TargetColumnLength = 2
$TargetColumnNullable = $true

$HostName = if ($env:DBSTATE_DEMO_PG_HOST) { $env:DBSTATE_DEMO_PG_HOST } else { "localhost" }
$Port = if ($env:DBSTATE_DEMO_PG_PORT) { $env:DBSTATE_DEMO_PG_PORT } else { "5433" }
$Database = if ($env:DBSTATE_DEMO_PG_DATABASE) { $env:DBSTATE_DEMO_PG_DATABASE } else { $ApprovedDatabase }
$Username = if ($env:DBSTATE_DEMO_PG_USERNAME) { $env:DBSTATE_DEMO_PG_USERNAME } else { "exitpass" }
$PsqlPath = if ($env:DBSTATE_DEMO_PSQL_PATH) { $env:DBSTATE_DEMO_PSQL_PATH } else { "psql" }
$Password = $env:DBSTATE_DEMO_PG_PASSWORD

function Write-ResultJson {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Status,
        [Parameter(Mandatory = $true)]
        [bool]$ColumnExists,
        [AllowNull()]
        $DataType,
        [AllowNull()]
        $MaximumLength,
        [AllowNull()]
        $Nullable,
        [Parameter(Mandatory = $true)]
        [bool]$Changed
    )

    [ordered]@{
        action = $Action
        status = $Status
        database = $Database
        table = "$TargetSchema.$TargetTable"
        column = $TargetColumn
        columnExists = $ColumnExists
        dataType = $DataType
        maximumLength = $MaximumLength
        nullable = $Nullable
        changed = $Changed
    } | ConvertTo-Json -Compress
}

function Exit-WithJsonFailure {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Status,
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    [Console]::Error.WriteLine($Message)
    Write-ResultJson -Status $Status -ColumnExists $false -DataType $null -MaximumLength $null -Nullable $null -Changed $false
    exit 1
}

function Assert-SafeTarget {
    if ($ApprovedHosts -notcontains $HostName) {
        Exit-WithJsonFailure -Status "UnsafeHost" -Message "Refusing to target a non-local PostgreSQL host."
    }
    if ($Database -ne $ApprovedDatabase) {
        Exit-WithJsonFailure -Status "UnexpectedDatabase" -Message "Refusing to target an unexpected database. This script is local Pagila demo-only."
    }
    if (-not $Password) {
        Exit-WithJsonFailure -Status "MissingPassword" -Message "DBSTATE_DEMO_PG_PASSWORD is required and must be supplied through the process environment."
    }
    if ($env:DBSTATE_DEMO_PSQL_PATH -and -not (Test-Path -LiteralPath $env:DBSTATE_DEMO_PSQL_PATH -PathType Leaf)) {
        Exit-WithJsonFailure -Status "PsqlUnavailable" -Message "DBSTATE_DEMO_PSQL_PATH does not point to a psql executable."
    }
    if (-not $env:DBSTATE_DEMO_PSQL_PATH -and -not (Get-Command $PsqlPath -ErrorAction SilentlyContinue)) {
        Exit-WithJsonFailure -Status "PsqlUnavailable" -Message "psql was not found on PATH. Set DBSTATE_DEMO_PSQL_PATH to a local psql executable."
    }
}

function Invoke-DemoPsql {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Sql
    )

    $previousPassword = $env:PGPASSWORD
    try {
        $env:PGPASSWORD = $Password
        $arguments = @(
            "-h", $HostName,
            "-p", $Port,
            "-U", $Username,
            "-d", $Database,
            "-v", "ON_ERROR_STOP=1",
            "-t",
            "-A",
            "-F", "`t",
            "-c", $Sql
        )
        $output = & $PsqlPath @arguments 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "psql failed for the local Pagila demo target."
        }
        return ($output | Out-String).Trim()
    } finally {
        if ($null -eq $previousPassword) {
            Remove-Item Env:\PGPASSWORD -ErrorAction SilentlyContinue
        } else {
            $env:PGPASSWORD = $previousPassword
        }
    }
}

function Get-ColumnState {
    $sql = @"
SELECT
  current_database(),
  CASE WHEN to_regclass('public.country') IS NULL THEN 'false' ELSE 'true' END,
  COALESCE(c.data_type, ''),
  COALESCE(c.character_maximum_length::text, ''),
  COALESCE(c.is_nullable, '')
FROM (SELECT 1) AS marker
LEFT JOIN information_schema.columns AS c
  ON c.table_schema = 'public'
 AND c.table_name = 'country'
 AND c.column_name = 'iso_code';
"@
    $raw = Invoke-DemoPsql -Sql $sql
    $line = ($raw -split "`r?`n" | Where-Object { $_.Trim() } | Select-Object -First 1)
    if (-not $line) {
        Exit-WithJsonFailure -Status "MalformedState" -Message "Could not read public.country column state."
    }
    $parts = $line -split "`t", 5
    if ($parts.Count -lt 5) {
        Exit-WithJsonFailure -Status "MalformedState" -Message "Unexpected PostgreSQL state response."
    }
    $columnExists = $parts[2] -ne ""
    $maximumLength = if ($parts[3] -ne "") { [int]$parts[3] } else { $null }
    $nullable = if ($parts[4] -eq "YES") { $true } elseif ($parts[4] -eq "NO") { $false } else { $null }
    [pscustomobject]@{
        Database = $parts[0]
        TableExists = $parts[1] -eq "true"
        ColumnExists = $columnExists
        DataType = if ($columnExists) { $parts[2] } else { $null }
        MaximumLength = $maximumLength
        Nullable = $nullable
    }
}

function Assert-TableExists {
    param([Parameter(Mandatory = $true)]$State)
    if (-not $State.TableExists) {
        Exit-WithJsonFailure -Status "TableMissing" -Message "Required table public.country was not found."
    }
}

function Assert-ExpectedColumnShape {
    param([Parameter(Mandatory = $true)]$State)
    if (
        -not $State.ColumnExists -or
        $State.DataType -ne $TargetColumnType -or
        $State.MaximumLength -ne $TargetColumnLength -or
        $State.Nullable -ne $TargetColumnNullable
    ) {
        Exit-WithJsonFailure -Status "VerificationFailed" -Message "public.country.iso_code did not match the expected nullable varchar(2) shape."
    }
}

Assert-SafeTarget

try {
    $state = Get-ColumnState
    if ($state.Database -ne $ApprovedDatabase) {
        Exit-WithJsonFailure -Status "UnexpectedDatabase" -Message "Connected to an unexpected database."
    }
    Assert-TableExists -State $state

    if ($Action -eq "Status") {
        Write-ResultJson -Status "Ready" -ColumnExists $state.ColumnExists -DataType $state.DataType -MaximumLength $state.MaximumLength -Nullable $state.Nullable -Changed $state.ColumnExists
        exit 0
    }

    if ($Action -eq "Apply") {
        if ($state.ColumnExists) {
            Exit-WithJsonFailure -Status "AlreadyApplied" -Message "public.country.iso_code already exists. Reset the demo database before applying this change."
        }
        Invoke-DemoPsql -Sql "ALTER TABLE public.country ADD COLUMN iso_code varchar(2);"
        $after = Get-ColumnState
        Assert-ExpectedColumnShape -State $after
        Write-ResultJson -Status "Applied" -ColumnExists $true -DataType $after.DataType -MaximumLength $after.MaximumLength -Nullable $after.Nullable -Changed $true
        exit 0
    }

    if ($Action -eq "Reset") {
        if (-not $state.ColumnExists) {
            Write-ResultJson -Status "AlreadyReset" -ColumnExists $false -DataType $null -MaximumLength $null -Nullable $null -Changed $false
            exit 0
        }
        Invoke-DemoPsql -Sql "ALTER TABLE public.country DROP COLUMN iso_code;"
        $after = Get-ColumnState
        if ($after.ColumnExists) {
            Exit-WithJsonFailure -Status "VerificationFailed" -Message "public.country.iso_code still exists after reset."
        }
        Write-ResultJson -Status "Reset" -ColumnExists $false -DataType $null -MaximumLength $null -Nullable $null -Changed $true
        exit 0
    }
} catch {
    Exit-WithJsonFailure -Status "ConnectionFailed" -Message "The local Pagila demo database could not be reached or verified."
}
