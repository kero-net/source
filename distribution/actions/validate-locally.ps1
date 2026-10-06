$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$heap = [IO.Path]::GetFullPath((Join-Path $workspace '.heap'))

if ($env:KERO_CLEAN -eq '1') {
    if ((Split-Path -Parent $heap) -ne $workspace -or (Split-Path -Leaf $heap) -ne '.heap') {
        throw 'Refusing to clean a path outside the source heap.'
    }
    if (Test-Path -LiteralPath $heap) { Remove-Item -LiteralPath $heap -Recurse -Force }
}

$wslWorkspace = (& wsl.exe --exec wslpath -u $workspace).Trim()
if ($LASTEXITCODE -ne 0 -or -not $wslWorkspace) {
    throw 'A working WSL distribution is required for the local GitHub workflow gate.'
}

$previousClean = $env:KERO_CLEAN
$previousActValidated = $env:KERO_ACT_VALIDATED
$previousWslenv = $env:WSLENV
try {
    $env:KERO_CLEAN = '0'
    & wsl.exe --exec /bin/bash --noprofile --norc "$wslWorkspace/distribution/actions/validate-act.sh"
    if ($LASTEXITCODE -ne 0) { throw "GitHub workflow replay failed with exit code $LASTEXITCODE." }

    $env:KERO_ACT_VALIDATED = '1'
    $env:WSLENV = (@($env:WSLENV, 'KERO_ACT_VALIDATED') | Where-Object { $_ }) -join ':'
    & (Join-Path $PSScriptRoot 'kero-build.ps1')
    if ($LASTEXITCODE -ne 0) { throw "Native distribution build failed with exit code $LASTEXITCODE." }

    $checksums = Join-Path $heap 'release/artifacts/SHA256SUMS'
    $message = Join-Path $heap 'release/RELEASE-MESSAGE.md'
    if (-not (Test-Path -LiteralPath $checksums) -or -not (Test-Path -LiteralPath $message)) {
        throw 'Validation did not produce the required release manifest and checksums.'
    }
    if ((Get-Content -LiteralPath $checksums).Count -ne 4) {
        throw 'Validation did not produce all four distribution checksums.'
    }
    if (-not (Select-String -LiteralPath $message -Pattern 'Status: **complete**' -SimpleMatch -Quiet)) {
        throw 'The release message does not mark the build complete.'
    }
    $artifactRoot = [IO.Path]::GetFullPath((Join-Path $heap 'release/artifacts/distributions'))
    foreach ($line in (Get-Content -LiteralPath $checksums)) {
        if ($line -notmatch '^([a-fA-F0-9]{64})  distributions/([^/\\]+)$') {
            throw "Invalid release checksum entry: $line"
        }
        $artifact = [IO.Path]::GetFullPath((Join-Path $artifactRoot $Matches[2]))
        if ((Split-Path -Parent $artifact) -ne $artifactRoot -or -not (Test-Path -LiteralPath $artifact)) {
            throw "Missing or invalid release artifact: $artifact"
        }
        if ((Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash -ne $Matches[1]) {
            throw "Release artifact checksum mismatch: $artifact"
        }
    }
}
finally {
    $env:KERO_CLEAN = $previousClean
    $env:KERO_ACT_VALIDATED = $previousActValidated
    $env:WSLENV = $previousWslenv
}
