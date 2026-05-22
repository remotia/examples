use std::fs::File;
use std::io::Read;

use y4m::Decoder;

pub struct Y4MReader<R: Read> {
    decoder: Decoder<R>,
    width: usize,
    height: usize,
    frame_index: i64,
}

impl Y4MReader<Box<File>> {
    pub fn from_file(file_path: &str) -> Self {
        let reader = Box::new(File::open(file_path).expect("Unable to open Y4M file"));
        let decoder = y4m::decode(reader).expect("Unable to parse Y4M file");
        let width = decoder.get_width();
        let height = decoder.get_height();
        Self {
            decoder,
            width,
            height,
            frame_index: 0,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn read_next_frame(&mut self, rgba_buf: &mut [u8]) -> std::io::Result<bool> {
        let frame = match self.decoder.read_frame() {
            Ok(f) => f,
            Err(_) => return Ok(false),
        };

        let rgba = yuv420_to_rgba(
            frame.get_y_plane(),
            frame.get_u_plane(),
            frame.get_v_plane(),
            self.width,
            self.height,
        );
        rgba_buf.copy_from_slice(&rgba);
        self.frame_index += 1;
        Ok(true)
    }
}

fn yuv420_to_rgba(y_plane: &[u8], u_plane: &[u8], v_plane: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);
    let uv_width = width / 2;

    for row in 0..height {
        for col in 0..width {
            let y_idx = row * width + col;
            let uv_row = row / 2;
            let uv_col = col / 2;
            let uv_idx = uv_row * uv_width + uv_col;

            let y = y_plane[y_idx] as f32;
            let u = u_plane[uv_idx] as f32 - 128.0;
            let v = v_plane[uv_idx] as f32 - 128.0;

            let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
            let g = (y - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
            let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;

            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(255);
        }
    }

    rgba
}
