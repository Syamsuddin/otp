# SPDX-License-Identifier: GPL-3.0-only
param([string]$Destination = "$env:LOCALAPPDATA\Programs\ODIN-Turbo-Pascal\2.0.0-dev.2")
$ErrorActionPreference = 'Stop'
if (-not (Test-Path "$PSScriptRoot\otp.exe")) { throw 'Run this installer from a built Windows package.' }
if (-not (Test-Path "$PSScriptRoot\otp_rtl.lib")) { throw 'The MSVC runtime library is missing.' }
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
foreach ($Name in @('otp.exe','otpc.exe','otp_rtl.lib','docs','examples','assets','third-party-licenses','README.md','LICENSE','THIRD_PARTY.md','BUILD.json')) {
 Copy-Item -Recurse -Force -Path (Join-Path $PSScriptRoot $Name) -Destination $Destination
}
$Shell = New-Object -ComObject WScript.Shell
$StartMenu = [Environment]::GetFolderPath('Programs')
$Shortcut = $Shell.CreateShortcut((Join-Path $StartMenu 'ODIN Turbo Pascal 2.0.lnk'))
$Shortcut.TargetPath = Join-Path $Destination 'otp.exe'
$Shortcut.WorkingDirectory = [Environment]::GetFolderPath('MyDocuments')
$Shortcut.Save()
Write-Host "Installed to $Destination"
Write-Host 'Compilation needs LLVM clang and the Visual Studio x64 C++ Build Tools environment.'
Write-Host 'For this development build, start OTP from an x64 Native Tools command prompt.'
