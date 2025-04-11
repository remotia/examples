use async_trait::async_trait;
use bon::Builder;
use libwayshot::WayshotConnection;
use libwayshot_image::DynamicImage;
use remotia::{
    buffers::{BufMut, BytesMut},
    traits::{FrameProcessor, PullableFrameProperties},
};

#[derive(Builder)]
pub struct WayshotCapturer<K> {
    buffer_key: K,
}

pub mod wayshot_utils {
    use super::*;
    pub fn display_size() -> (u32, u32) {
        let wayshot_connection = WayshotConnection::new().unwrap();
        let dimensions = &wayshot_connection
            .get_all_outputs()
            .first()
            .unwrap()
            .dimensions;

        (dimensions.height as u32, dimensions.width as u32)
    }
}

#[async_trait]
impl<K, F> FrameProcessor<F> for WayshotCapturer<K>
where
    F: Send + 'static,
    K: Send + Copy,
    F: PullableFrameProperties<K, BytesMut>,
{
    async fn process(&mut self, mut dto: F) -> Option<F> {
        // Capture screen data
        log::debug!("Capturing screen data...");
        let wayshot_connection = WayshotConnection::new().unwrap();
        let rgba_image = wayshot_connection.screenshot_all(false).unwrap();

        // Remove the alpha channel
        log::debug!("Removing alpha channel...");
        let rgb_image = DynamicImage::ImageRgba8(rgba_image).into_rgb8();

        // Write data into the DTO buffer
        log::debug!("Writing data to DTO...");
        let mut buffer = dto
            .pull(&self.buffer_key)
            .expect("No buffer to pull from frame data");

        buffer.clear();
        log::debug!("Buffer len before write: {}", buffer.len());
        buffer.put_slice(rgb_image.as_raw());
        log::debug!("Buffer len after write: {}", buffer.len());

        dto.push(self.buffer_key, buffer);

        // Return the filled DTO
        log::debug!("Done");
        Some(dto)
    }
}
