param([switch]$AfterReboot)

$ErrorActionPreference = 'Stop'
$taskName = 'Vesper Virtual Audio Test Install'
$workRoot = Join-Path $env:ProgramData 'Vesper\VirtualAudio'
$driverHardwareId = 'ROOT\VesperVirtualAudio'
$devcon = 'C:\Program Files (x86)\Windows Kits\10\Tools\10.0.26100.0\x64\devcon.exe'
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())

if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script from an elevated Administrator PowerShell session.'
}

if (-not $AfterReboot) {
    $packageRoot = Join-Path $PSScriptRoot 'vendor\simpleaudiosample\Package\x64\Release\package'
    $certificate = Join-Path $PSScriptRoot 'vendor\simpleaudiosample\Package\x64\Release\package.cer'
    if (-not (Test-Path $packageRoot) -or -not (Test-Path $certificate) -or -not (Test-Path $devcon)) {
        throw 'Build the test-signed package before installing it.'
    }

    New-Item -ItemType Directory -Path $workRoot -Force | Out-Null
    Copy-Item (Join-Path $packageRoot '*') $workRoot -Force
    Copy-Item $certificate (Join-Path $workRoot 'VesperVirtualAudio.cer') -Force
    Copy-Item $PSCommandPath (Join-Path $workRoot 'Install-VesperVirtualAudio-Test.ps1') -Force

    Import-Certificate -FilePath (Join-Path $workRoot 'VesperVirtualAudio.cer') -CertStoreLocation Cert:\LocalMachine\Root | Out-Null
    Import-Certificate -FilePath (Join-Path $workRoot 'VesperVirtualAudio.cer') -CertStoreLocation Cert:\LocalMachine\TrustedPublisher | Out-Null
    & bcdedit.exe /set testsigning on
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to enable Windows test signing.'
    }

    $taskScript = Join-Path $workRoot 'Install-VesperVirtualAudio-Test.ps1'
    $taskCommand = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$taskScript`" -AfterReboot"
    & schtasks.exe /Create /TN $taskName /TR $taskCommand /SC ONSTART /RU SYSTEM /RL HIGHEST /F
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to schedule the post-reboot driver installation.'
    }
    exit 3010
}

$logPath = Join-Path $workRoot 'install.log'
$infPath = Join-Path $workRoot 'VesperVirtualAudio.inf'
$existingDevices = @(Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
    Where-Object { $_.Class -eq 'MEDIA' -and $_.FriendlyName -eq 'Vesper Virtual Audio Device' })
& pnputil.exe /add-driver $infPath /install *>&1 | Tee-Object -FilePath $logPath
$result = $LASTEXITCODE

if ($existingDevices.Count -eq 0) {
    & $devcon install $infPath $driverHardwareId *>&1 | Tee-Object -FilePath $logPath -Append
    $result = $LASTEXITCODE
}
& schtasks.exe /Delete /TN $taskName /F | Out-Null
exit $result
