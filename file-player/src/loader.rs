use std::{fs::File, io::Read};

use async_trait::async_trait;
use remotia::traits::FrameProcessor;
use y4m::Decoder;

pub struct Y4MLoader<R: Read> {
    decoder: Decoder<R>,
}

impl Y4MLoader<Box<File>> {
    pub fn from_file(file_path: &str) -> Self {
        let reader = Box::new(File::open(file_path).unwrap());
        let decoder = y4m::decode(reader).unwrap();
        Self { decoder }
    }
}

#[async_trait]
impl<R, F> FrameProcessor<F> for Y4MLoader<R>
where
    R: Read + Send,
    F: Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let (width, height) = (self.decoder.get_width(), self.decoder.get_height());
        match self.decoder.read_frame() {
            Ok(frame) => {
                let y_plane = frame.get_y_plane();
                let u_plane = frame.get_u_plane();
                let v_plane = frame.get_v_plane();
                let rgba = yuv_to_rgba(y_plane, u_plane, v_plane, width, height);

                Some(frame_data)
            }
            _ => None,
        }
    }
}

fn yuv_to_rgba(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);

    for i in 0..width * height {
        let y = y_plane[i] as f32;
        let u = u_plane[i] as f32 - 128.0;
        let v = v_plane[i] as f32 - 128.0;

        // Convert YUV to RGB
        let r = y + 1.402 * v;
        let g = y - 0.344 * u - 0.714 * v;
        let b = y + 1.772 * u;

        // Clamp the values to the range [0, 255]
        let r = r.clamp(0.0, 255.0) as u8;
        let g = g.clamp(0.0, 255.0) as u8;
        let b = b.clamp(0.0, 255.0) as u8;

        // Push RGBA values to the vector
        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(255); // Alpha value (fully opaque)
    }

    rgba
}
