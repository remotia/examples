Y4M_INPUT="$1"

if [ -z "$Y4M_INPUT" ]; then
    echo "Error: y4m input file is required. Usage: $0 <input.y4m> [output_dir]" >&2
    exit 1
fi

OUTPUT_DIR="${2:-sample-runs}"

mkdir -p "$OUTPUT_DIR"
basename=$(basename "$Y4M_INPUT" .y4m)
H264_OUTPUT="${OUTPUT_DIR}/${basename}_x264.h264"
DECODED_DIR="${OUTPUT_DIR}/${basename}_x264_decoded"

cargo run --release --bin y4m-encoder -- \
    -i "$Y4M_INPUT" -o "$H264_OUTPUT" --frames 30 \
    --codec libx264 --option "crf=23" --option "preset=medium" --option "tune=film"

cargo run --release --bin y4m-decoder -- \
    -i "$H264_OUTPUT" -o "$DECODED_DIR" -W 1920 -H 1080 --codec h264
