#!/usr/bin/env pwsh
# llmwiki-cli installer for PowerShell 7+ (cross-platform).
# Usage:
#   irm https://github.com/Git-Fg/llmwiki/releases/latest/download/install.ps1 | iex
#   iex ((irm https://github.com/Git-Fg/llmwiki/releases/latest/download/install.ps1) -BinDir C:\tools)

[CmdletBinding()]
param(
    [Parameter()]
    [string]$Version = "latest",

    [Parameter()]
    [string]$BinDir = $(
        if ($IsWindows) {
            if ($env:LOCALAPPDATA) {
                Join-Path $env:LOCALAPPDATA "llmwiki-cli\bin"
            } else {
                Join-Path $HOME ".local\bin"
            }
        } else {
            if ($env:HOME) {
                Join-Path $env:HOME ".local/bin"
            } else {
                "/usr/local/bin"
            }
        }
    ),

    [Parameter()]
    [switch]$Force,

    [Parameter()]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$Repo = "Git-Fg/llmwiki"
$Binary = "llmwiki-cli"

function New-TempDir {
    if ($IsWindows) {
        $path = Join-Path $env:TEMP ("llmwiki-" + [Guid]::NewGuid().ToString("N"))
    } else {
        $path = "/tmp/llmwiki-" + [Guid]::NewGuid().ToString("N")
    }
    New-Item -ItemType Directory -Path $path | Out-Null
    return $path
}

# PowerShell 5.1 (Windows PowerShell) requires Tls12 for HTTPS downloads
# from GitHub. PowerShell 7+ uses Tls12 by default.
if ($PSVersionTable.PSVersion.Major -lt 7) {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
}

if ($Help) {
    @"
Usage: install.ps1 [-Version <ver>] [-BinDir <dir>] [-Force] [-Help]

Options:
  -Version <ver>    Release tag to install (default: latest)
  -BinDir   <dir>   Install directory (default: platform-specific)
  -Force            Overwrite an existing binary without prompting
  -Help             Show this help and exit

This script requires PowerShell 7+ (cross-platform pwsh). On Windows you
can install it via:  winget install Microsoft.PowerShell

If you are on Windows PowerShell 5.1, Tls12 is enabled for HTTPS downloads.
"@
    exit 0
}

# --- Detect OS and architecture ---
if ($IsWindows) {
    $TargetOs = "pc-windows-gnu"
} elseif ($IsLinux) {
    $TargetOs = "unknown-linux-musl"
} elseif ($IsMacOS) {
    $TargetOs = "apple-darwin"
} else {
    Write-Error "Unsupported OS: $($PSVersionTable.OS)"
    exit 1
}

switch -Wildcard ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64" { $TargetArch = "x86_64" }
    "ARM64" { $TargetArch = "aarch64" }
    "x86_64" { $TargetArch = "x86_64" }
    "aarch64" { $TargetArch = "aarch64" }
    default {
        Write-Error "Unsupported architecture: $($env:PROCESSOR_ARCHITECTURE)"
        exit 1
    }
}

$Target = "${TargetArch}-${TargetOs}"
Write-Host "Detected platform: $Target"

# --- Resolve the download URL ---
if ($Version -eq "latest") {
    $Base = "https://github.com/$Repo/releases/latest/download"
} else {
    $Base = "https://github.com/$Repo/releases/download/$Version"
}

if ($IsWindows) {
    $Asset = "${Binary}-${Target}.zip"
} else {
    $Asset = "${Binary}-${Target}.tar.gz"
}

$Url = "$Base/$Asset"
$ShaUrl = "$Url.sha256"
$Tmp = New-TempDir

try {
    # --- Overwrite guard ---
    $ExistingPath = Join-Path $BinDir $Binary
    $ExistingExe = if ($IsWindows) { "$ExistingPath.exe" } else { $ExistingPath }
    if ((Test-Path $ExistingExe) -and -not $Force) {
        Write-Error "Existing binary found at $ExistingExe. Use -Force to overwrite."
        exit 1
    }

    # --- Download ---
    Write-Host "Downloading $Url..."
    Invoke-WebRequest -Uri $Url -OutFile "$Tmp/$Asset" -UseBasicParsing

    # --- Verify SHA256 ---
    Write-Host "Verifying SHA256..."
    $Expected = (Invoke-WebRequest -Uri $ShaUrl -UseBasicParsing).Content.Trim().Split()[0]
    $Actual = (Get-FileHash -Path "$Tmp/$Asset" -Algorithm SHA256).Hash.ToLower()
    if ($Expected -ne $Actual) {
        Write-Error "SHA256 mismatch:`n  expected: $Expected`n  actual:   $Actual"
        exit 1
    }

    # --- Extract + install ---
    if ($IsWindows) {
        Expand-Archive -Path "$Tmp/$Asset" -DestinationPath $Tmp -Force
    } else {
        tar -xzf "$Tmp/$Asset" -C $Tmp
    }

    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    $InstalledSrc = if ($IsWindows) { "$Tmp/$Binary.exe" } else { "$Tmp/$Binary" }
    Move-Item -Path $InstalledSrc -Destination $ExistingExe -Force

    $Tag = $Version
    if ($Tag -eq "latest") { $Tag = "(latest)" }

    Write-Host ""
    Write-Host "✓ Installed $Binary $Tag to $BinDir"
    Write-Host ""
    Write-Host "Next steps:"
    if ($IsWindows) {
        Write-Host "  1. Add $BinDir to your PATH if not already:"
        Write-Host "       [Environment]::SetEnvironmentVariable('Path', '$BinDir;' + `$env:Path, 'User')"
    } else {
        Write-Host "  1. Ensure $BinDir is in your PATH:"
        Write-Host "       export PATH=`"$BinDir:`$PATH`""
    }
    Write-Host "  2. Verify the install:"
    Write-Host "       $Binary doctor"
}
finally {
    if (Test-Path $Tmp) { Remove-Item -Recurse -Force $Tmp }
}