use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use async_trait::async_trait;
use png::{BitDepth, ColorType, Compression, Encoder};
use remotia::traits::{BorrowFrameProperties, FrameProcessor};

use crate::{BufferType, FrameData};

/// Writes decoded RGBA frames to PNG files in an output directory.
///
/// Each frame is saved as `frame_NNNN.png` with a monotonically increasing counter.
/// Handles line-size padding by repacking rows to the expected RGBA stride before
/// encoding.
pub struct PNGWriter {
    output_dir: PathBuf,
    width: u32,
    height: u32,
    frame_count: u32,
}

impl PNGWriter {
    /// Creates a new PNG writer. The output directory is created if it does not exist.
    pub fn new(output_dir: PathBuf, width: u32, height: u32) -> Self {
        std::fs::create_dir_all(&output_dir).expect("Unable to create output directory");
        Self {
            output_dir,
            width,
            height,
            frame_count: 0,
        }
    }

    fn write_frame(&mut self, rgba_data: &[u8]) {
        if rgba_data.is_empty() {
            return;
        }

        let row_bytes = self.width as usize * 4;
        let expected_linesize = row_bytes;
        let actual_linesize = rgba_data.len() / self.height as usize;

        let packed = if actual_linesize == expected_linesize {
            rgba_data.to_vec()
        } else {
            let mut buf = Vec::with_capacity(row_bytes * self.height as usize);
            for row in 0..self.height as usize {
                let offset = row * actual_linesize;
                buf.extend_from_slice(&rgba_data[offset..offset + row_bytes]);
            }
            buf
        };

        let path = self.output_dir.join(format!("frame_{:04}.png", self.frame_count));
        let file = BufWriter::new(File::create(&path).expect("Unable to create PNG file"));
        let mut encoder = Encoder::new(file, self.width, self.height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Fast);
        let mut writer = encoder.write_header().expect("Unable to write PNG header");
        writer.write_image_data(&packed).expect("Unable to write PNG image data");

        log::info!("Saved {}", path.display());
        self.frame_count += 1;
    }

    /// Returns the number of frames written so far.
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for PNGWriter {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if let Some(buf) = frame_data.get_ref(&BufferType::DecodedRGBAFrame) {
            self.write_frame(buf);
        }
        Some(frame_data)
    }
}
