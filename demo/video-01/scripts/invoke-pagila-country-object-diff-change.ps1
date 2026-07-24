param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Status", "Apply", "Reset")]
    [string]$Action
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ApprovedHosts = @("localhost", "127.0.0.1")
$ApprovedDatabase = "pagila"
$ApprovedUsername = "exitpass"
$ApprovedContainer = "exitpass-postgres"
$ApprovedDockerPort = "5432"
$ApprovedLocalPort = "5433"
$TargetSchema = "public"
$TargetTable = "country"
$TargetColumn = "iso_code"
$TargetColumnType = "character varying"
$TargetColumnLength = 2
$TargetColumnNullable = $true

$ExecutionMode = if ($env:DBSTATE_DEMO_PG_EXECUTION_MODE) { $env:DBSTATE_DEMO_PG_EXECUTION_MODE.ToLowerInvariant() } else { "docker" }
$Container = if ($env:DBSTATE_DEMO_PG_CONTAINER) { $env:DBSTATE_DEMO_PG_CONTAINER } else { $ApprovedContainer }
$HostName = if ($env:DBSTATE_DEMO_PG_HOST) { $env:DBSTATE_DEMO_PG_HOST } else { "127.0.0.1" }
$Port = if ($env:DBSTATE_DEMO_PG_PORT) { $env:DBSTATE_DEMO_PG_PORT } elseif ($ExecutionMode -eq "local") { $ApprovedLocalPort } else { $ApprovedDockerPort }
$Database = if ($env:DBSTATE_DEMO_PG_DATABASE) { $env:DBSTATE_DEMO_PG_DATABASE } else { $ApprovedDatabase }
$Username = if ($env:DBSTATE_DEMO_PG_USERNAME) { $env:DBSTATE_DEMO_PG_USERNAME } else { $ApprovedUsername }
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
        [string]$Message,
        [AllowNull()]
        $State = $null
    )

    [Console]::Error.WriteLine($Message)
    if ($null -ne $State) {
        Write-ResultJson `
            -Status $Status `
            -ColumnExists $State.ColumnExists `
            -DataType $State.DataType `
            -MaximumLength $State.MaximumLength `
            -Nullable $State.Nullable `
            -Changed $false
    } else {
        Write-ResultJson -Status $Status -ColumnExists $false -DataType $null -MaximumLength $null -Nullable $null -Changed $false
    }
    exit 1
}

function Invoke-ProcessCaptured {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    $stdoutFile = [System.IO.Path]::GetTempFileName()
    $stderrFile = [System.IO.Path]::GetTempFileName()
    try {
        & $FilePath @Arguments > $stdoutFile 2> $stderrFile
        $exitCode = $LASTEXITCODE
        if ($null -eq $exitCode) {
            $exitCode = 0
        }
        [pscustomobject]@{
            ExitCode = [int]$exitCode
            Stdout = [System.IO.File]::ReadAllText($stdoutFile)
            Stderr = [System.IO.File]::ReadAllText($stderrFile)
        }
    } finally {
        Remove-Item -LiteralPath $stdoutFile -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $stderrFile -ErrorAction SilentlyContinue
    }
}

function Assert-LocalHost {
    if ($ApprovedHosts -notcontains $HostName) {
        Exit-WithJsonFailure -Status "UnsafeHost" -Message "Refusing to target a non-local PostgreSQL host."
    }
}

function Assert-BaseConfiguration {
    if ($ExecutionMode -ne "docker" -and $ExecutionMode -ne "local") {
        Exit-WithJsonFailure -Status "UnsupportedExecutionMode" -Message "DBSTATE_DEMO_PG_EXECUTION_MODE must be docker or local."
    }
    Assert-LocalHost
    if ($Database -ne $ApprovedDatabase) {
        Exit-WithJsonFailure -Status "UnexpectedDatabase" -Message "Refusing to target an unexpected database. This script is local Pagila demo-only."
    }
    if ($Username -ne $ApprovedUsername) {
        Exit-WithJsonFailure -Status "UnexpectedUsername" -Message "Refusing to target an unexpected PostgreSQL username."
    }
    if (-not $Password) {
        Exit-WithJsonFailure -Status "MissingPassword" -Message "DBSTATE_DEMO_PG_PASSWORD is required and must be supplied through the process environment."
    }
}

function Assert-DockerPreconditions {
    if ([string]::IsNullOrWhiteSpace($Container)) {
        Exit-WithJsonFailure -Status "UnsafeContainer" -Message "DBSTATE_DEMO_PG_CONTAINER is required in Docker mode."
    }
    if ($Container -ne $ApprovedContainer) {
        Exit-WithJsonFailure -Status "UnsafeContainer" -Message "Refusing to target an unexpected Docker container."
    }
    if ($Port -ne $ApprovedDockerPort) {
        Exit-WithJsonFailure -Status "UnexpectedPort" -Message "Docker mode must use PostgreSQL internal port 5432."
    }
    if (-not (Get-Command "docker" -ErrorAction SilentlyContinue)) {
        Exit-WithJsonFailure -Status "DockerUnavailable" -Message "docker was not found on PATH."
    }

    $dockerVersion = Invoke-ProcessCaptured -FilePath "docker" -Arguments @("version", "--format", "{{.Server.Version}}")
    if ($dockerVersion.ExitCode -ne 0) {
        Exit-WithJsonFailure -Status "DockerUnavailable" -Message "Docker is unavailable or the Docker engine cannot be reached."
    }

    $inspect = Invoke-ProcessCaptured `
        -FilePath "docker" `
        -Arguments @("inspect", "--format", "{{.Name}}|{{.State.Running}}", $Container)
    if ($inspect.ExitCode -ne 0) {
        Exit-WithJsonFailure -Status "ContainerMissing" -Message "Required Docker container exitpass-postgres was not found."
    }

    $inspectLine = (($inspect.Stdout -split "`r?`n") | Where-Object { $_.Trim() } | Select-Object -First 1)
    $inspectParts = $inspectLine -split "\|"
    if ($inspectParts.Count -ne 2 -or $inspectParts[0] -ne "/$ApprovedContainer") {
        Exit-WithJsonFailure -Status "UnsafeContainer" -Message "Docker inspection returned an unexpected container identity."
    }
    if ($inspectParts[1] -ne "true") {
        Exit-WithJsonFailure -Status "ContainerStopped" -Message "Required Docker container exitpass-postgres is not running."
    }

    $psqlCheck = Invoke-ProcessCaptured -FilePath "docker" -Arguments @("exec", $Container, "psql", "--version")
    if ($psqlCheck.ExitCode -ne 0) {
        Exit-WithJsonFailure -Status "ContainerPsqlUnavailable" -Message "psql was not found inside exitpass-postgres."
    }
}

function Assert-LocalPsqlPreconditions {
    if ($Port -ne $ApprovedLocalPort) {
        Exit-WithJsonFailure -Status "UnexpectedPort" -Message "Local mode must use the approved Pagila host port 5433."
    }
    if ($env:DBSTATE_DEMO_PSQL_PATH -and -not (Test-Path -LiteralPath $env:DBSTATE_DEMO_PSQL_PATH -PathType Leaf)) {
        Exit-WithJsonFailure -Status "PsqlUnavailable" -Message "DBSTATE_DEMO_PSQL_PATH does not point to a psql executable."
    }
    if (-not $env:DBSTATE_DEMO_PSQL_PATH -and -not (Get-Command $PsqlPath -ErrorAction SilentlyContinue)) {
        Exit-WithJsonFailure -Status "PsqlUnavailable" -Message "psql was not found on PATH. Set DBSTATE_DEMO_PSQL_PATH to a local psql executable or use Docker mode."
    }
}

function Assert-SafeTarget {
    Assert-BaseConfiguration
    if ($ExecutionMode -eq "docker") {
        Assert-DockerPreconditions
    } else {
        Assert-LocalPsqlPreconditions
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
        $psqlArguments = @(
            "-h", $HostName,
            "-p", $Port,
            "-U", $Username,
            "-d", $Database,
            "--tuples-only",
            "--no-align",
            "--field-separator=|",
            "--set=ON_ERROR_STOP=1",
            "--command", $Sql
        )

        if ($ExecutionMode -eq "docker") {
            $result = Invoke-ProcessCaptured `
                -FilePath "docker" `
                -Arguments (@("exec", "-i", "--env", "PGPASSWORD", $Container, "psql") + $psqlArguments)
        } else {
            $result = Invoke-ProcessCaptured -FilePath $PsqlPath -Arguments $psqlArguments
        }

        if ($result.ExitCode -ne 0) {
            throw "psql failed for the local Pagila demo target."
        }
        return $result.Stdout
    } finally {
        if ($null -eq $previousPassword) {
            Remove-Item Env:\PGPASSWORD -ErrorAction SilentlyContinue
        } else {
            $env:PGPASSWORD = $previousPassword
        }
    }
}

function Get-SingleDataRow {
    param(
        [AllowNull()]
        [string]$Stdout,
        [Parameter(Mandatory = $true)]
        [string]$MalformedMessage
    )

    $rows = @($Stdout -split "`r?`n" | Where-Object { $_.Trim().Length -gt 0 } | ForEach-Object { $_.Trim() })
    if ($rows.Count -ne 1) {
        Exit-WithJsonFailure -Status "MalformedState" -Message $MalformedMessage
    }
    return $rows[0]
}

function Test-TableExists {
    $raw = Invoke-DemoPsql -Sql "SELECT CASE WHEN to_regclass('public.country') IS NULL THEN 'false' ELSE 'true' END;"
    $row = Get-SingleDataRow -Stdout $raw -MalformedMessage "Could not verify public.country existence."
    if ($row -eq "true") {
        return $true
    }
    if ($row -eq "false") {
        return $false
    }
    Exit-WithJsonFailure -Status "MalformedState" -Message "Unexpected public.country existence response."
}

function ConvertTo-ColumnState {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RawOutput
    )

    $line = Get-SingleDataRow -Stdout $RawOutput -MalformedMessage "Could not read public.country.iso_code column state."
    $parts = $line -split "\|"
    if ($parts.Count -ne 4) {
        Exit-WithJsonFailure -Status "MalformedState" -Message "Unexpected PostgreSQL state response."
    }

    if ($parts[0] -ne "true" -and $parts[0] -ne "false") {
        Exit-WithJsonFailure -Status "MalformedState" -Message "Unexpected column existence value in PostgreSQL state response."
    }

    $columnExists = $parts[0] -eq "true"
    if (-not $columnExists) {
        if ($parts[1] -ne "" -or $parts[2] -ne "" -or $parts[3] -ne "") {
            Exit-WithJsonFailure -Status "MalformedState" -Message "Absent column state included unexpected metadata."
        }
        return [pscustomobject]@{
            ColumnExists = $false
            DataType = $null
            MaximumLength = $null
            Nullable = $null
        }
    }

    if ($parts[1] -eq "" -or $parts[2] -eq "" -or $parts[3] -eq "") {
        Exit-WithJsonFailure -Status "MalformedState" -Message "Present column state was missing metadata."
    }

    $parsedLength = 0
    if (-not [int]::TryParse($parts[2], [ref]$parsedLength)) {
        Exit-WithJsonFailure -Status "MalformedState" -Message "Column maximum length was not a valid integer."
    }

    $nullable = if ($parts[3] -eq "YES") {
        $true
    } elseif ($parts[3] -eq "NO") {
        $false
    } else {
        Exit-WithJsonFailure -Status "MalformedState" -Message "Column nullability was not a valid PostgreSQL value."
    }

    [pscustomobject]@{
        ColumnExists = $true
        DataType = $parts[1]
        MaximumLength = $parsedLength
        Nullable = $nullable
    }
}

function Get-ColumnState {
    $sql = @"
SELECT
    CASE WHEN c.column_name IS NULL THEN 'false' ELSE 'true' END,
    COALESCE(c.data_type, ''),
    COALESCE(c.character_maximum_length::text, ''),
    COALESCE(c.is_nullable, '')
FROM (SELECT 1) AS seed
LEFT JOIN information_schema.columns AS c
    ON c.table_schema = 'public'
   AND c.table_name = 'country'
   AND c.column_name = 'iso_code';
"@
    ConvertTo-ColumnState -RawOutput (Invoke-DemoPsql -Sql $sql)
}

function Assert-ExpectedColumnShape {
    param([Parameter(Mandatory = $true)]$State)
    if (
        -not $State.ColumnExists -or
        $State.DataType -ne $TargetColumnType -or
        $State.MaximumLength -ne $TargetColumnLength -or
        $State.Nullable -ne $TargetColumnNullable
    ) {
        Exit-WithJsonFailure -Status "VerificationFailed" -Message "public.country.iso_code did not match the expected nullable varchar(2) shape." -State $State
    }
}

Assert-SafeTarget

try {
    if (-not (Test-TableExists)) {
        Exit-WithJsonFailure -Status "TableMissing" -Message "Required table public.country was not found."
    }

    $state = Get-ColumnState
    if ($state.ColumnExists) {
        Assert-ExpectedColumnShape -State $state
    }

    if ($Action -eq "Status") {
        Write-ResultJson -Status "Ready" -ColumnExists $state.ColumnExists -DataType $state.DataType -MaximumLength $state.MaximumLength -Nullable $state.Nullable -Changed $state.ColumnExists
        exit 0
    }

    if ($Action -eq "Apply") {
        if ($state.ColumnExists) {
            Exit-WithJsonFailure -Status "AlreadyApplied" -Message "public.country.iso_code already exists. Reset the demo database before applying this change." -State $state
        }
        $null = Invoke-DemoPsql -Sql "ALTER TABLE public.country ADD COLUMN iso_code varchar(2);"
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
        $null = Invoke-DemoPsql -Sql "ALTER TABLE public.country DROP COLUMN iso_code;"
        $after = Get-ColumnState
        if ($after.ColumnExists) {
            Exit-WithJsonFailure -Status "VerificationFailed" -Message "public.country.iso_code still exists after reset." -State $after
        }
        Write-ResultJson -Status "Reset" -ColumnExists $false -DataType $null -MaximumLength $null -Nullable $null -Changed $true
        exit 0
    }
} catch {
    Exit-WithJsonFailure -Status "ConnectionFailed" -Message "The local Pagila demo database could not be reached or verified."
}
