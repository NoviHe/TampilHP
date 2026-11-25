$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$FetchScript = Join-Path $PSScriptRoot "fetch-scrcpy.ps1"
if (Test-Path $FetchScript) {
    & $FetchScript
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host "[1/4] Building frontend..."
npm run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[2/4] Building Tauri release binary (no installer)..."
npx tauri build --no-bundle
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$ReleaseDir = Join-Path $Root "src-tauri\target\release"
$ExePath = Join-Path $ReleaseDir "tampilhp.exe"
$ResSrc = Join-Path $ReleaseDir "resources\scrcpy"
if (!(Test-Path $ResSrc)) {
    $ResSrc = Join-Path $Root "src-tauri\resources\scrcpy"
}

if (!(Test-Path $ExePath)) { throw "tampilhp.exe tidak ditemukan di $ExePath" }
if (!(Test-Path $ResSrc)) { throw "Folder resources scrcpy tidak ditemukan di $ResSrc" }

Write-Host "[3/4] Assembling TampilHP-Portable..."
$OutBase = Join-Path $Root "dist-portable"
$PortableDir = Join-Path $OutBase "TampilHP-Portable"
$ZipPath = Join-Path $OutBase "TampilHP-Portable-win64.zip"

if (Test-Path $PortableDir) { Remove-Item $PortableDir -Recurse -Force }
if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }

New-Item -ItemType Directory -Force -Path (Join-Path $PortableDir "resources\scrcpy") | Out-Null
Copy-Item $ExePath -Destination (Join-Path $PortableDir "tampilhp.exe") -Force
Copy-Item "$ResSrc\*" -Destination (Join-Path $PortableDir "resources\scrcpy") -Recurse -Force
Set-Content -Path (Join-Path $PortableDir "portable.flag") -Value "TampilHP True Portable Mode" -Encoding UTF8

$Readme = @"
TampilHP Portable (Windows x64)
================================
1. Jalankan tampilhp.exe langsung tanpa install.
2. Semua pengaturan (config.json) dan screenshot disimpan di folder ini.
3. Wireless 1-Klik: colok USB -> klik '1-Klik Wi-Fi' -> cabut kabel setelah notif muncul.
4. Tanpa kabel (Android 11+): buka Tab QR -> Scan dari Wireless Debugging di HP.
5. OBS / TikTok Studio: pakai Window Capture. Jika hitam, ganti Render Driver ke OpenGL atau Software.
"@
Set-Content -Path (Join-Path $PortableDir "README-PORTABLE.txt") -Value $Readme -Encoding UTF8

Write-Host "[4/4] Creating ZIP archive..."
Compress-Archive -Path $PortableDir -DestinationPath $ZipPath -CompressionLevel Optimal

$ZipSizeMB = [math]::Round((Get-Item $ZipPath).Length / 1MB, 2)
Write-Host "DONE! Portable folder: $PortableDir"
Write-Host "DONE! Portable ZIP   : $ZipPath ($ZipSizeMB MB)"
