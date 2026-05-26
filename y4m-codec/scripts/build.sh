#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

log() { echo "[*] $*"; }

main() {
    log "Building FFmpeg via cargo-vcpkg..."
    cargo vcpkg --verbose build
    workdir="$PROJECT_DIR"

    log "Building y4m-codec binaries..."
    cargo build
    workdir="$PROJECT_DIR"

    log "Build complete"
}

main "$@"
