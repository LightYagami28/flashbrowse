# One-script installer for flashbrowse (Windows).
#   irm https://raw.githubusercontent.com/nhrbuilds/flashbrowse/main/install.ps1 | iex
param([string]$Version = "")
$ErrorActionPreference = "Stop"
$Repo = "nhrbuilds/flashbrowse"
$Asset = "flashbrowse-windows-x86_64"
$Tag = if ($Version -notmatch "^v") { "v$Version" } else { $Version }
if (-not $Version) {
  $Tag = (curl.exe -s -o NUL -w "%{url_effective}" "https://github.com/$Repo/releases/latest") -replace ".*releases/tag/",""
}
$Url = "https://github.com/$Repo/releases/download/$Tag/$Asset.zip"
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { "$HOME\.local\bin" }
$Tmp = Join-Path $env:TEMP "flashbrowse-$Tag"
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
Write-Host "-> Downloading $Asset ($Tag) ..."
curl.exe -fsSL $Url -o "$Tmp\$Asset.zip"
Expand-Archive "$Tmp\$Asset.zip" -DestinationPath $Tmp -Force
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item "$Tmp\flashbrowse.exe" "$InstallDir\flashbrowse.exe" -Force
# Clear the "downloaded from the internet" mark so SmartScreen does not warn
# (code signing is optional - this removes the friction for installer users).
try { Unblock-File "$InstallDir\flashbrowse.exe" } catch {}
Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
Write-Host "Installed: $InstallDir\flashbrowse.exe ($Tag)"
Write-Host "Add to PATH if needed:  `$env:PATH += `"$InstallDir`""
Write-Host "Then:"
Write-Host "  flashbrowse.exe setup"
Write-Host "  flashbrowse.exe open https://example.com"
Write-Host "  flashbrowse.exe mcp"
