cargo run --release --example server -- \
    --codec-option "crf 26" \
    --codec-option "preset veryfast" \
    --codec-option "tune zerolatency" \
    --codec-option "x264opts keyint=30"
