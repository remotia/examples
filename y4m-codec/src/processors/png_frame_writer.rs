use std::fs;
use std::path::PathBuf;

use image::RgbaImage;

pub struct PNGWriter {
    output_dir: PathBuf,
    width: u32,
    height: u32,
    frame_count: u32,
}

impl PNGWriter {
    pub fn new(output_dir: PathBuf, width: u32, height: u32) -> Self {
        fs::create_dir_all(&output_dir).expect("Unable to create output directory");
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

        let pixels = rgba_data.to_vec();
        let image = RgbaImage::from_raw(self.width, self.height, pixels)
            .expect("Unable to create RGBA image from buffer");

        let path = self.output_dir.join(format!("frame_{:04}.png", self.frame_count));
        image.save(&path).expect("Unable to save PNG file");

        log::info!("Saved {}", path.display());
        self.frame_count += 1;
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
}
