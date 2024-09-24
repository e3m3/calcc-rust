# Copyright 2024, Giordano Salvador
# SPDX-License-Identifier: BSD-3-Clause

Set-PSDebug -Trace 2

#   Set input environment variable defaults
if ( !$env:BUILD_MODE ) {
    $env:BUILD_MODE="debug"
}

if ( !$env:WORKSPACE ) {
    $env:WORKSPACE="$env:USERPROFILE\Workspace"
    md "$env:WORKSPACE" -ea 0
}

#   Bootstrap chocolatey
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
Import-Module $env:ChocolateyInstall\helpers\chocolateyProfile.psm1
refreshenv

#   Install dependencies
choco feature disable --name showDownloadProgress
choco install -y git.install
choco install -y ninja
choco install -y cmake.install
choco install -y gnuwin32-coreutils.install
choco install -y python3
choco install -y python3-virtualenv
choco install -y StrawberryPerl
choco install -y psutils --checksum "8B651AC5DFF04BA64468F9ABC78F1A0483EE05CDB51218C63DA98096A5A7F5A0"
choco install -y rustup.install
choco install -y visualstudio2022-workload-vctools
refreshenv

#   Setup python
$LLVM_VER_LONG="18.1.8"
$env:PATH="C:\python312;$env:PATH"
$env:PYTHON_VENV_PATH="$env:WORKSPACE\python3-venv"
mkdir "$env:PYTHON_VENV_PATH"
python -m venv "$env:PYTHON_VENV_PATH"
$env:PATH="$env:PYTHON_VENV_PATH\bin;$env:PATH"
pip install "lit==$LLVM_VER_LONG"

#   Setup llvm
$LLVM_VER="18.x"
$LLVM_SRC="$env:WORKSPACE\llvm-project"
$env:LLVM_SYS_181_PREFIX="$LLVM_SRC\install"
$JOBS="4"

git clone --recursive --branch "release/$LLVM_VER" "https://github.com/llvm/llvm-project" "$LLVM_SRC"
cd "$LLVM_SRC"
cmake `
    -DCMAKE_BUILD_TYPE="MinSizeRel" `
    -DCMAKE_INSTALL_PREFIX="$LLVM_SRC\install" `
    -DLLVM_ENABLE_PROJECTS="clang;clang-tools-extra;compiler-rt;libc;lld;mlir;polly" `
    -DLLVM_TARGETS_TO_BUILD="AArch64;X86" `
    -S "$LLVM_SRC\llvm" `
    -B "$LLVM_SRC\build"
cmake --build "$LLVM_SRC\build" -j "$JOBS"
cmake --build "$LLVM_SRC\build" -j "$JOBS" --target install
$env:PATH="$LLVM_SRC\install\bin;$env:PATH"
cd "$env:WORKSPACE"

#   Setup rust
$RUSTUP_CHANNEL="stable"

rustup toolchain install "$RUSTUP_CHANNEL"
rustup override set "$RUSTUP_CHANNEL"

if ( "$env:BUILD_MODE" -eq "debug" ) {
    cargo build --verbose
    cargo test --verbose -- --nocapture
} elseif ( "$env:BUILD_MODE" -eq "release" ) {
    cargo build --verbose --release
    cargo test --verbose --release -- --nocapture
} else {
    Write-Error "Unknown build mode: $env:BUILD_MODE"
    exit 1
}
