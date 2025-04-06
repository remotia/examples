use async_trait::async_trait;
use bon::Builder;
use image::DynamicImage;
use remotia::{
    buffers::{BufMut, BytesMut},
    traits::{FrameProcessor, PullableFrameProperties},
};
use xcap::Monitor;

#[derive(Builder)]
pub struct XCapCapturer<K> {
    buffer_key: K,
    monitor_id: usize,
}

pub mod xcap_utils {
    use super::*;

    pub fn fetch_monitor_by_id(monitor_id: usize) -> Monitor {
        Monitor::all()
            .expect("Unable to fetch monitors")
            .remove(monitor_id)
    }

    pub fn display_size(monitor_id: usize) -> (u32, u32) {
        let monitor = fetch_monitor_by_id(monitor_id);
        (
            monitor.height().expect("Unable to fetch monitor's height"),
            monitor.width().expect("Unable to fetch monitor's width"),
        )
    }

    pub fn expected_buffer_size_for_monitor(monitor_id: usize) -> usize {
        let (height, width) = display_size(monitor_id);
        height as usize * width as usize * 3
    }
}

#[async_trait]
impl<K, F> FrameProcessor<F> for XCapCapturer<K>
where
    F: Send + 'static,
    K: Send + Copy,
    F: PullableFrameProperties<K, BytesMut>,
{
    async fn process(&mut self, mut dto: F) -> Option<F> {
        // Capture screen data
        log::debug!("Capturing screen data...");
        let rgba_image = xcap_utils::fetch_monitor_by_id(self.monitor_id)
            .capture_image()
            .expect("Unable to capture screen buffer to image");

        // Remove the alpha channel
        log::debug!("Removing alpha channel...");
        let rgb_image = DynamicImage::ImageRgba8(rgba_image).into_rgb8();

        // Write data into the DTO buffer
        log::debug!("Writing data to DTO...");
        let mut buffer = dto
            .pull(&self.buffer_key)
            .expect("No buffer to pull from frame data");

        buffer.clear();
        buffer.put_slice(rgb_image.as_raw());

        dto.push(self.buffer_key, buffer);

        // Return the filled DTO
        log::debug!("Done");
        Some(dto)
    }
}
