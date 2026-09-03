# Parity harness: Kotlin fixture export (legacy oracle) -> Rust parity tests -> matrix report.
param(
    [switch]$Export,
    [switch]$SkipExport
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$UmaSim = Join-Path $RepoRoot "legacy\uma-sim-kotlin"
$Core = Join-Path $RepoRoot "uma-sim-core"
$Fixtures = Join-Path $Core "tests\fixtures"

Write-Host "=== Uma Sim Parity Harness ===" -ForegroundColor Cyan

if ($Export -or -not $SkipExport) {
    Write-Host "`n[1/3] Export Kotlin parity fixtures (legacy oracle)..." -ForegroundColor Yellow
    if (-not (Test-Path $UmaSim)) {
        throw "Legacy Kotlin oracle missing at $UmaSim"
    }
    Push-Location $UmaSim
    try {
        & .\gradlew.bat :sim-engine:jvmTest "-DexportParity=true" "--tests" "ParityFixtureExportTest" 2>&1 | Out-Host
        if ($LASTEXITCODE -ne 0) { throw "Kotlin export failed (exit $LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
} else {
    Write-Host "`n[1/3] Skipping Kotlin export (-SkipExport)" -ForegroundColor DarkGray
}

Write-Host "`n[2/3] Rust parity tests..." -ForegroundColor Yellow
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Push-Location $Core
try {
    & cargo test --test parity 2>&1 | Out-Host
    $testExit = $LASTEXITCODE
} finally {
    Pop-Location
}

Write-Host "`n[3/3] Parity matrix (from fixtures + spot checks)..." -ForegroundColor Yellow
Write-Host ("{0,4} {1,-14} {2}" -f "seed", "scenario", "fixtures")
foreach ($seed in @(1, 42, 7)) {
    foreach ($scenario in @("ura", "grand_concert", "unity", "trackblazer")) {
        $rng = Join-Path $Fixtures "rng_trace_${seed}_${scenario}.json"
        $turn = Join-Path $Fixtures "turn_trace_${seed}_${scenario}.json"
        $ok = (Test-Path $rng) -and (Test-Path $turn)
        Write-Host ("{0,4} {1,-14} {2}" -f $seed, $scenario, $(if ($ok) { "OK" } else { "MISSING" }))
    }
}

if ($testExit -ne 0) {
    Write-Host "`nParity tests FAILED (exit $testExit)" -ForegroundColor Red
    exit $testExit
}

Write-Host "`nParity harness complete." -ForegroundColor Green
exit 0
