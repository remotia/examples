use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::PathBuf;

use async_trait::async_trait;
use remotia::{
    buffers::BytesMut,
    traits::{BorrowMutFrameProperties, FrameProcessor},
};

pub struct PngRenderer<K> {
    buffer_key: K,
    output_dir: PathBuf,
    frame_count: u32,

    width: u32,
    height: u32,
}

impl<K> PngRenderer<K> {
    pub fn new(buffer_key: K, output_dir: PathBuf, (width, height): (u32, u32)) -> Self {
        fs::create_dir_all(&output_dir).unwrap();
        Self {
            buffer_key,
            output_dir,
            width,
            height,
            frame_count: 0,
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for PngRenderer<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        log::debug!("Saving frame as PNG...");

        let raw_frame_buffer = frame_data.get_mut_ref(&self.buffer_key).unwrap();

        log::debug!("Head: {:?}...", &raw_frame_buffer[0..64]);

        let image_buffer: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(self.width, self.height, raw_frame_buffer.to_vec()).unwrap();
        let file_path = self
            .output_dir
            .join(format!("frame_{}.png", self.frame_count));
        image_buffer.save(file_path).unwrap();

        self.frame_count += 1;
        Some(frame_data)
    }
}
