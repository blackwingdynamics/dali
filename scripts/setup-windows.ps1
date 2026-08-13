$ErrorActionPreference = "Stop"

$RepositoryRoot = Split-Path -Parent $PSScriptRoot
$RustToolchain = "1.97.1"
$EmbeddedTarget = "thumbv7em-none-eabihf"
$GitCliffVersion = "2.13.0"
$ToolsRoot = Join-Path $env:LOCALAPPDATA "DaliTools"
$ToolsBin = Join-Path $ToolsRoot "bin"
$CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$RenodeArchiveUrl = "https://builds.renode.io/renode-latest.windows-portable.zip"
$DfuArchiveUrl = "https://sourceforge.net/projects/dfu-util/files/dfu-util-0.11-binaries.tar.xz/download"

function Write-SetupLog([string]$Message) {
    Write-Host "[setup-windows] $Message"
}

function Fail-Setup([string]$Message) {
    throw "[setup-windows] ERROR: $Message"
}

function Require-Windows {
    if ($env:OS -ne "Windows_NT") {
        Fail-Setup "This script supports Windows only."
    }
}

function Require-RepositoryRoot {
    $toolchainPath = Join-Path $RepositoryRoot "rust-toolchain.toml"
    if (-not (Test-Path -LiteralPath $toolchainPath)) {
        Fail-Setup "Run this script from a Dali OS checkout containing rust-toolchain.toml."
    }
}

function Require-WinGet {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Fail-Setup "WinGet is required. Install or update App Installer, then rerun this script."
    }
}

function Add-UserPath([string]$Directory) {
    New-Item -ItemType Directory -Force -Path $Directory | Out-Null
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @($currentPath -split ";" | Where-Object { $_ })
    if ($pathEntries -notcontains $Directory) {
        $updatedPath = ($pathEntries + $Directory) -join ";"
        [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
    }
    if ($env:Path -notlike "*${Directory}*") {
        $env:Path = "${Directory};$env:Path"
    }
}

function Install-WinGetPackage([string]$PackageId, [string]$Override = "") {
    $arguments = @(
        "install", "--id", $PackageId, "--exact",
        "--accept-source-agreements", "--accept-package-agreements"
    )
    if ($Override) {
        $arguments += @("--override", $Override)
    }
    Write-SetupLog "Installing or updating $PackageId."
    & winget @arguments
    if ($LASTEXITCODE -ne 0) {
        Fail-Setup "WinGet failed for package: $PackageId"
    }
}

function Install-SystemTools {
    Install-WinGetPackage "Git.Git"
    Install-WinGetPackage "Kitware.CMake"
    Install-WinGetPackage "Python.Python.3"
    Install-WinGetPackage "Rustlang.Rustup"
    Install-WinGetPackage "Microsoft.VisualStudio.2022.BuildTools" "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
}

function Install-RustToolchain {
    Add-UserPath $CargoBin
    Write-SetupLog "Installing the pinned Rust toolchain and embedded target."
    & rustup toolchain install $RustToolchain `
        --profile minimal `
        --component clippy `
        --component llvm-tools-preview `
        --component rustfmt `
        --target $EmbeddedTarget
    if ($LASTEXITCODE -ne 0) {
        Fail-Setup "rustup failed to install the pinned toolchain."
    }
}

function Install-CargoTool([string]$PackageName, [string]$CommandName, [string]$Version = "") {
    if (Get-Command $CommandName -ErrorAction SilentlyContinue) {
        Write-SetupLog "$CommandName is already installed."
        return
    }
    Write-SetupLog "Installing $PackageName with Cargo."
    if ($Version) {
        & cargo install $PackageName --version $Version --locked
    } else {
        & cargo install $PackageName --locked
    }
    if ($LASTEXITCODE -ne 0) {
        Fail-Setup "Cargo failed to install $PackageName."
    }
}

function Install-Renode {
    if (Get-Command renode -ErrorAction SilentlyContinue) {
        Write-SetupLog "Renode is already installed."
        return
    }

    $downloadDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "dali-renode"
    $archivePath = Join-Path $downloadDirectory "renode.zip"
    $renodeDirectory = Join-Path $ToolsRoot "renode"
    New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null
    New-Item -ItemType Directory -Force -Path $renodeDirectory | Out-Null

    Write-SetupLog "Installing the portable Renode release."
    Invoke-WebRequest -Uri $RenodeArchiveUrl -OutFile $archivePath
    Expand-Archive -Path $archivePath -DestinationPath $renodeDirectory -Force
    $renodeExecutable = Get-ChildItem -Path $renodeDirectory -Filter "renode.exe" -Recurse | Select-Object -First 1
    if (-not $renodeExecutable) {
        Fail-Setup "Renode executable was not found in the downloaded archive."
    }
    Add-UserPath $renodeExecutable.DirectoryName
    Remove-Item -Recurse -Force $downloadDirectory
}

function Install-DfuUtil {
    if (Get-Command dfu-util -ErrorAction SilentlyContinue) {
        Write-SetupLog "dfu-util is already installed."
        return
    }

    $downloadDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "dali-dfu-util"
    $archivePath = Join-Path $downloadDirectory "dfu-util.tar.xz"
    New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null

    Write-SetupLog "Installing dfu-util from the official Windows binary archive."
    Invoke-WebRequest -Uri $DfuArchiveUrl -OutFile $archivePath
    tar -xf $archivePath -C $downloadDirectory
    $dfuExecutable = Get-ChildItem -Path $downloadDirectory -Filter "dfu-util.exe" -Recurse | Select-Object -First 1
    if (-not $dfuExecutable) {
        Fail-Setup "dfu-util.exe was not found in the downloaded archive."
    }
    Copy-Item -LiteralPath $dfuExecutable.FullName -Destination (Join-Path $ToolsBin "dfu-util.exe") -Force
    Add-UserPath $ToolsBin
    Remove-Item -Recurse -Force $downloadDirectory
}

function Install-DevelopmentTools {
    Install-DfuUtil
    Install-Renode
    Install-CargoTool "cargo-binutils" "cargo-objcopy"
    Install-CargoTool "just" "just"
    Install-CargoTool "lefthook" "lefthook"
    Install-CargoTool "git-cliff" "git-cliff" $GitCliffVersion
    Install-CargoTool "probe-rs-tools" "probe-rs"

    Write-SetupLog "Installing Lefthook hooks."
    Push-Location $RepositoryRoot
    try {
        & lefthook install
        if ($LASTEXITCODE -ne 0) {
            Fail-Setup "Lefthook installation failed."
        }
    } finally {
        Pop-Location
    }
}

function Verify-Tools {
    $requiredCommands = @(
        "cargo", "cargo-objcopy", "dfu-util", "git", "git-cliff",
        "just", "lefthook", "probe-rs", "renode", "rustc", "rustup"
    )
    Write-SetupLog "Verifying installed commands."
    foreach ($commandName in $requiredCommands) {
        if (-not (Get-Command $commandName -ErrorAction SilentlyContinue)) {
            Fail-Setup "Required command is not available: $commandName"
        }
    }
    & rustup run $RustToolchain rustc --version
    $installedTargets = & rustup target list --installed --toolchain $RustToolchain
    if ($installedTargets -notcontains $EmbeddedTarget) {
        Fail-Setup "Embedded target is not installed: $EmbeddedTarget"
    }
    & probe-rs --version
    & renode --version
}

function Run-RepositoryCheck {
    Write-SetupLog "Running the embedded kernel check."
    Push-Location $RepositoryRoot
    try {
        & cargo check-kernel
        if ($LASTEXITCODE -ne 0) {
            Fail-Setup "The embedded kernel check failed."
        }
    } finally {
        Pop-Location
    }
}

function Main {
    Require-Windows
    Require-RepositoryRoot
    Require-WinGet
    Install-SystemTools
    Install-RustToolchain
    Install-DevelopmentTools
    Verify-Tools
    Run-RepositoryCheck
    Write-SetupLog "Dali OS Windows development environment is ready."
    Write-SetupLog "DFU devices require a WinUSB-compatible driver. ST-Link may require its vendor driver."
}

Main
