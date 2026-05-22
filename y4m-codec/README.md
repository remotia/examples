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

### 2. Set environment variables

```bash
export VCPKG_ROOT="${PWD}/target/vcpkg"
export FFMPEG_BINDING_PATH="$(find ~/.cargo/registry/src -path '*/rusty_ffmpeg-0.16.7*/src/binding.rs' -type f 2>/dev/null)"
```

- **`VCPKG_ROOT`** — Tells the vcpkg crate where to find the vcpkg installation for library linking.
- **`FFMPEG_BINDING_PATH`** — Uses the pre-generated FFmpeg 7 binding shipped inside the `rusty_ffmpeg` crate, bypassing runtime bindgen. This is necessary when the system has FFmpeg 8+ headers installed, because bindgen would otherwise pick those up and generate incompatible opaque struct bindings.

> **Note:** If your system does **not** have FFmpeg headers installed (i.e. `/usr/include/libavformat/avformat.h` does not exist), you can omit `FFMPEG_BINDING_PATH` and let bindgen generate bindings at build time from the vcpkg headers. The pre-generated binding path is unstable (it includes the registry source hash) — if the `find` command returns empty, run a `cargo build` once without `FFMPEG_BINDING_PATH` to force `rusty_ffmpeg` to be fetched, then re-run the `find` command.

### 3. Build

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

## Troubleshooting

### "No vcpkg installation found"

Make sure `VCPKG_ROOT` is set and points to the `target/vcpkg` directory. If you ran `cargo clean`, the vcpkg installed files are also removed — re-run `cargo vcpkg --verbose build`.

### Opaque struct errors from rsmpeg (e.g. "no field `pb` on type `AVFormatContext`")

This means bindgen generated FFmpeg 8 bindings (opaque structs) instead of FFmpeg 7 bindings (full struct layout). Set `FFMPEG_BINDING_PATH` to use the pre-generated binding.

### `FFMPEG_BINDING_PATH` not found

The pre-generated binding lives inside cargo's registry cache. The exact path includes a hash that varies per machine. If the `find` command returns nothing, the `rusty_ffmpeg` crate source hasn't been fetched yet. Run `cargo build` once (it will fail, but the source gets downloaded), then retry the `find` command.

### Build fails on Windows MSVC

Make sure `.cargo/config.toml` exists in the project with the required Windows link arguments (see the [rsmpeg vcpkg guide](https://github.com/larksuite/rsmpeg/blob/master/doc/vcpkg.md) for details).
