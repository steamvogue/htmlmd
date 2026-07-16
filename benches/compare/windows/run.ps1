# SPDX-License-Identifier: MIT OR Apache-2.0
<#
.SYNOPSIS
    Benchmark htmlmd against other HTML->Markdown converters on Windows.

.DESCRIPTION
    The Windows counterpart of ../run.sh, with the same rules: every tool
    converts byte-identical input from the shared synthetic corpus, is asked
    for GFM-style output where it has a flag for it, and is timed as a full
    CLI invocation (process and interpreter startup included). Tools that are
    not installed are skipped with a note rather than failing the run.

    Requires: setup.ps1 to have been run, htmlmd.exe and dump_corpus.exe in
    this folder (or in ..\..\..\dist\x86_64-pc-windows-gnu\).

.PARAMETER OutputProfile
    htmlmd output profile to benchmark. Default: gfm. (Not named -Profile:
    $Profile is a PowerShell automatic variable.)

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File run.ps1
#>

param(
    [string] $OutputProfile = 'gfm'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# See setup.ps1: unset until a native command runs, and fatal to read
# under StrictMode.
$global:LASTEXITCODE = 0

$Root    = $PSScriptRoot
$Tools   = Join-Path $Root 'tools'
$Corpus  = Join-Path $Root 'corpus'
$Results = Join-Path $Root 'results'

function Find-Exe {
    param([string] $Name)
    foreach ($dir in @($Root, (Join-Path $Root '..\..\..\dist\x86_64-pc-windows-gnu'))) {
        $p = Join-Path $dir $Name
        if (Test-Path $p) { return (Resolve-Path $p).Path }
    }
    return $null
}

$Hyperfine = Join-Path $Tools 'hyperfine.exe'
if (-not (Test-Path $Hyperfine)) {
    throw "hyperfine not found. Run setup.ps1 first."
}

$Htmlmd = Find-Exe 'htmlmd.exe'
if (-not $Htmlmd) {
    throw "htmlmd.exe not found. Copy it next to this script (or into dist\x86_64-pc-windows-gnu\)."
}

# --- corpus ----------------------------------------------------------------
$DumpCorpus = Find-Exe 'dump_corpus.exe'
if (Test-Path $Corpus) {
    Write-Host "Using existing corpus in $Corpus" -ForegroundColor DarkGray
} elseif ($DumpCorpus) {
    Write-Host "==> Generating corpus" -ForegroundColor Cyan
    & $DumpCorpus $Corpus
    if ($LASTEXITCODE -ne 0) { throw "dump_corpus failed ($LASTEXITCODE)" }
} else {
    throw "No corpus\ folder and no dump_corpus.exe to generate one."
}

New-Item -ItemType Directory -Force -Path $Results | Out-Null

# --- assemble the tool list ------------------------------------------------
$Node    = Join-Path $Tools 'node\node.exe'
$Venv    = Join-Path $Tools 'venv\Scripts\python.exe'
$H2m     = Join-Path $Tools 'html2markdown.exe'
$Pandoc  = Join-Path $Tools 'pandoc.exe'
$Adapters = Join-Path $Root '..\adapters'

Write-Host ""
Write-Host "htmlmd:  $Htmlmd" -ForegroundColor White
Write-Host "profile: $OutputProfile" -ForegroundColor White
Write-Host ""

foreach ($doc in @('wiki', 'news', 'docs', 'tables')) {
    $inputPath = Join-Path $Corpus "$doc.html"
    if (-not (Test-Path $inputPath)) {
        Write-Warning "corpus\$doc.html missing, skipping"
        continue
    }

    $cmds = @('--command-name', 'htmlmd', "`"$Htmlmd`" --profile $OutputProfile `"$inputPath`"")

    if ((Test-Path $Node) -and (Test-Path (Join-Path $Root 'node_modules\turndown'))) {
        $script = Join-Path $Adapters 'turndown.js'
        $cmds += @('--command-name', 'turndown', "`"$Node`" `"$script`" `"$inputPath`"")
    } else {
        Write-Warning "turndown skipped (run setup.ps1)"
    }

    if (Test-Path $Venv) {
        $script = Join-Path $Adapters 'markdownify_adapter.py'
        $cmds += @('--command-name', 'markdownify', "`"$Venv`" `"$script`" `"$inputPath`"")
    } else {
        Write-Warning "markdownify skipped (no Python venv)"
    }

    if (Test-Path $H2m) {
        # html2markdown reads stdin; cmd.exe redirection keeps it comparable.
        $cmds += @('--command-name', 'html2markdown-v2',
                   "`"$H2m`" --plugin-table --plugin-strikethrough < `"$inputPath`"")
    } else {
        Write-Warning "html2markdown skipped (run setup.ps1)"
    }

    if (Test-Path $Pandoc) {
        $cmds += @('--command-name', 'pandoc',
                   "`"$Pandoc`" -f html -t gfm --wrap=none `"$inputPath`"")
    } else {
        Write-Warning "pandoc skipped (run setup.ps1)"
    }

    $size = (Get-Item $inputPath).Length
    Write-Host "==> $doc ($size bytes)" -ForegroundColor Cyan
    & $Hyperfine --warmup 2 --min-runs 5 --output null `
        --export-json (Join-Path $Results "$doc.json") @cmds
    if ($LASTEXITCODE -ne 0) { Write-Warning "hyperfine returned $LASTEXITCODE for $doc" }
}

# --- summary ---------------------------------------------------------------
# Implemented here rather than reusing ../summarize.py so the summary works
# even when Python is absent (markdownify is the only Python dependency).
Write-Host ""
$docs = @('wiki', 'news', 'docs', 'tables') | Where-Object {
    Test-Path (Join-Path $Results "$_.json")
}
if (-not $docs) { Write-Warning "no results to summarize"; exit 0 }

$rows = @{}
$tools = [System.Collections.ArrayList]@()
foreach ($doc in $docs) {
    $json = Get-Content (Join-Path $Results "$doc.json") -Raw | ConvertFrom-Json
    foreach ($r in $json.results) {
        if (-not $tools.Contains($r.command)) { [void]$tools.Add($r.command) }
        if (-not $rows.ContainsKey($r.command)) { $rows[$r.command] = @{} }
        $rows[$r.command][$doc] = @{ mean = $r.mean; stddev = $r.stddev }
    }
}

$baseline = @{}
foreach ($doc in $docs) {
    if ($rows.ContainsKey('htmlmd') -and $rows['htmlmd'].ContainsKey($doc)) {
        $baseline[$doc] = $rows['htmlmd'][$doc].mean
    }
}

# Invariant culture throughout: the machine's locale must not decide whether
# 1482 ms prints as "1,482" or "1.482" in a table of milliseconds.
$inv = [System.Globalization.CultureInfo]::InvariantCulture

"| tool | " + ($docs -join ' | ') + " |"
"|---|" + ("---|" * $docs.Count)
foreach ($tool in $tools) {
    $cells = foreach ($doc in $docs) {
        if ($rows[$tool].ContainsKey($doc)) {
            $m = $rows[$tool][$doc].mean * 1000
            $s = $rows[$tool][$doc].stddev * 1000
            $rel = ''
            if ($tool -ne 'htmlmd' -and $baseline.ContainsKey($doc) -and $baseline[$doc] -gt 0) {
                $rel = [string]::Format($inv, ' ({0:F1}x)',
                                        ($rows[$tool][$doc].mean / $baseline[$doc]))
            }
            [string]::Format($inv, '{0:F0} +/-{1:F0} ms{2}', $m, $s, $rel)
        } else { '-' }
    }
    "| $tool | " + ($cells -join ' | ') + " |"
}

Write-Host ""
Write-Host "Raw JSON in $Results" -ForegroundColor DarkGray
Write-Host "Windows numbers are NOT comparable to the Linux numbers in docs/BENCHMARKS.md;" -ForegroundColor Yellow
Write-Host "publish them under their own machine heading." -ForegroundColor Yellow
