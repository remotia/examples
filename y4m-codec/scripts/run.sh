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
DECODER_TIMEOUT=120
MAX_FRAMES=0

Y4M_INPUT=""
H264_OUTPUT=""
DECODED_DIR=""
Y4M_ACTUAL=""

log() { echo "[*] $*"; }
err() { echo "[!] $*" >&2; }

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -n|--frames)
                MAX_FRAMES="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                Y4M_INPUT="$1"
                shift
                ;;
        esac
    done
}

resolve_input() {
    if [[ -n "$Y4M_INPUT" ]]; then
        if [[ ! -f "$Y4M_INPUT" ]]; then
            err "Specified input not found: $Y4M_INPUT"
            exit 1
        fi
    else
        Y4M_INPUT="${WORK_DIR}/sample_${VIDEO_WIDTH}x${VIDEO_HEIGHT}.y4m"
        generate_test_y4m
    fi

    truncate_input

    local basename
    basename=$(basename "$Y4M_ACTUAL" .y4m)
    H264_OUTPUT="${WORK_DIR}/${basename}_crf${CRF}.h264"
    DECODED_DIR="${WORK_DIR}/${basename}_crf${CRF}_decoded"
}

truncate_input() {
    if [[ "$MAX_FRAMES" -le 0 ]]; then
        Y4M_ACTUAL="$Y4M_INPUT"
        return 0
    fi

    local basename
    basename=$(basename "$Y4M_INPUT" .y4m)
    local truncated="${WORK_DIR}/${basename}_frames${MAX_FRAMES}.y4m"

    if [[ -f "$truncated" ]]; then
        log "Truncated Y4M already exists: $truncated"
        Y4M_ACTUAL="$truncated"
        FRAME_COUNT="$MAX_FRAMES"
        return 0
    fi

    if ! command -v ffmpeg &>/dev/null; then
        err "ffmpeg not found. Required for --frames truncation."
        exit 1
    fi

    log "Truncating to ${MAX_FRAMES} frames: $Y4M_INPUT -> $truncated"
    ffmpeg -y \
        -i "$Y4M_INPUT" \
        -vframes "$MAX_FRAMES" \
        -pix_fmt yuv420p \
        "$truncated" \
        2>/dev/null

    Y4M_ACTUAL="$truncated"
    FRAME_COUNT="$MAX_FRAMES"
    log "Truncated: $truncated"
}

generate_test_y4m() {
    if [[ -f "$Y4M_INPUT" ]]; then
        log "Test Y4M file already exists: $Y4M_INPUT"
        return 0
    fi

    if ! command -v ffmpeg &>/dev/null; then
        err "ffmpeg not found. Install ffmpeg or provide a Y4M file as argument."
        err "Example: $0 path/to/video.y4m"
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
    log "Encoding (CRF=${CRF}): $Y4M_ACTUAL -> $H264_OUTPUT"
    cargo run --manifest-path "${PROJECT_DIR}/Cargo.toml" --bin y4m-encoder -- \
        -i "$Y4M_ACTUAL" \
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

usage() {
    echo "Usage: $0 [OPTIONS] [Y4M_FILE]"
    echo ""
    echo "Runs the y4m-codec encode/decode cycle."
    echo ""
    echo "Options:"
    echo "  -n, --frames N  Encode only the first N frames"
    echo "  -h, --help      Show this help"
    echo ""
    echo "If no Y4M_FILE is given, generates a synthetic test clip."
    echo ""
    echo "Examples:"
    echo "  $0"
    echo "  $0 test_data/Beauty_1920x1080.y4m"
    echo "  $0 --frames 30 test_data/Beauty_1920x1080.y4m"
}

main() {
    parse_args "$@"

    log "=== y4m-codec sample run ==="

    mkdir -p "$WORK_DIR"
    resolve_input
    run_encoder
    run_decoder
    compare_roundtrip

    log "=== Sample run complete ==="
    log "Outputs in: $WORK_DIR"
}

main "$@"
