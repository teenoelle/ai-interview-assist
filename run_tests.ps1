# Run the full test suite for AI Interview Assistant.
# Usage:
#   .\run_tests.ps1            # unit tests only (no network, no API quota used)
#   .\run_tests.ps1 -Integration  # also run API integration tests (uses real keys + quota)

param(
    [switch]$Integration
)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$backend = Join-Path $root "backend"

Write-Host ""
Write-Host "=== AI Interview Assistant — Test Suite ===" -ForegroundColor Cyan
Write-Host ""

# ── Unit tests ────────────────────────────────────────────────────────────────
Write-Host "Running unit tests (all crates)..." -ForegroundColor Yellow
Push-Location $backend
try {
    cargo test --workspace --quiet 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "UNIT TESTS FAILED" -ForegroundColor Red
        exit 1
    }
} finally {
    Pop-Location
}
Write-Host "Unit tests PASSED" -ForegroundColor Green

# ── Integration tests (optional) ──────────────────────────────────────────────
if ($Integration) {
    Write-Host ""
    Write-Host "Running integration tests (real API calls)..." -ForegroundColor Yellow
    Write-Host "Note: these consume API quota. Failures may indicate rate limits, not bugs." -ForegroundColor Gray
    Write-Host ""

    Push-Location $backend
    try {
        cargo test -p server --test api_integration -- --ignored --nocapture 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-Host "INTEGRATION TESTS FAILED" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }
    Write-Host "Integration tests PASSED" -ForegroundColor Green
}

Write-Host ""
Write-Host "All tests passed." -ForegroundColor Cyan
