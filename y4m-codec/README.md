# y4m-codec

Y4M video encoder and decoder using FFmpeg (via rsmpeg) and the remotia pipeline framework.

## Binaries

- **y4m-encoder** — Reads a Y4M stream from stdin, encodes it to H.264, and writes packets to a file.
- **y4m-decoder** — Reads an H.264 stream from a file, decodes it, and writes individual frames as PNGs to a directory.

## Prerequisites

- Rust toolchain (MSRV 1.77+)
- [cargo-vcpkg](https://github.com/mcgoo/cargo-vcpkg): `cargo install cargo-vcpkg`
- Clang/LLVM (required by bindgen, which is a build dependency of `rusty_ffmpeg`)

## Building

This example uses **vcpkg** to build FFmpeg as a static library, and **rsmpeg 0.18** for Rust bindings. The build process follows the [cargo-vcpkg approach](https://github.com/larksuite/rsmpeg/blob/master/doc/vcpkg.md) described in the rsmpeg documentation.

### 1. Build FFmpeg via cargo-vcpkg

```bash
cargo vcpkg --verbose build
```

This clones vcpkg at the pinned revision and builds FFmpeg with x264 support. This step is slow on the first run (compiles FFmpeg from source) but cached afterwards.

### 2. Build

```bash
cargo build
```

## Usage

### Decoder

```bash
cargo run --bin y4m-decoder -- \
  -i input.h264 \
  -o output_frames/ \
  -w 1920 \
  -h 1080
```

Decodes an H.264 bitstream into individual PNG frames in the output directory.

### Encoder

```bash
cat input.y4m | cargo run --bin y4m-encoder -- \
  -i - \
  -o output.h264 \
  --crf 23
```

Encodes a Y4M stream (read from stdin) into an H.264 file.

## Scripts

Helper scripts for building, downloading test data, and running sample encode/decode cycles.

### Additional prerequisites

- `ffmpeg` — for generating test clips and converting raw YUV to Y4M
- `curl` — for downloading test data
- `7z` — for extracting .7z archives

### `scripts/build.sh`

Builds the FFmpeg dependency via cargo-vcpkg and compiles the `y4m-encoder` and `y4m-decoder` binaries.

```bash
scripts/build.sh
```

### `scripts/download_test_data.sh`

Downloads the [Beauty](https://ultravideo.fi/video/Beauty_1920x1080_120fps_420_8bit_YUV_RAW.7z) raw video (~883 MB), extracts it, and converts it to Y4M format (1920x1080, 30fps).

All files are placed in `test_data/`, which is tracked in git but ignores its contents.

```bash
scripts/download_test_data.sh
```

Resulting files:

| File | Description |
|---|---|
| `test_data/Beauty_1920x1080_120fps_420_8bit_YUV_RAW.7z` | Downloaded archive |
| `test_data/Beauty_1920x1080_120fps_420_8bit_YUV.yuv` | Extracted raw YUV |
| `test_data/Beauty_1920x1080.y4m` | Converted Y4M (encoder input) |

### `scripts/run.sh`

Runs a full encode/decode cycle and validates the roundtrip frame count.

```bash
scripts/run.sh [OPTIONS] [Y4M_FILE]
```

**Options:**

| Option | Description |
|---|---|
| `-n, --frames N` | Encode only the first N frames |
| `-h, --help` | Show help |

If no `Y4M_FILE` is given, a synthetic 1920x1080 test clip (5s, 30fps) is generated via ffmpeg's `testsrc`.

The encoder runs at CRF 23 by default. The decoder has a 120s timeout to avoid hanging on known EOF-handling issues.

All outputs go to `sample-runs/`.

**Examples:**

```bash
# Synthetic test clip (auto-generated)
scripts/run.sh

# Full Beauty video
scripts/download_test_data.sh
scripts/run.sh test_data/Beauty_1920x1080.y4m

# First 30 frames only
scripts/run.sh --frames 30 test_data/Beauty_1920x1080.y4m
```

### Typical workflow

```bash
scripts/build.sh
scripts/download_test_data.sh
scripts/run.sh --frames 30 test_data/Beauty_1920x1080.y4m
```

### Output structure

```
sample-runs/
  <name>_crf23.h264               # Encoded H.264 bitstream
  <name>_crf23_decoded/            # Decoded PNG frames
    frame_0000.png
    frame_0001.png
    ...
  <name>_frames30.y4m             # Truncated Y4M (when --frames is used)
```

### Known issues

The decoder may hang before reading all frames due to a pipeline EOF-handling bug. The `run.sh` script applies a 120s timeout to the decoder to prevent indefinite hangs. With `--frames 30`, the full roundtrip completes successfully.