<#!
.SYNOPSIS
Registers REA Idm Mangaer's Native Messaging host for installed browsers.
.DESCRIPTION
The browser extension itself is loaded through the browser's extension management page.
This script only creates the local, per-user manifest and registry entries required for
secure native messaging. It never elevates, modifies browser policies, or sends data off-device.
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$NativeHostPath,
  [Parameter(Mandatory = $true)][ValidatePattern('^[a-p]{32}$')][string]$ChromiumExtensionId,
  [string]$ExtensionOutput = (Join-Path $PSScriptRoot '..\extension\build'),
  [switch]$FirefoxOnly
)
$ErrorActionPreference = 'Stop'
$resolvedHost = (Resolve-Path -LiteralPath $NativeHostPath).Path
if (-not (Test-Path -LiteralPath $ExtensionOutput -PathType Container)) { throw "Extension output not found: $ExtensionOutput. Run pnpm extension:build first." }
$dataDir = Join-Path $env:LOCALAPPDATA 'com.rea.idm-mangaer'
New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
$tokenPath = Join-Path $dataDir 'bridge.token'
if (-not (Test-Path -LiteralPath $tokenPath)) { [guid]::NewGuid().ToString() | Set-Content -LiteralPath $tokenPath -NoNewline -Encoding ascii }
$token = (Get-Content -LiteralPath $tokenPath -Raw).Trim()
$configPath = Join-Path $ExtensionOutput 'config.js'
"globalThis.REA_BRIDGE_TOKEN = '$token';" | Set-Content -LiteralPath $configPath -NoNewline -Encoding utf8
$manifestPath = Join-Path $dataDir 'com.rea.idm_mangaer.json'
$manifest = @{ name = 'com.rea.idm_mangaer'; description = 'REA Idm Mangaer secure local bridge'; path = $resolvedHost; type = 'stdio'; allowed_origins = @("chrome-extension://$ChromiumExtensionId/") }
$manifest | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $manifestPath -Encoding utf8
if (-not $FirefoxOnly) {
  foreach ($key in @('HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\Vivaldi\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\Opera Software\Opera Stable\NativeMessagingHosts\com.rea.idm_mangaer')) { New-Item -Path $key -Force | Out-Null; Set-ItemProperty -Path $key -Name '(Default)' -Value $manifestPath }
}
$firefoxManifest = @{ name = 'com.rea.idm_mangaer'; description = 'REA Idm Mangaer secure local bridge'; path = $resolvedHost; type = 'stdio'; allowed_extensions = @('rea-idm-mangaer@example.invalid') }
$firefoxPath = Join-Path $dataDir 'com.rea.idm_mangaer.firefox.json'
$firefoxManifest | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $firefoxPath -Encoding utf8
$firefoxKey = 'HKCU:\Software\Mozilla\NativeMessagingHosts\com.rea.idm_mangaer'
New-Item -Path $firefoxKey -Force | Out-Null; Set-ItemProperty -Path $firefoxKey -Name '(Default)' -Value $firefoxPath
Write-Host 'Native bridge registered for the current Windows user.'
Write-Host "Load '$ExtensionOutput' as an unpacked extension in Chrome, Edge, Brave, Opera, or Vivaldi."
Write-Host "For Firefox, package/load manifest.firefox.json alongside the extension and use about:debugging for temporary development installation."
