use std::io::Read;

pub struct H264Reader<R: Read> {
    reader: R,
}

impl H264Reader<std::fs::File> {
    pub fn from_file(file_path: &str) -> Self {
        let reader = std::fs::File::open(file_path).expect("Unable to open H264 input file");
        Self { reader }
    }

    pub fn read_all(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        self.reader.read_to_end(&mut data).expect("Error reading H264 file");
        data
    }
}
