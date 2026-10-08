# Downloads the pinned PDFium binaries (scripts/pdfium.toml) into vendor/pdfium/<target>/
# and verifies their SHA-256. Idempotent: a target whose checksum stamp matches is skipped.
# Windows counterpart of fetch-pdfium.sh. Needs tar.exe (Windows 10 1803 and later).
#
# Usage: ./scripts/fetch-pdfium.ps1 [-All] [<target>...]
#   no argument   the host target (win-x64 or win-arm64)
#   -All          every target in pdfium.toml (for packaging)
#   <target>      one or more targets by name, for example win-arm64
[CmdletBinding()]
param(
    [switch]$All,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Targets
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Root = Split-Path -Parent $PSScriptRoot
$Config = Join-Path $Root 'scripts/pdfium.toml'
$Vendor = Join-Path $Root 'vendor/pdfium'

# Parses the simple pdfium.toml format: top level keys and [targets.<name>] sections.
function Read-PdfiumConfig {
    $top = @{}
    $targets = [ordered]@{}
    $current = $null
    foreach ($line in Get-Content -LiteralPath $Config) {
        if ($line -match '^\s*#' -or $line -match '^\s*$') { continue }
        if ($line -match '^\[targets\.([^\]]+)\]\s*$') {
            $current = @{}
            $targets[$Matches[1]] = $current
            continue
        }
        if ($line -match '^\s*([A-Za-z0-9_]+)\s*=\s*"([^"]*)"') {
            if ($null -eq $current) { $top[$Matches[1]] = $Matches[2] } else { $current[$Matches[1]] = $Matches[2] }
        }
    }
    return @{ Top = $top; Targets = $targets }
}

function Get-HostTarget {
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($env:PROCESSOR_ARCHITEW6432) { $arch = $env:PROCESSOR_ARCHITEW6432 }
    switch ($arch) {
        'AMD64' { return 'win-x64' }
        'ARM64' { return 'win-arm64' }
        default { throw "unsupported architecture: $arch" }
    }
}

function Get-Pdfium([hashtable]$Cfg, [string]$Target) {
    $t = $Cfg.Targets[$Target]
    if ($null -eq $t) {
        throw "unknown target '$Target' (known: $($Cfg.Targets.Keys -join ' '))"
    }
    $dest = Join-Path $Vendor $Target
    $stamp = Join-Path $dest '.sha256'
    $library = Join-Path $dest $t.library
    if ((Test-Path -LiteralPath $stamp) -and (Test-Path -LiteralPath $library) -and
        ((Get-Content -LiteralPath $stamp -Raw).Trim() -eq $t.sha256)) {
        Write-Host "pdfium ${Target}: up to date ($dest)"
        return
    }

    New-Item -ItemType Directory -Force -Path $Vendor | Out-Null
    $tmp = Join-Path $Vendor (".fetch-$Target-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $archive = Join-Path $tmp $t.archive
        Write-Host "pdfium ${Target}: downloading $($t.archive)"
        Invoke-WebRequest -UseBasicParsing -Uri "$($Cfg.Top.base_url)/$($t.archive)" -OutFile $archive
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
        if ($actual -ne $t.sha256) {
            throw "pdfium ${Target}: checksum mismatch for $($t.archive)`n  expected $($t.sha256)`n  actual   $actual"
        }
        $out = Join-Path $tmp 'out'
        New-Item -ItemType Directory -Path $out | Out-Null
        tar -xzf $archive -C $out
        if ($LASTEXITCODE -ne 0) { throw "pdfium ${Target}: tar failed with exit code $LASTEXITCODE" }
        if (-not (Test-Path -LiteralPath (Join-Path $out $t.library))) {
            throw "pdfium ${Target}: $($t.library) missing in $($t.archive)"
        }
        Set-Content -LiteralPath (Join-Path $out '.sha256') -Value $t.sha256 -NoNewline
        if (Test-Path -LiteralPath $dest) { Remove-Item -Recurse -Force -LiteralPath $dest }
        Move-Item -LiteralPath $out -Destination $dest
        Write-Host "pdfium ${Target}: installed $library"
    }
    finally {
        if (Test-Path -LiteralPath $tmp) { Remove-Item -Recurse -Force -LiteralPath $tmp }
    }
}

$cfg = Read-PdfiumConfig
if ($All) {
    $list = @($cfg.Targets.Keys)
}
elseif ($Targets -and $Targets.Count -gt 0) {
    $list = $Targets
}
else {
    $list = @(Get-HostTarget)
}
foreach ($t in $list) { Get-Pdfium $cfg $t }
