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