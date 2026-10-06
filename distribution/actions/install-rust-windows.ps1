$ErrorActionPreference = 'Stop'

$version = '1.95.0'
$temporary = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { Join-Path $PSScriptRoot '../../.heap/cache/build/bootstrap' }
New-Item -ItemType Directory -Force -Path $temporary | Out-Null
$env:CARGO_HOME = Join-Path $temporary 'kero-cargo-home'
$env:RUSTUP_HOME = Join-Path $temporary 'kero-rustup-home'
$cargoBin = Join-Path $env:CARGO_HOME 'bin'
$env:PATH = "$cargoBin;$env:PATH"
if ($env:GITHUB_ENV) {
    Add-Content -LiteralPath $env:GITHUB_ENV -Value "CARGO_HOME=$env:CARGO_HOME"
    Add-Content -LiteralPath $env:GITHUB_ENV -Value "RUSTUP_HOME=$env:RUSTUP_HOME"
}
if ($env:GITHUB_PATH) { Add-Content -LiteralPath $env:GITHUB_PATH -Value $cargoBin }

if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) {
    $architecture = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'aarch64' } else { 'x86_64' }
    $hostTarget = "$architecture-pc-windows-msvc"
    $url = "https://static.rust-lang.org/rustup/dist/$hostTarget/rustup-init.exe"
    $installer = Join-Path $temporary 'rustup-init.exe'
    $checksumFile = Join-Path $temporary 'rustup-init.exe.sha256'
    Invoke-WebRequest -Uri $url -OutFile $installer
    Invoke-WebRequest -Uri "$url.sha256" -OutFile $checksumFile
    $expected = ((Get-Content -LiteralPath $checksumFile -Raw).Trim() -split '\s+')[0]
    $actual = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
    if ($expected -notmatch '^[a-fA-F0-9]{64}$' -or $actual -ne $expected) {
        throw 'The downloaded rustup installer failed its SHA-256 check.'
    }
    & $installer -y --no-modify-path --profile minimal
    if ($LASTEXITCODE -ne 0) { throw 'rustup installation failed.' }
}

& rustup.exe toolchain install $version --profile minimal
if ($LASTEXITCODE -ne 0) { throw "Rust $version installation failed." }
& rustup.exe default $version
if ($LASTEXITCODE -ne 0) { throw "Rust $version selection failed." }
foreach ($target in ($env:KERO_RUST_TARGETS -split ',' | ForEach-Object Trim | Where-Object { $_ })) {
    & rustup.exe target add --toolchain $version $target
    if ($LASTEXITCODE -ne 0) { throw "Rust target $target installation failed." }
}
foreach ($component in ($env:KERO_RUST_COMPONENTS -split ',' | ForEach-Object Trim | Where-Object { $_ })) {
    & rustup.exe component add --toolchain $version $component
    if ($LASTEXITCODE -ne 0) { throw "Rust component $component installation failed." }
}
& rustc.exe --version
