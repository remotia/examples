#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TEST_DATA_DIR="${PROJECT_DIR}/test_data"

VIDEO_URL="https://ultravideo.fi/video/Beauty_1920x1080_120fps_420_8bit_YUV_RAW.7z"
ARCHIVE_NAME="Beauty_1920x1080_120fps_420_8bit_YUV_RAW.7z"
RAW_YUV_NAME="Beauty_1920x1080_120fps_420_8bit_YUV.yuv"
Y4M_NAME="Beauty_1920x1080.y4m"

VIDEO_WIDTH=1920
VIDEO_HEIGHT=1080
FPS=30

log() { echo "[*] $*"; }
err() { echo "[!] $*" >&2; }

download_archive() {
    local archive_path="${TEST_DATA_DIR}/${ARCHIVE_NAME}"

    if [[ -f "$archive_path" ]]; then
        log "Archive already exists: $archive_path"
        return 0
    fi

    if ! command -v curl &>/dev/null; then
        err "curl not found. Install curl to download test data."
        return 1
    fi

    log "Downloading ${ARCHIVE_NAME} (~883MB)..."
    curl -L --progress-bar -o "$archive_path" "$VIDEO_URL"
    log "Downloaded: $archive_path"
}

extract_archive() {
    local archive_path="${TEST_DATA_DIR}/${ARCHIVE_NAME}"
    local yuv_path="${TEST_DATA_DIR}/${RAW_YUV_NAME}"

    if [[ -f "$yuv_path" ]]; then
        log "Raw YUV already extracted: $yuv_path"
        return 0
    fi

    if ! command -v 7z &>/dev/null; then
        err "7z not found. Install p7zip to extract the archive."
        return 1
    fi

    log "Extracting ${ARCHIVE_NAME}..."
    7z x -aoa -o"${TEST_DATA_DIR}" "$archive_path" -bso0 -bse0
    log "Extracted: $yuv_path"
}

convert_to_y4m() {
    local yuv_path="${TEST_DATA_DIR}/${RAW_YUV_NAME}"
    local y4m_path="${TEST_DATA_DIR}/${Y4M_NAME}"

    if [[ -f "$y4m_path" ]]; then
        log "Y4M already exists: $y4m_path"
        return 0
    fi

    if ! command -v ffmpeg &>/dev/null; then
        err "ffmpeg not found. Install ffmpeg to convert raw YUV to Y4M."
        return 1
    fi

    log "Converting raw YUV to Y4M (${VIDEO_WIDTH}x${VIDEO_HEIGHT}, ${FPS}fps)..."
    ffmpeg -y \
        -f rawvideo -pix_fmt yuv420p \
        -s "${VIDEO_WIDTH}x${VIDEO_HEIGHT}" -r "$FPS" \
        -i "$yuv_path" \
        -pix_fmt yuv420p \
        "$y4m_path" \
        2>/dev/null

    log "Converted: $y4m_path"
}

main() {
    log "=== Download test data ==="

    mkdir -p "$TEST_DATA_DIR"
    download_archive
    extract_archive
    convert_to_y4m

    log "Cleaning up intermediate files..."
    rm -f "${TEST_DATA_DIR}/${ARCHIVE_NAME}" "${TEST_DATA_DIR}/${RAW_YUV_NAME}"

    log "=== Download complete ==="
    log "Test data in: $TEST_DATA_DIR"
}

main "$@"
