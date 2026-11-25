$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$TargetDir = Join-Path $Root "src-tauri\resources\scrcpy"
$Version = "v5.0.1"
$ZipUrl = "https://github.com/Genymobile/scrcpy/releases/download/$Version/scrcpy-win64-$Version.zip"

if ((Test-Path (Join-Path $TargetDir "scrcpy.exe")) -and (Test-Path (Join-Path $TargetDir "adb.exe"))) {
    Write-Host "scrcpy $Version sudah ada di $TargetDir"
    exit 0
}

New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
$ZipFile = Join-Path $env:TEMP "scrcpy-win64-$Version.zip"
$ExtractDir = Join-Path $env:TEMP "scrcpy-extract-$Version"

Write-Host "Mengunduh scrcpy $Version dari $ZipUrl..."
Invoke-WebRequest -Uri $ZipUrl -OutFile $ZipFile

if (Test-Path $ExtractDir) { Remove-Item $ExtractDir -Recurse -Force }
Expand-Archive -Path $ZipFile -DestinationPath $ExtractDir -Force

$Inner = Get-ChildItem $ExtractDir -Directory | Select-Object -First 1
Copy-Item "$($Inner.FullName)\*" -Destination $TargetDir -Recurse -Force

Remove-Item $ZipFile, $ExtractDir -Recurse -Force
Write-Host "Selesai mengekstrak scrcpy $Version ke $TargetDir"
