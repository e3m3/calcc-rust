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
$CHOCOLATEY_URL="https://community.chocolatey.org/install.ps1"
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = `
    [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
iex ((New-Object System.Net.WebClient).DownloadString("$CHOCOLATEY_URL"))
Import-Module "$env:ChocolateyInstall\helpers\chocolateyProfile.psm1"
refreshenv

#   Install dependencies
choco feature disable --name showDownloadProgress
choco install -y sed
choco install -y git.install
choco install -y ninja
choco install -y cmake.install
choco install -y gnuwin32-coreutils.install
choco install -y python3
choco install -y python3-virtualenv
choco install -y StrawberryPerl
choco install -y psutils `
    --checksum "8B651AC5DFF04BA64468F9ABC78F1A0483EE05CDB51218C63DA98096A5A7F5A0"
choco install -y rustup.install
choco install -y visualstudio2022-workload-vctools
refreshenv

#   Setup visual studio
$VS_PATH="C:\Program Files (x86)\Microsoft Visual Studio\2022"
cmd /C "$VS_PATH\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" "amd64"

#   Setup python
$LLVM_VER_LONG="18.1.8"
$env:PATH="C:\python312;$env:PATH"
$env:PYTHON_VENV_PATH="$env:WORKSPACE\python3-venv"
mkdir "$env:PYTHON_VENV_PATH"
python -m venv "$env:PYTHON_VENV_PATH"
$env:PATH="$env:PYTHON_VENV_PATH\bin;$env:PATH"
pip install "lit==$LLVM_VER_LONG"

#   Setup llvm
$LLVM_VER_MAJOR="18"
$LLVM_VER="$LLVM_VER_MAJOR.x"
$LLVM_SRC="$env:WORKSPACE\llvm-project"
$env:LLVM_SYS_181_PREFIX="$LLVM_SRC\install"
$JOBS="4"
$CMAKE_BUILD_TYPE="MinSizeRel"
$LLVM_PROJECTS="clang;clang-tools-extra;compiler-rt;libc;lld;polly"

git clone --recursive --branch "release/$LLVM_VER" `
    "https://github.com/llvm/llvm-project" "$LLVM_SRC"
cd "$LLVM_SRC"
cmake `
    -DCMAKE_BUILD_TYPE="$CMAKE_BUILD_TYPE" `
    -DCMAKE_INSTALL_PREFIX="$env:LLVM_SYS_181_PREFIX" `
    -DLLVM_ENABLE_PROJECTS="$LLVM_PROJECTS" `
    -DLLVM_TARGETS_TO_BUILD="X86" `
    -DLLVM_LIT_TOOLS="C:\Program Files (x86)\GnuWin32" `
    -DLLVM_BUILD_TOOLS=ON `
    -DLLVM_INCLUDE_TOOLS=ON `
    -DLLVM_INSTALL_UTILS=ON `
    -S "$LLVM_SRC\llvm" `
    -B "$LLVM_SRC\build"
cmake --build "$LLVM_SRC\build" -j "$JOBS" --config "$CMAKE_BUILD_TYPE"

#   Perform LLVM install manually
md "$env:LLVM_SYS_181_PREFIX" -ea 0
md "$env:LLVM_SYS_181_PREFIX\bin" -ea 0
md "$env:LLVM_SYS_181_PREFIX\include" -ea 0
md "$env:LLVM_SYS_181_PREFIX\lib" -ea 0
md "$env:LLVM_SYS_181_PREFIX\share" -ea 0
cp "$LLVM_SRC\build\$CMAKE_BUILD_TYPE\libllvm-c.args" "$env:LLVM_SYS_181_PREFIX"
cp "$LLVM_SRC\build\$CMAKE_BUILD_TYPE\libllvm-c.exports" "$env:LLVM_SYS_181_PREFIX"
robocopy /e /v /xn "$LLVM_SRC\build\$CMAKE_BUILD_TYPE\bin" "$env:LLVM_SYS_181_PREFIX\bin"
robocopy /e /v /xn "$LLVM_SRC\build\$CMAKE_BUILD_TYPE\lib" "$env:LLVM_SYS_181_PREFIX\lib"
robocopy /e /v /xn "$LLVM_SRC\build\$CMAKE_BUILD_TYPE\lib\clang\$LLVM_VER_MAJOR\include" `
    "$env:LLVM_SYS_181_PREFIX\include"
robocopy /e /v /xn "$LLVM_SRC\build\$CMAKE_BUILD_TYPE\lib\clang\$LLVM_VER_MAJOR\lib" `
    "$env:LLVM_SYS_181_PREFIX\lib"
robocopy /e /v /xn "$LLVM_SRC\build\$CMAKE_BUILD_TYPE\lib\clang\$LLVM_VER_MAJOR\share" `
    "$env:LLVM_SYS_181_PREFIX\share"
robocopy /e /v /xn "$LLVM_SRC\build\include" "$env:LLVM_SYS_181_PREFIX\include"
robocopy /e /v /xn "$LLVM_SRC\llvm\include" "$env:LLVM_SYS_181_PREFIX\include"
foreach ( $project in $LLVM_PROJECTS.split(";") ) {
    $project_dir="$LLVM_SRC\$project\include"
    if ( Test-Path -Path "$project_dir" ) {
        robocopy /e /v /xn "$project_dir" "$env:LLVM_SYS_181_PREFIX\include"
    }
}
$env:PATH="$env:LLVM_SYS_181_PREFIX\bin;$env:PATH"

#   Setup rust
$RUSTUP_CHANNEL="stable"
rustup toolchain install "$RUSTUP_CHANNEL"
rustup override set "$RUSTUP_CHANNEL"

cd "$PSScriptRoot\.."
sed -i "s/force-dynamic/force-static/g" ".\Cargo.toml"

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
