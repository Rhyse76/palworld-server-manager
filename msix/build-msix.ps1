<#
  Builds an MSIX package for RhyseGaming Server Manager.

  TWO MODES:

  1) Local test build (default -Test): uses placeholder identity + a self-signed
     certificate so you can install and run the packaged app on THIS machine to
     confirm everything works inside the MSIX container.

        powershell -ExecutionPolicy Bypass -File build-msix.ps1 -Test

  2) Store build: pass the three values from Partner Center. Do NOT sign it —
     upload the unsigned .msix to Partner Center and Microsoft signs it.

        powershell -ExecutionPolicy Bypass -File build-msix.ps1 `
          -PackageName "12345RhyseGaming.ServerManager" `
          -PublisherId "CN=ABC12345-6789-...." `
          -PublisherDisplayName "Rhyse Gaming" `
          -AppDisplayName "RhyseGaming Server Manager"

  Requires: the release build to exist
  (src-tauri\target\release\RhyseGamingServerManager.exe -> run `npm run tauri build` first)
  and the Windows SDK (makeappx.exe / signtool.exe).
#>
param(
  [string]$PackageName          = "RhyseGaming.ServerManager.Test",
  [string]$PublisherId          = "CN=RhyseGamingTest",
  [string]$PublisherDisplayName = "Rhyse Gaming",
  [string]$AppDisplayName       = "RhyseGaming Server Manager",
  [string]$Version              = "0.4.10.0",
  [switch]$Test
)

$ErrorActionPreference = "Stop"
$here    = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe     = Join-Path $here "..\src-tauri\target\release\RhyseGamingServerManager.exe"
$staging = Join-Path $here "staging"
$outDir  = Join-Path $here "out"
$msix    = Join-Path $outDir "RhyseGamingServerManager.msix"

if (-not (Test-Path $exe)) { throw "Release exe not found at $exe. Run 'npm run tauri build' first." }

# --- locate Windows SDK tools ---
function Find-SdkTool($name) {
  $base = "C:\Program Files (x86)\Windows Kits\10\bin"
  Get-ChildItem -Path $base -Recurse -Filter $name -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "\\x64\\" } |
    Sort-Object FullName | Select-Object -Last 1 -ExpandProperty FullName
}
$makeappx = Find-SdkTool "makeappx.exe"
$signtool = Find-SdkTool "signtool.exe"
if (-not $makeappx) { throw "makeappx.exe not found (install the Windows SDK)." }

# --- stage the package layout ---
if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
New-Item -ItemType Directory -Force -Path $staging, $outDir | Out-Null
Copy-Item $exe (Join-Path $staging "RhyseGamingServerManager.exe")
Copy-Item (Join-Path $here "assets") (Join-Path $staging "assets") -Recurse

# --- write the manifest with real values ---
$manifest = Get-Content (Join-Path $here "AppxManifest.xml") -Raw
$manifest = $manifest.Replace("__PACKAGE_IDENTITY_NAME__", $PackageName)
$manifest = $manifest.Replace("__PUBLISHER_ID__", $PublisherId)
$manifest = $manifest.Replace("__PUBLISHER_DISPLAY_NAME__", $PublisherDisplayName)
$manifest = $manifest.Replace("__APP_DISPLAY_NAME__", $AppDisplayName)
# Case-sensitive (-creplace) so we don't touch the xml declaration's lowercase version=.
# The negative lookbehind (?<![A-Za-z]) ensures we only match a standalone Version="..."
# (Identity's), NOT MinVersion="..."/MaxVersionTested="..." where a letter precedes "Version".
$manifest = $manifest -creplace '(?<![A-Za-z])Version="[0-9.]+"', "Version=`"$Version`""
# Write UTF-8 WITHOUT BOM (a BOM before <?xml ?> breaks makeappx manifest validation).
[System.IO.File]::WriteAllText((Join-Path $staging "AppxManifest.xml"), $manifest, (New-Object System.Text.UTF8Encoding($false)))

# --- pack ---
Write-Host "Packing MSIX..." -ForegroundColor Cyan
& $makeappx pack /d $staging /p $msix /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed." }
Write-Host "Built: $msix" -ForegroundColor Green

if ($Test) {
  if (-not $signtool) { throw "signtool.exe not found." }
  Write-Host "Creating self-signed test certificate (subject must match Publisher: $PublisherId)..." -ForegroundColor Cyan
  $cert = New-SelfSignedCertificate -Type Custom -Subject $PublisherId `
            -KeyUsage DigitalSignature -FriendlyName "RhyseGaming Server Manager Test" `
            -CertStoreLocation "Cert:\CurrentUser\My" `
            -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
  $pfxPath = Join-Path $outDir "test-cert.pfx"
  $pwd = ConvertTo-SecureString -String "test" -Force -AsPlainText
  Export-PfxCertificate -Cert "Cert:\CurrentUser\My\$($cert.Thumbprint)" -FilePath $pfxPath -Password $pwd | Out-Null
  & $signtool sign /fd SHA256 /a /f $pfxPath /p "test" $msix
  if ($LASTEXITCODE -ne 0) { throw "signtool failed." }
  Write-Host ""
  Write-Host "Signed for local testing. To install & test (run as Admin, once):" -ForegroundColor Green
  Write-Host "  1. Trust the self-signed test cert (password is 'test'):" -ForegroundColor Yellow
  Write-Host "     `$pw = ConvertTo-SecureString -String 'test' -Force -AsPlainText" -ForegroundColor Gray
  Write-Host "     Import-PfxCertificate -FilePath '$pfxPath' -CertStoreLocation Cert:\LocalMachine\TrustedPeople -Password `$pw" -ForegroundColor Gray
  Write-Host "     Import-PfxCertificate -FilePath '$pfxPath' -CertStoreLocation Cert:\LocalMachine\Root -Password `$pw" -ForegroundColor Gray
  Write-Host "     (Root import clears the 0x800B0109 'root not trusted' error for a self-signed cert.)" -ForegroundColor Gray
  Write-Host "  2. Install:  Add-AppxPackage '$msix'   (then launch from the Start menu)." -ForegroundColor Yellow
  Write-Host "  Verify: install server, start/stop, backups all work inside the MSIX." -ForegroundColor Yellow
} else {
  Write-Host ""
  Write-Host "Store build ready (UNSIGNED - correct for Partner Center):" -ForegroundColor Green
  Write-Host "  Upload $msix in your app's submission. Microsoft signs it." -ForegroundColor Yellow
  Write-Host "  Make sure PackageName/PublisherId/PublisherDisplayName match your Partner Center product identity." -ForegroundColor Yellow
}
