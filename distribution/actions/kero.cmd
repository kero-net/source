@echo off
setlocal EnableExtensions EnableDelayedExpansion
set "KERO_ROOT=%~dp0..\.."
for %%I in ("%KERO_ROOT%") do set "KERO_ROOT=%%~fI"
set "KERO_STAGE=%~1"
if "%KERO_STAGE%"=="" set "KERO_STAGE=build"
set "KERO_TARGET=%~2"
if "%KERO_TARGET%"=="" set "KERO_TARGET=current"

for /f %%I in ('powershell.exe -NoProfile -NonInteractive -Command "(Get-CimInstance -ClassName Win32_Processor).Architecture"') do set "KERO_CPU=%%I"
if "%KERO_TARGET%"=="current" (
  if "%KERO_CPU%"=="12" (set "KERO_TARGET=windows-arm64") else (set "KERO_TARGET=windows-x64")
)

if /I "%KERO_STAGE%"=="portable" goto :run
if /I "%KERO_STAGE%"=="propose" goto :run

if "%KERO_TARGET%"=="windows-arm64" (
  for /f "delims=" %%I in ('powershell.exe -NoProfile -NonInteractive -Command "$kits=@(Get-ChildItem -Path 'C:\Qt\*\msvc2022_arm64' -Directory -ErrorAction SilentlyContinue); if($kits.Count){$kits[$kits.Count-1].FullName}"') do set "KERO_QT_PREFIX=%%I"
  if not defined KERO_QT_PREFIX (
    echo KERO requires a Qt Windows ARM64 kit under C:\Qt. Install Qt msvc2022_arm64. 1>&2
    exit /b 1
  )
  for /f "delims=" %%I in ('powershell.exe -NoProfile -NonInteractive -Command "$roots=@('C:\Program Files\Microsoft Visual Studio\2022','C:\Program Files (x86)\Microsoft Visual Studio\2022'); $tool=Get-ChildItem $roots -Directory -ErrorAction SilentlyContinue | ForEach-Object { Get-ChildItem (Join-Path $_.FullName 'VC\Tools\MSVC') -Directory -ErrorAction SilentlyContinue } | Where-Object { Test-Path (Join-Path $_.FullName 'bin\Hostarm64\arm64\cl.exe') } | Sort-Object FullName | Select-Object -Last 1; if($tool){$tool.FullName}"') do set "KERO_MSVC_ROOT=%%I"
  if not defined KERO_MSVC_ROOT (
    echo KERO requires Visual Studio 2022 with the C++ ARM64 target component. 1>&2
    exit /b 1
  )
  for /f "delims=" %%I in ('powershell.exe -NoProfile -NonInteractive -Command "$kit=Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\Include' -Directory -ErrorAction SilentlyContinue | Where-Object { Test-Path (Join-Path $_.FullName 'um\windows.h') } | Sort-Object Name | Select-Object -Last 1; if($kit){$kit.Name}"') do set "KERO_WINDOWS_SDK_VERSION=%%I"
  if not defined KERO_WINDOWS_SDK_VERSION (
    echo KERO requires a Windows 10 or later SDK with ARM64 libraries. 1>&2
    exit /b 1
  )
  set "KERO_WINDOWS_SDK=C:\Program Files (x86)\Windows Kits\10"
  set "KERO_MSVC_LINKER=!KERO_MSVC_ROOT!\bin\Hostarm64\arm64\link.exe"
  set "KERO_MSVC_ARCHIVER=!KERO_MSVC_ROOT!\bin\Hostarm64\arm64\lib.exe"
  set "VCToolsInstallDir=!KERO_MSVC_ROOT!\"
  set "VCINSTALLDIR=!KERO_MSVC_ROOT!\..\..\"
  set "WindowsSdkDir=!KERO_WINDOWS_SDK!\"
  set "WindowsSDKVersion=!KERO_WINDOWS_SDK_VERSION!\"
  set "UniversalCRTSdkDir=!KERO_WINDOWS_SDK!\"
  set "UCRTVersion=!KERO_WINDOWS_SDK_VERSION!\"
  set "PATH=!KERO_MSVC_ROOT!\bin\Hostarm64\arm64;!KERO_WINDOWS_SDK!\bin\!KERO_WINDOWS_SDK_VERSION!\arm64;!PATH!"
  set "INCLUDE=!KERO_MSVC_ROOT!\include;!KERO_WINDOWS_SDK!\Include\!KERO_WINDOWS_SDK_VERSION!\ucrt;!KERO_WINDOWS_SDK!\Include\!KERO_WINDOWS_SDK_VERSION!\shared;!KERO_WINDOWS_SDK!\Include\!KERO_WINDOWS_SDK_VERSION!\um;!KERO_WINDOWS_SDK!\Include\!KERO_WINDOWS_SDK_VERSION!\winrt;!KERO_WINDOWS_SDK!\Include\!KERO_WINDOWS_SDK_VERSION!\cppwinrt"
  set "LIB=!KERO_MSVC_ROOT!\lib\arm64;!KERO_WINDOWS_SDK!\Lib\!KERO_WINDOWS_SDK_VERSION!\ucrt\arm64;!KERO_WINDOWS_SDK!\Lib\!KERO_WINDOWS_SDK_VERSION!\um\arm64"
  pushd "%KERO_ROOT%"
  where cl.exe >nul 2>nul || (
    echo KERO requires the Visual Studio 17.14 C++ ARM64 target component: Microsoft.VisualStudio.Component.VC.14.44.17.14.ARM64. 1>&2
    popd
    exit /b 1
  )
  if not exist "!KERO_MSVC_LINKER!" (
    echo KERO requires the Visual Studio ARM64 linker. 1>&2
    popd
    exit /b 1
  )
  if not exist "!KERO_MSVC_ARCHIVER!" (
    echo KERO requires the Visual Studio ARM64 librarian. 1>&2
    popd
    exit /b 1
  )
  lua distribution\scripts\local-run.lua "%KERO_STAGE%" "%KERO_TARGET%" %3 %4 %5
  set "KERO_EXIT=%ERRORLEVEL%"
  popd
  exit /b %KERO_EXIT%
)

if "%KERO_TARGET%"=="windows-x64" (
  for /f "delims=" %%I in ('powershell.exe -NoProfile -NonInteractive -Command "$kit=Get-ChildItem 'C:\Qt\*\llvm-mingw_64' -Directory -ErrorAction SilentlyContinue | Sort-Object FullName | Select-Object -Last 1; if($kit){$kit.FullName}"') do set "KERO_QT_PREFIX=%%I"
  for /f "delims=" %%I in ('powershell.exe -NoProfile -NonInteractive -Command "$compiler=Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\MartinStorsjo.LLVM-MinGW.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe') -Recurse -Filter x86_64-w64-mingw32-clang++.exe -ErrorAction SilentlyContinue | Select-Object -Last 1; if($compiler){$compiler.FullName}"') do set "KERO_CXX_COMPILER=%%I"
  if not defined KERO_QT_PREFIX (
    echo KERO requires a Qt llvm-mingw_64 kit under C:\Qt for Windows x64 cross-builds. 1>&2
    exit /b 1
  )
  if not defined KERO_CXX_COMPILER (
    echo KERO requires LLVM-MinGW x86_64 cross tools for Windows x64 builds. 1>&2
    exit /b 1
  )
  for /f "delims=" %%I in ('powershell.exe -NoProfile -NonInteractive -Command "$compiler=Get-Item $env:KERO_CXX_COMPILER; $compiler.Directory.Parent.FullName"') do set "KERO_LLVM_ROOT=%%I"
  set "KERO_WINDRES=!KERO_LLVM_ROOT!\bin\x86_64-w64-mingw32-windres.exe"
  set "KERO_RUNTIME_DIR=!KERO_LLVM_ROOT!\x86_64-w64-mingw32\bin"
  if not exist "!KERO_WINDRES!" (
    echo KERO requires the LLVM-MinGW x86_64 resource compiler. 1>&2
    exit /b 1
  )
  if not exist "!KERO_RUNTIME_DIR!\libc++.dll" (
    echo KERO requires the LLVM-MinGW x86_64 runtime DLL directory. 1>&2
    exit /b 1
  )
  pushd "%KERO_ROOT%"
  lua distribution\scripts\local-run.lua "%KERO_STAGE%" "%KERO_TARGET%" %3 %4 %5
  set "KERO_EXIT=%ERRORLEVEL%"
  popd
  exit /b %KERO_EXIT%
)

:run
pushd "%KERO_ROOT%"
lua distribution\scripts\local-run.lua "%KERO_STAGE%" "%KERO_TARGET%" %3 %4 %5
set "KERO_EXIT=%ERRORLEVEL%"
popd
exit /b %KERO_EXIT%
