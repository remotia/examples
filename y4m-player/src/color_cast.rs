use async_trait::async_trait;
use remotia::{traits::{FrameProcessor, PullableFrameProperties}, buffers::BufMut};

use crate::types::{FrameData, BufferType::*};

pub struct ColorCaster {
    width: usize,
    height: usize,
}

impl ColorCaster {
    pub fn new(width: u32, height: u32) -> Self {
        Self { 
            width: width as usize, 
            height: height as usize
        }
    }
}

fn yuv_to_rgb((y, u, v): (u8, u8, u8)) -> (u8, u8, u8) {
    let y = y as f32;
    let u = u as f32;
    let v = v as f32;

    (
        (y + 1.14 * v) as u8,
        (y + -0.396 * u - 0.581 * v) as u8,
        (y + 2.029 * u) as u8
    )
}

#[async_trait]
impl FrameProcessor<FrameData> for ColorCaster {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let mut rgba_buffer = frame_data.pull(&DecodedRGBABuffer).unwrap();
        let yuv_buffer = frame_data.pull(&YUVBuffer).unwrap();

        let y_buffer = &yuv_buffer[0..yuv_buffer.len()/2];
        let u_buffer = &yuv_buffer[y_buffer.len()..y_buffer.len() + yuv_buffer.len()/4];
        let v_buffer = &yuv_buffer[(y_buffer.len() + u_buffer.len())..(y_buffer.len() + u_buffer.len()) + yuv_buffer.len()/4];

        for w in 0..self.width {
            for h in 0..self.height {
                let y = y_buffer[h * self.width + w];
                let u = u_buffer[h/2 * self.width + w/2];
                let v = v_buffer[h/2 * self.width + w/2];

                let (r, g, b) = yuv_to_rgb((y, u, v));
                rgba_buffer.put_u8(r);
                rgba_buffer.put_u8(g);
                rgba_buffer.put_u8(b);
                rgba_buffer.put_u8(255);
            }
        }

        frame_data.push(DecodedRGBABuffer, rgba_buffer);
        frame_data.push(YUVBuffer, yuv_buffer);
        Some(frame_data)
    }
}
