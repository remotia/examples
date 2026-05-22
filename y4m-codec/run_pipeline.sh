#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -lt 3 ]; then
    echo "Usage: $0 <input.y4m> <width> <height> [crf]"
    echo ""
    echo "  input.y4m  Path to the input Y4M file"
    echo "  width      Video width in pixels"
    echo "  height     Video height in pixels"
    echo "  crf        H.264 CRF value (default: 23)"
    exit 1
fi

INPUT_Y4M="$1"
WIDTH="$2"
HEIGHT="$3"
CRF="${4:-23}"

WORK_DIR=$(mktemp -d)
H264_FILE="${WORK_DIR}/output.h264"
DECODE_DIR="${WORK_DIR}/decoded"

echo "=== y4m-codec encode-decode pipeline ==="
echo "Input:  ${INPUT_Y4M} (${WIDTH}x${HEIGHT})"
echo "CRF:    ${CRF}"
echo "Work:   ${WORK_DIR}"
echo ""

cargo build --release --manifest-path "${SCRIPT_DIR}/Cargo.toml" 2>&1

ENCODER="${SCRIPT_DIR}/target/release/y4m-encoder"
DECODER="${SCRIPT_DIR}/target/release/y4m-decoder"

echo "--- Encoding ${INPUT_Y4M} -> ${H264_FILE} ---"
RUST_LOG=info "${ENCODER}" --input "${INPUT_Y4M}" --output "${H264_FILE}" --crf "${CRF}"

H264_SIZE=$(stat -c%s "${H264_FILE}" 2>/dev/null || stat -f%z "${H264_FILE}")
echo "Encoded file size: ${H264_SIZE} bytes"
echo ""

echo "--- Decoding ${H264_FILE} -> ${DECODE_DIR}/ ---"
RUST_LOG=info "${DECODER}" --input "${H264_FILE}" --output-dir "${DECODE_DIR}" --width "${WIDTH}" --height "${HEIGHT}"

FRAME_COUNT=$(ls -1 "${DECODE_DIR}"/*.png 2>/dev/null | wc -l)
echo "Decoded ${FRAME_COUNT} frames to ${DECODE_DIR}/"
echo ""

echo "=== Pipeline complete ==="
echo "H264 file:   ${H264_FILE}"
echo "PNG frames:  ${DECODE_DIR}/"
