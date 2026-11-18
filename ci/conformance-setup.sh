#!/usr/bin/env bash
set -euo pipefail

rustup target add wasm32-wasip1

WASMTIME_VERSION="v50.0.0"
WASMTIME_ARCHIVE="wasmtime-${WASMTIME_VERSION}-x86_64-linux.tar.xz"
WASMTIME_URL="https://github.com/bytecodealliance/wasmtime/releases/download/${WASMTIME_VERSION}/${WASMTIME_ARCHIVE}"
WASMTIME_SHA256="77c19ee0437991f9ac7837b9c269d9c7076744f85dad263fea5214cc2723c7ff"

TMP_DIR="$(mktemp -d)"
curl -fsSL "${WASMTIME_URL}" -o "${TMP_DIR}/${WASMTIME_ARCHIVE}"
echo "${WASMTIME_SHA256}  ${TMP_DIR}/${WASMTIME_ARCHIVE}" | sha256sum -c -
tar -xJf "${TMP_DIR}/${WASMTIME_ARCHIVE}" -C "${TMP_DIR}"

INSTALL_DIR="${TMP_DIR}/wasmtime-${WASMTIME_VERSION}-x86_64-linux"
if [ -n "${GITHUB_PATH:-}" ]; then
    echo "${INSTALL_DIR}" >> "${GITHUB_PATH}"
fi
export PATH="${INSTALL_DIR}:${PATH}"
