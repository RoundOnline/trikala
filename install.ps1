# install.ps1 — fetch the trikala CLI binary for Windows.
#
# Usage:
#   irm https://trikala.round.online/install.ps1 | iex
#
# Honors $env:TRIKALA_INSTALL_DIR (default: $env:LOCALAPPDATA\trikala\bin).

$ErrorActionPreference = "Stop"

$Repo = "RoundOnline/trikala"
$Version = if ($args.Count -gt 0) { $args[0] } else { "latest" }
$Asset = "trikala-windows-x86_64.zip"

$Url = if ($Version -eq "latest") {
    "https://github.com/$Repo/releases/latest/download/$Asset"
} else {
    "https://github.com/$Repo/releases/download/$Version/$Asset"
}

$Tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("trikala-install-" + [guid]::NewGuid())) | Select-Object -ExpandProperty FullName

try {
    $ZipPath = Join-Path $Tmp $Asset
    Write-Host "→ fetching $Asset"
    try {
        Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing
    } catch {
        Write-Host "[ATI-103] download failed: $Url" -ForegroundColor Red
        Write-Host "  hint: check the version exists at https://github.com/$Repo/releases"
        exit 1
    }

    Expand-Archive -Path $ZipPath -DestinationPath $Tmp -Force

    $InstallDir = if ($env:TRIKALA_INSTALL_DIR) { $env:TRIKALA_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "trikala\bin" }
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

    $DstPath = Join-Path $InstallDir "trikala.exe"
    Move-Item -Force -Path (Join-Path $Tmp "trikala.exe") -Destination $DstPath

    Write-Host ""
    Write-Host "✓ installed to $DstPath" -ForegroundColor Green

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        Write-Host ""
        Write-Host "$InstallDir is NOT on your PATH. Add it with:"
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$InstallDir', 'User')"
    }

    Write-Host ""
    Write-Host "Try: trikala --version"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
