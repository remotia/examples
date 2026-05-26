#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
WORK_DIR="${PROJECT_DIR}/sample-runs"

VIDEO_WIDTH=1920
VIDEO_HEIGHT=1080
DURATION=5
FPS=30
FRAME_COUNT=$((DURATION * FPS))
CRF=23

Y4M_INPUT="${WORK_DIR}/sample_${VIDEO_WIDTH}x${VIDEO_HEIGHT}.y4m"
H264_OUTPUT="${WORK_DIR}/encoded_crf${CRF}.h264"
DECODED_DIR="${WORK_DIR}/decoded_crf${CRF}"
DECODER_TIMEOUT=120

log() { echo "[*] $*"; }
err() { echo "[!] $*" >&2; }

generate_test_y4m() {
    if [[ -f "$Y4M_INPUT" ]]; then
        log "Test Y4M file already exists: $Y4M_INPUT"
        return 0
    fi

    if ! command -v ffmpeg &>/dev/null; then
        err "ffmpeg not found. Install ffmpeg or provide a Y4M file at: $Y4M_INPUT"
        err "Example: ffmpeg -f lavfi -i testsrc=s=${VIDEO_WIDTH}x${VIDEO_HEIGHT}:r=${FPS} -t ${DURATION} -pix_fmt yuv420p \"$Y4M_INPUT\""
        return 1
    fi

    log "Generating test Y4M file (${VIDEO_WIDTH}x${VIDEO_HEIGHT}, ${DURATION}s, ${FPS}fps)..."
    ffmpeg -y \
        -f lavfi -i "testsrc=s=${VIDEO_WIDTH}x${VIDEO_HEIGHT}:r=${FPS}" \
        -f lavfi -i "sine=frequency=440:duration=${DURATION}" \
        -t "$DURATION" \
        -pix_fmt yuv420p \
        "$Y4M_INPUT" \
        2>/dev/null

    log "Generated: $Y4M_INPUT"
}

run_encoder() {
    log "Encoding (CRF=${CRF}): $Y4M_INPUT -> $H264_OUTPUT"
    cargo run --manifest-path "${PROJECT_DIR}/Cargo.toml" --bin y4m-encoder -- \
        -i "$Y4M_INPUT" \
        -o "$H264_OUTPUT" \
        --crf "$CRF"

    local size
    size=$(stat -c%s "$H264_OUTPUT" 2>/dev/null || stat -f%z "$H264_OUTPUT" 2>/dev/null)
    log "Encoded output: $H264_OUTPUT ($size bytes)"
}

run_decoder() {
    log "Decoding: $H264_OUTPUT -> $DECODED_DIR/ (timeout: ${DECODER_TIMEOUT}s)"
    timeout "$DECODER_TIMEOUT" \
        cargo run --manifest-path "${PROJECT_DIR}/Cargo.toml" --bin y4m-decoder -- \
        -i "$H264_OUTPUT" \
        -o "$DECODED_DIR" \
        -W "$VIDEO_WIDTH" \
        -H "$VIDEO_HEIGHT" || true

    local frame_count
    frame_count=$(find "$DECODED_DIR" -name "*.png" 2>/dev/null | wc -l)
    log "Decoded $frame_count frames to $DECODED_DIR/"
}

compare_roundtrip() {
    local frame_count
    frame_count=$(find "$DECODED_DIR" -name "*.png" 2>/dev/null | wc -l)

    if [[ "$frame_count" -eq "$FRAME_COUNT" ]]; then
        log "Roundtrip: OK (${frame_count}/${FRAME_COUNT} frames)"
    else
        err "Roundtrip: frame count mismatch (${frame_count}/${FRAME_COUNT})"
    fi
}

main() {
    log "=== y4m-codec sample run ==="

    mkdir -p "$WORK_DIR"
    generate_test_y4m
    run_encoder
    run_decoder
    compare_roundtrip

    log "=== Sample run complete ==="
    log "Outputs in: $WORK_DIR"
}

main "$@"
