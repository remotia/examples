use async_trait::async_trait;
use bon::Builder;
use image::RgbImage;
use remotia::{
    buffers::BytesMut,
    traits::{FrameProcessor, PullableFrameProperties},
};

#[derive(Builder)]
pub struct PNGBufferSaver<K> {
    #[builder(field = 0)]
    current_id: usize,

    height: usize,
    width: usize,

    buffer_key: K,
    path: &'static str,
}

#[async_trait]
impl<K, F> FrameProcessor<F> for PNGBufferSaver<K>
where
    F: Send + 'static,
    K: Send + Copy,
    F: PullableFrameProperties<K, BytesMut>,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        self.current_id += 1;

        let path = format!("{}/{}.png", self.path, self.current_id);

        let pixels = {
            let buffer = frame_data
                .pull(&self.buffer_key)
                .expect("No screen buffer to pull from DTO");
            let value = buffer.to_vec();
            frame_data.push(self.buffer_key, buffer);
            value
        };

        RgbImage::from_raw(self.width as u32, self.height as u32, pixels)
            .unwrap()
            .save(path)
            .unwrap();

        Some(frame_data)
    }
}
