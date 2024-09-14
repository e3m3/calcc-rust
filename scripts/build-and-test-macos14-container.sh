#!/bin/bash
# Copyright 2024, Giordano Salvador
# SPDX-License-Identifier: BSD-3-Clause

HOMEBREW_HOME=/opt/homebrew
eval "$(${HOMEBREW_HOME}/bin/brew shellenv)"

brew install podman

case ${BUILD_MODE}
    debug)      build_mode=
    release)    build_mode=--release
    *)          echo "Error: BUILD_MODE=$BUILD_MODE" >2  &&  exit 1
esac

podman machine init
podman machine start
podman build \
    -f container/Containerfile.${DISTRO}${OS_VER} \
    -t ${OS}-calcc-${build_mode} \
    --build-arg=BUILD_MODE=${build_mode} \
    .
