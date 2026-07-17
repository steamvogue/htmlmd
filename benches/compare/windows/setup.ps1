# SPDX-License-Identifier: MIT OR Apache-2.0
<#
.SYNOPSIS
    Fetch the tools needed to benchmark htmlmd against other HTML->Markdown
    converters on Windows.

.DESCRIPTION
    Everything is downloaded into .\tools\ next to this script: no admin
    rights, no PATH changes, nothing installed system-wide. Delete the
    tools\ folder to undo.

    Versions are pinned so results stay comparable between runs and machines.
    Bump them deliberately, and say so when you publish numbers.

    Python is the one thing not fetched portably (pip inside the embeddable
    distribution is more trouble than it is worth). If a system Python is
    found, a venv with markdownify is created; otherwise markdownify is
    skipped and run.ps1 will simply leave it out of the comparison.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File setup.ps1
#>

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# $LASTEXITCODE does not exist until a native command has run, and reading an
# undefined variable is a terminating error under StrictMode.
$global:LASTEXITCODE = 0

function Test-NativeSucceeds {
    <#
       Run a command that is EXPECTED to fail sometimes, and report success as
       a bool. PowerShell 7.4+ turns a non-zero native exit code into a
       terminating error when $ErrorActionPreference is 'Stop', so probes have
       to opt out of that explicitly.
    #>
    param([Parameter(Mandatory)][string] $Exe, [string[]] $Arguments = @())
    try {
        $global:LASTEXITCODE = 0
        & $Exe @Arguments *> $null
        return ($global:LASTEXITCODE -eq 0)
    } catch {
        return $false
    }
}

# --- pinned versions -------------------------------------------------------
$HyperfineVersion = '1.20.0'
$PandocVersion    = '3.10'
$Html2mdVersion   = '2.5.2'
$NodeVersion      = '24.18.0'

$Root  = $PSScriptRoot
$Tools = Join-Path $Root 'tools'
New-Item -ItemType Directory -Force -Path $Tools | Out-Null

# TLS 1.2 for older PowerShell hosts that still default to SSL3/TLS1.0.
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Install-FromZip {
    <#
       Download a zip, find one named executable anywhere inside it, and copy
       that to tools\. Locating the exe by search rather than by hardcoded
       path keeps this working when upstream changes its archive layout.
    #>
    param(
        [Parameter(Mandatory)][string] $Name,
        [Parameter(Mandatory)][string] $Url,
        [Parameter(Mandatory)][string] $ExeName
    )

    $target = Join-Path $Tools $ExeName
    if (Test-Path $target) {
        Write-Host "  $Name already present, skipping" -ForegroundColor DarkGray
        return
    }

    Write-Host "==> $Name" -ForegroundColor Cyan
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("htmlmd-bench-" + [Guid]::NewGuid())
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    try {
        $zip = Join-Path $tmp 'download.zip'
        Write-Host "    fetching $Url"
        Invoke-WebRequest -Uri $Url -OutFile $zip -UseBasicParsing
        Expand-Archive -LiteralPath $zip -DestinationPath $tmp -Force

        $found = Get-ChildItem -Path $tmp -Recurse -Filter $ExeName -File |
                 Select-Object -First 1
        if (-not $found) {
            throw "$ExeName not found inside $Url"
        }
        Copy-Item -LiteralPath $found.FullName -Destination $target -Force
        Write-Host "    -> tools\$ExeName" -ForegroundColor Green
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Fetching benchmark tools into $Tools" -ForegroundColor White
Write-Host ""

# --- hyperfine (the benchmark driver; required) ----------------------------
Install-FromZip -Name "hyperfine $HyperfineVersion" -ExeName 'hyperfine.exe' `
    -Url "https://github.com/sharkdp/hyperfine/releases/download/v$HyperfineVersion/hyperfine-v$HyperfineVersion-x86_64-pc-windows-msvc.zip"

# --- pandoc ----------------------------------------------------------------
Install-FromZip -Name "pandoc $PandocVersion" -ExeName 'pandoc.exe' `
    -Url "https://github.com/jgm/pandoc/releases/download/$PandocVersion/pandoc-$PandocVersion-windows-x86_64.zip"

# --- html-to-markdown v2 (Go) ----------------------------------------------
Install-FromZip -Name "html-to-markdown $Html2mdVersion" -ExeName 'html2markdown.exe' `
    -Url "https://github.com/JohannesKaufmann/html-to-markdown/releases/download/v$Html2mdVersion/html-to-markdown_Windows_x86_64.zip"

# --- Node + turndown -------------------------------------------------------
$NodeDir = Join-Path $Tools 'node'
if (Test-Path (Join-Path $NodeDir 'node.exe')) {
    Write-Host "  node already present, skipping" -ForegroundColor DarkGray
} else {
    Write-Host "==> node $NodeVersion" -ForegroundColor Cyan
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("htmlmd-node-" + [Guid]::NewGuid())
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    try {
        $zip = Join-Path $tmp 'node.zip'
        $url = "https://nodejs.org/dist/v$NodeVersion/node-v$NodeVersion-win-x64.zip"
        Write-Host "    fetching $url"
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
        Expand-Archive -LiteralPath $zip -DestinationPath $tmp -Force
        $extracted = Join-Path $tmp "node-v$NodeVersion-win-x64"
        Move-Item -LiteralPath $extracted -Destination $NodeDir -Force
        Write-Host "    -> tools\node\" -ForegroundColor Green
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# node_modules has to sit next to adapters\, which the repo keeps one level up
# and the standalone kit ships beside this script: adapters\turndown.js calls
# require('turndown'), and Node resolves that from the script's own directory
# upward, never from the working directory. Left to itself npm picks the root
# by walking up for the nearest package.json, which lands outside the kit in
# one layout or the other -- and exits 0 either way, so --prefix is what keeps
# setup.ps1 and run.ps1 agreeing on where the packages went.
$NodeRoot = if (Test-Path (Join-Path $Root 'adapters')) {
    $Root
} else {
    (Resolve-Path (Join-Path $Root '..')).Path
}

Write-Host "==> turndown + turndown-plugin-gfm" -ForegroundColor Cyan
$npm = Join-Path $NodeDir 'npm.cmd'
& $npm install --prefix $NodeRoot --no-fund --no-audit --silent turndown turndown-plugin-gfm
if ($LASTEXITCODE -ne 0) { throw "npm install failed ($LASTEXITCODE)" }
Write-Host ("    -> " + (Join-Path $NodeRoot 'node_modules')) -ForegroundColor Green

# --- Python + markdownify (optional) ---------------------------------------
Write-Host "==> markdownify (optional)" -ForegroundColor Cyan
$python = $null
foreach ($candidate in @('py', 'python', 'python3')) {
    $cmd = Get-Command $candidate -ErrorAction SilentlyContinue
    if ($cmd) {
        # 'python' on a bare Windows install is a Store stub that does nothing.
        if (Test-NativeSucceeds -Exe $cmd.Source -Arguments @('-c', 'import sys')) {
            $python = $cmd.Source
            break
        }
    }
}

if (-not $python) {
    Write-Warning "  no working Python found - markdownify will be skipped."
    Write-Warning "  Install from https://www.python.org (or 'winget install Python.Python.3.13'), then re-run."
} else {
    $venv = Join-Path $Tools 'venv'
    if (-not (Test-Path (Join-Path $venv 'Scripts\python.exe'))) {
        & $python -m venv $venv
        if ($LASTEXITCODE -ne 0) { throw "venv creation failed ($LASTEXITCODE)" }
    }
    & (Join-Path $venv 'Scripts\python.exe') -m pip install --quiet --upgrade pip markdownify
    if ($LASTEXITCODE -ne 0) { throw "pip install markdownify failed ($LASTEXITCODE)" }
    Write-Host "    -> tools\venv\" -ForegroundColor Green
}

# --- report ----------------------------------------------------------------
Write-Host ""
Write-Host "Done. Installed:" -ForegroundColor White
foreach ($t in @('hyperfine.exe', 'pandoc.exe', 'html2markdown.exe')) {
    $p = Join-Path $Tools $t
    $state = if (Test-Path $p) { "OK" } else { "MISSING" }
    Write-Host ("  {0,-20} {1}" -f $t, $state)
}
Write-Host ("  {0,-20} {1}" -f 'node', $(if (Test-Path (Join-Path $NodeDir 'node.exe')) { "OK" } else { "MISSING" }))
# Checked at the exact path run.ps1 looks in, so the two cannot disagree.
Write-Host ("  {0,-20} {1}" -f 'turndown', $(if (Test-Path (Join-Path $NodeRoot 'node_modules\turndown')) { "OK" } else { "MISSING" }))
Write-Host ("  {0,-20} {1}" -f 'markdownify', $(if (Test-Path (Join-Path $Tools 'venv\Scripts\python.exe')) { "OK" } else { "skipped" }))

Write-Host ""
Write-Host "Next: copy htmlmd.exe into this folder, then run:" -ForegroundColor White
Write-Host "  powershell -ExecutionPolicy Bypass -File run.ps1" -ForegroundColor Yellow
