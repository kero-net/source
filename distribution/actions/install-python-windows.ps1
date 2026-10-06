$ErrorActionPreference = 'Stop'

$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$manifest = Join-Path $root 'distribution/tools/python-windows.toml'
$versionMatch = Select-String -LiteralPath $manifest -Pattern '^version = "([^"]+)"$' | Select-Object -First 1
if (-not $versionMatch) { throw 'Windows Python version is missing from distribution/tools/python-windows.toml' }
$version = $versionMatch.Matches[0].Groups[1].Value

$architecture = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    'Arm64' { 'windows-arm64' }
    'X64' { 'windows-x64' }
    default { throw "Unsupported Windows architecture: $([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)" }
}

$section = $false
$archive = $null
$url = $null
$sha256 = $null
foreach ($line in Get-Content -LiteralPath $manifest) {
    if ($line -match '^\[([^]]+)\]$') { $section = ($Matches[1] -eq $architecture); continue }
    if (-not $section) { continue }
    if ($line -match '^archive = "([^"]+)"$') { $archive = $Matches[1]; continue }
    if ($line -match '^url = "([^"]+)"$') { $url = $Matches[1]; continue }
    if ($line -match '^sha256 = "([a-f0-9]{64})"$') { $sha256 = $Matches[1]; continue }
}
if (-not $archive -or -not $url -or -not $sha256) { throw "Incomplete Windows Python manifest for $architecture" }

$cacheRoot = if ($env:RUNNER_TEMP) { Join-Path $env:RUNNER_TEMP 'kero-python' } else { Join-Path $root '.heap/cache/toolchains/python' }
$runtime = Join-Path $cacheRoot "$version-$architecture"
$python = Join-Path $runtime 'python.exe'

if (-not (Test-Path -LiteralPath $python)) {
    if (Test-Path -LiteralPath $runtime) { Remove-Item -LiteralPath $runtime -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $runtime | Out-Null
    New-Item -ItemType Directory -Force -Path $cacheRoot | Out-Null
    $archivePath = Join-Path $cacheRoot $archive
    Invoke-WebRequest -Uri $url -OutFile $archivePath
    $actual = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $sha256) { throw "Windows Python checksum mismatch: expected $sha256, received $actual" }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $runtime -Force
}

if (-not (Test-Path -LiteralPath $python)) { throw "Windows Python archive did not contain python.exe: $runtime" }
& $python --version
if ($LASTEXITCODE -ne 0) { throw 'Windows Python runtime failed to execute.' }
& $python -m pip --version
if ($LASTEXITCODE -ne 0) { throw 'Windows Python runtime does not include pip.' }

if ($env:GITHUB_PATH) {
    Add-Content -LiteralPath $env:GITHUB_PATH -Value $runtime
    $scripts = Join-Path $runtime 'Scripts'
    if (Test-Path -LiteralPath $scripts) { Add-Content -LiteralPath $env:GITHUB_PATH -Value $scripts }
}
Write-Host "Using disposable Python $version from $runtime"
