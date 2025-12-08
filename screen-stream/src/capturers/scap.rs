use std::sync::mpsc::RecvError;

use async_trait::async_trait;
use log::debug;
use remotia::{
    buffers::{BufMut, BytesMut},
    traits::{BorrowMutFrameProperties, FrameProcessor},
};
use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::{Frame, FrameType},
};
use tokio::sync::watch;

pub struct ScapFrameCapturer<K> {
    buffer_key: K,
    capturer: Capturer,
    started: bool,
}

impl<K> ScapFrameCapturer<K> {
    pub fn new(buffer_key: K, capturer: Capturer) -> Self {
        Self {
            buffer_key,
            capturer,
            started: false,
        }
    }

    pub fn new_from_primary(fps: u32, buffer_key: K) -> Self {
        let capturer = Capturer::build(Options {
            fps,
            show_cursor: true,
            show_highlight: true,
            target: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: Resolution::Captured,
            ..Default::default()
        })
        .unwrap();

        Self::new(buffer_key, capturer)
    }

    pub fn start(&mut self) {
        self.capturer.start_capture();
    }

    pub fn resolution(&mut self) -> (u32, u32) {
        let frame_size = self.capturer.get_output_frame_size();
        (frame_size[0], frame_size[1])
    }

    pub fn buffer_size(&mut self) -> usize {
        let frame_size = self.capturer.get_output_frame_size();
        let buffer_size = frame_size[0] * frame_size[1] * 4;
        buffer_size as usize
    }

    pub fn next_frame(&mut self) -> Result<Frame, RecvError> {
        self.capturer.get_next_frame()
    }

    pub fn capturer(&mut self) -> &mut Capturer {
        &mut self.capturer
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for ScapFrameCapturer<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        debug!("Capturing...");
        if !self.started {
            self.capturer.start_capture();
            self.started = true;
        }

        let captured_frame = self.next_frame();
        let output_buffer = match frame_data.get_mut_ref(&self.buffer_key) {
            Some(buffer) => buffer,
            None => {
                log::warn!("Dumping captured frame because no buffer is available");
                return None;
            }
        };

        match captured_frame {
            Ok(frame) => {
                let buffer = match frame {
                    scap::frame::Frame::BGRx(bgraframe) => bgraframe,
                    _ => panic!("Unexpected frame format"),
                };
                output_buffer.put(buffer.data.as_slice());
                log::debug!("Written buffer: {:?}", &output_buffer[0..64]);
            }
            Err(error) => {
                panic!("Scap capture error: {}", error);
            }
        }
        Some(frame_data)
    }
}
