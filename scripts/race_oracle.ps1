# GPL-3.0 race oracle — quarantined under C:\Programming\umalator-ref
# This script only shells out; it does not import or redistribute GPL sources.
#
# Usage:
#   .\scripts\race_oracle.ps1 -File path\to\request.json
#   Get-Content request.json | .\scripts\race_oracle.ps1
#   .\scripts\race_oracle.ps1 -Verify

param(
    [string]$File,
    [switch]$Verify,
    [string]$OracleRoot = "C:\Programming\umalator-ref\oracle"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $OracleRoot)) {
    Write-Error "Oracle root missing: $OracleRoot (expected R8.0 quarantine checkout)"
}

Push-Location $OracleRoot
try {
    if ($Verify) {
        npx --yes ts-node --project tsconfig.json verify_fixtures.ts
        exit $LASTEXITCODE
    }

    if ($File) {
        npx --yes ts-node --project tsconfig.json race_oracle.ts --file (Resolve-Path $File)
        exit $LASTEXITCODE
    }

    $stdin = [Console]::In.ReadToEnd()
    if (-not $stdin.Trim()) {
        Write-Error "Pass -File, -Verify, or JSON on stdin"
    }
    $stdin | npx --yes ts-node --project tsconfig.json race_oracle.ts
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
