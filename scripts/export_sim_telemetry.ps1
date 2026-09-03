# Export Android-shaped JSONL from uma-sim bot run (for calibrate_sim.py).
param(
    [long]$Seed = 42,
    [string]$Scenario = "ura",
    [string]$Policy = "bot",
    [string]$Output = ""
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Core = Join-Path $Root "uma-sim-core"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Push-Location $Core
try {
    $cargoArgs = @("run", "--bin", "uma-sim", "--", "export-telemetry", "--seed=$Seed", "--scenario=$Scenario", "--policy=$Policy")
    if ($Output) { $cargoArgs += "--output=$Output" }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "export-telemetry failed (exit $LASTEXITCODE)" }
} finally {
    Pop-Location
}
