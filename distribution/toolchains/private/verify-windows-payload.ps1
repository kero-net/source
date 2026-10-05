param([Parameter(Mandatory = $true)][string]$Installer)

$ErrorActionPreference = 'Stop'
$Installer = $Installer.Trim()
$process = Start-Process -FilePath $Installer -ArgumentList '--verify-payload' -Wait -PassThru -WindowStyle Hidden
exit $process.ExitCode
