use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use png::{BitDepth, ColorType, Compression, Encoder};

pub struct PNGWriter {
    output_dir: PathBuf,
    width: u32,
    height: u32,
    frame_count: u32,
}

impl PNGWriter {
    pub fn new(output_dir: PathBuf, width: u32, height: u32) -> Self {
        std::fs::create_dir_all(&output_dir).expect("Unable to create output directory");
        Self {
            output_dir,
            width,
            height,
            frame_count: 0,
        }
    }

    pub fn write_frame(&mut self, rgba_data: &[u8]) {
        if rgba_data.is_empty() {
            return;
        }

        let path = self.output_dir.join(format!("frame_{:04}.png", self.frame_count));
        let file = BufWriter::new(File::create(&path).expect("Unable to create PNG file"));
        let mut encoder = Encoder::new(file, self.width, self.height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Fast);
        let mut writer = encoder.write_header().expect("Unable to write PNG header");
        writer.write_image_data(rgba_data).expect("Unable to write PNG image data");

        log::info!("Saved {}", path.display());
        self.frame_count += 1;
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
}
