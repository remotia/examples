use std::io::Write;

pub struct H264Writer<W: Write> {
    writer: W,
    pub bytes_written: usize,
}

impl H264Writer<std::fs::File> {
    pub fn from_file(file_path: &str) -> Self {
        let writer = std::fs::File::create(file_path).expect("Unable to create H264 output file");
        Self {
            writer,
            bytes_written: 0,
        }
    }

    pub fn write_packet(&mut self, data: &[u8]) {
        if !data.is_empty() {
            self.writer.write_all(data).expect("Unable to write H264 packet data");
            self.bytes_written += data.len();
        }
    }
}
