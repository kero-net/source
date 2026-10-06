$ErrorActionPreference = 'Stop'

$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$manifest = Join-Path $root 'distribution/tools/lune.toml'
$versionLine = Select-String -LiteralPath $manifest -Pattern '^version = "([^"]+)"$' | Select-Object -First 1
if (-not $versionLine) { throw 'Lune version is missing from distribution/tools/lune.toml' }
$version = $versionLine.Matches[0].Groups[1].Value

$architecture = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'aarch64' } else { 'x86_64' }
$key = "windows_$architecture"
$checksumLine = Select-String -LiteralPath $manifest -Pattern ('^' + [regex]::Escape($key) + ' = "([a-f0-9]{64})"$') | Select-Object -First 1
if (-not $checksumLine) { throw "Missing SHA-256 checksum for $key" }
$checksum = $checksumLine.Matches[0].Groups[1].Value.ToUpperInvariant()

$destination = if ($env:RUNNER_TEMP) { Join-Path $env:RUNNER_TEMP 'kero-lune' } else { Join-Path $root '.heap/cache/toolchains/lune' }
New-Item -ItemType Directory -Force -Path $destination | Out-Null
$executable = Join-Path $destination 'lune.exe'
if (Test-Path -LiteralPath $executable) {
    $existing = (& $executable --version 2>$null | Out-String)
    if ($LASTEXITCODE -eq 0 -and $existing -match [regex]::Escape($version)) {
        if ($env:GITHUB_PATH) { Add-Content -LiteralPath $env:GITHUB_PATH -Value $destination }
        exit 0
    }
}

$temporary = Join-Path ([IO.Path]::GetTempPath()) ('kero-lune-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $temporary | Out-Null
try {
    $archive = "lune-$version-windows-$architecture.zip"
    $archivePath = Join-Path $temporary $archive
    $url = "https://github.com/lune-org/lune/releases/download/v$version/$archive"
    Invoke-WebRequest -Uri $url -OutFile $archivePath
    $actual = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($actual -ne $checksum) { throw "Lune archive checksum mismatch for $archive" }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $temporary -Force
    Copy-Item -LiteralPath (Join-Path $temporary 'lune.exe') -Destination $executable -Force
}
finally {
    if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}

& $executable --version
if ($LASTEXITCODE -ne 0) { throw 'Lune installation failed.' }
if ($env:GITHUB_PATH) { Add-Content -LiteralPath $env:GITHUB_PATH -Value $destination }
