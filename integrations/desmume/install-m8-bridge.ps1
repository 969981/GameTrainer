param(
    [Parameter(Mandatory=$true)][string]$DeSmuMESource,
    [switch]$AllowDifferentUpstreamCommit
)
$ErrorActionPreference = "Stop"
$repo = (Resolve-Path $DeSmuMESource).Path
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$pin = 'b549f71b83287f265b6d74b9e77ef5d98a617190'
$actual = (& git -C $repo rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw "The DeSmuME path must be a Git checkout" }
if ($actual -ne $pin -and -not $AllowDifferentUpstreamCommit) {
    throw "DeSmuME commit mismatch: $actual. M8 patch tested against $pin. Inspect differences before using -AllowDifferentUpstreamCommit."
}
$patch = Join-Path $root 'integrations\desmume\0001-m8-identity-bridge.patch'
$header = Join-Path $root 'integrations\desmume\gt_hgss_m8_bridge.h'
$destination = Join-Path $repo 'desmume\src\gt_hgss_m8_bridge.h'
if (Test-Path $destination) { throw "Header already exists: $destination. Do not silently overwrite." }
& git -C $repo apply --check $patch
if ($LASTEXITCODE -ne 0) { throw "Patch does not apply cleanly. No changes made." }
& git -C $repo apply $patch
if ($LASTEXITCODE -ne 0) { throw "Patch apply failed; inspect Git worktree." }
Copy-Item -LiteralPath $header -Destination $destination -ErrorAction Stop
Write-Host "M8 source patch and header added in: $repo"
Write-Host "NEXT: Enable GDB_STUB and GT_HGSS_M8_BRIDGE for the Windows DeSmuME project,"
Write-Host "rebuild, then run with distinct --arm9gdb / --arm7gdb ports."
Write-Host "This script does not build DeSmuME, start the pipe or authorize game writes."
