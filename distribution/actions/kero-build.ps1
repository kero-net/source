$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$heap = [IO.Path]::GetFullPath((Join-Path $workspace '.heap'))
if ($env:KERO_CLEAN -eq '1') {
    if ((Split-Path -Parent $heap) -ne $workspace -or (Split-Path -Leaf $heap) -ne '.heap') {
        throw 'Refusing to clean a path outside the source heap.'
    }
    if (Test-Path -LiteralPath $heap) { Remove-Item -LiteralPath $heap -Recurse -Force }
}
$logs = Join-Path $workspace '.heap/logs'
New-Item -ItemType Directory -Path $logs -Force | Out-Null
$transcribing = $false
Push-Location -LiteralPath $workspace
try {
    Start-Transcript -Path (Join-Path $logs 'windows-launcher.log') -Append | Out-Null
    $transcribing = $true
    $env:KERO_CARGO = 'cargo.exe'
    $env:ACTIONLINT_BIN = 'actionlint.exe'
    & lune.exe run .github/scripts/source-checks.luau local
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $wslWorkspace = (& wsl.exe --exec wslpath -u $workspace).Trim()
    if ($LASTEXITCODE -ne 0 -or -not $wslWorkspace) { throw 'Cannot translate the source path for WSL.' }
    $env:KERO_SOURCE_CHECKED = '1'
    $forwarded = @($env:WSLENV, 'KERO_SOURCE_CHECKED') | Where-Object { $_ }
    if ($env:KERO_CLEAN -eq '1') { $forwarded += 'KERO_CLEAN' }
    $env:WSLENV = $forwarded -join ':'
    Stop-Transcript | Out-Null
    $transcribing = $false
    & wsl.exe --exec /bin/bash --noprofile --norc "$wslWorkspace/distribution/actions/kero-build.sh"
    exit $LASTEXITCODE
}
finally {
    if ($transcribing) { Stop-Transcript | Out-Null }
    Pop-Location
}
