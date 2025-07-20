use remotia::buffers::{BuffersMap, BytesMut, buffers_map};

#[derive(Debug, Default)]
#[buffers_map(buffers)]
pub struct FrameData {
    buffers: BuffersMap<Buffer>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Buffer {
    Full,
    Delta,
}

impl FrameData {
    pub fn print_buffers(&self) {
        for (key, buffer) in &self.buffers {
            log::info!("-- Buffers --");
            log::info!("{:?}: {:?}", key, &buffer[..]);
            log::info!("-------------");
        }
    }
}
