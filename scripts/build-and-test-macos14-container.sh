#!/bin/bash
# Copyright 2024, Giordano Salvador
# SPDX-License-Identifier: BSD-3-Clause

HOMEBREW_HOME=/opt/homebrew
eval "$(${HOMEBREW_HOME}/bin/brew shellenv)"

brew install docker-buildx

case ${BUILD_MODE} in
    debug)      build_mode= ;;
    release)    build_mode=--release ;;
    *)          echo "Error: BUILD_MODE=$BUILD_MODE" >&2  &&  exit 1 ;;
esac

docker-buildx build \
    -f container/Containerfile.${DISTRO}${OS_VER} \
    -t ${OS}-calcc${build_mode} \
    --build-arg=BUILD_MODE=${build_mode} \
    --progress=plain \
    .
