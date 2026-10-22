<# Removes only REA Idm Mangaer's per-user Native Messaging registrations. #>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
foreach ($key in @('HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\Vivaldi\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\Opera Software\Opera Stable\NativeMessagingHosts\com.rea.idm_mangaer','HKCU:\Software\Mozilla\NativeMessagingHosts\com.rea.idm_mangaer')) { if (Test-Path -LiteralPath $key) { Remove-Item -LiteralPath $key -Force } }
$dataDir = Join-Path $env:LOCALAPPDATA 'com.rea.idm-mangaer'
Remove-Item -LiteralPath (Join-Path $dataDir 'com.rea.idm_mangaer.json') -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $dataDir 'com.rea.idm_mangaer.firefox.json') -Force -ErrorAction SilentlyContinue
Write-Host 'REA Idm Mangaer Native Messaging registrations removed. The extension can now be removed in its browser.'
