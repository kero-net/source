@echo off
setlocal
call "%~dp0distribution\actions\kero.cmd" %*
exit /b %ERRORLEVEL%
