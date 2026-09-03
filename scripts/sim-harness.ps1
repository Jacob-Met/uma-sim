# Sim harness — golden seeds and bot parity (Rust primary; legacy Kotlin for oracle regen)
param(
    [ValidateSet("golden", "generate-golden", "perf", "parity", "replay-live", "calibrate", "all")]
    [string]$Mode = "all"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Core = Join-Path $Root "uma-sim-core"
$Legacy = Join-Path $Root "legacy\uma-sim-kotlin"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

function Run-CargoTest {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArgs)
    Push-Location $Core
    try {
        if ($CargoArgs -and $CargoArgs.Count -gt 0) {
            & cargo test @CargoArgs
        } else {
            & cargo test
        }
        if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
    } finally {
        Pop-Location
    }
}

function Run-LegacyGradleTest {
    param(
        [string]$Filter,
        [string[]]$ExtraArgs = @()
    )
    if (-not (Test-Path $Legacy)) { throw "Legacy Kotlin missing at $Legacy" }
    Push-Location $Legacy
    try {
        $gradleArgs = @(":sim-engine:jvmTest", "--no-daemon") + $ExtraArgs
        if ($Filter) { $gradleArgs += @("--tests", $Filter) }
        & .\gradlew.bat @gradleArgs
        if ($LASTEXITCODE -ne 0) { throw "Gradle tests failed" }
    } finally {
        Pop-Location
    }
}

switch ($Mode) {
    "golden" {
        Write-Host "=== Golden seed regression (Rust) ==="
        Run-CargoTest --test golden_seeds
    }
    "generate-golden" {
        Write-Host "=== Regenerate golden/summaries.json via legacy Kotlin (50x4) ==="
        Run-LegacyGradleTest "GenerateGoldenSummariesTest" @("-DgenerateGolden=true")
    }
    "perf" {
        Write-Host "=== Performance gate (Rust) ==="
        Run-CargoTest perf
    }
    "replay-live" {
        Write-Host "=== Live telemetry replay (Rust) ==="
        $jsonl = Get-ChildItem -Path (Join-Path $Root "runs") -Recurse -Filter "*.jsonl" -ErrorAction SilentlyContinue | Select-Object -First 1
        $replayOut = Join-Path $Legacy "sim-engine\src\commonTest\resources\telemetry_replay\converted_live.jsonl"
        if ($jsonl) {
            Write-Host "Converting $($jsonl.FullName) ..."
            python (Join-Path $Root "scripts\telemetry_replay.py") $jsonl.FullName -o $replayOut
        } else {
            Write-Host "No .jsonl in runs/ — using committed live_bot_sample.jsonl"
        }
        Run-CargoTest --test live_bot_telemetry
        Run-CargoTest --test live_telemetry_replay
    }
    "parity" {
        Write-Host "=== Bot / fixture parity (Rust) ==="
        Run-CargoTest --test parity
        Run-CargoTest --test telemetry_replay
        Run-CargoTest --test live_telemetry_replay
    }
    "calibrate" {
        Write-Host "=== Training gain calibration (stub) ==="
        python (Join-Path $Root "scripts\calibrate_sim.py") --stub
        if ($LASTEXITCODE -ne 0) { throw "Calibration stub failed" }
    }
    "all" {
        Write-Host "=== Full Rust uma-sim-core test suite ==="
        Run-CargoTest
    }
}

Write-Host "Harness OK ($Mode)"
