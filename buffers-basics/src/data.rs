use remotia::buffers::{BuffersMap, BytesMut, buffers_map};

#[derive(Debug, Default)]
#[buffers_map(current_buffers)]
pub struct FrameData {
    current_buffers: BuffersMap<Buffer>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Buffer {
    Dummy,
}

impl FrameData {
    pub fn print_buffer(self) -> Option<Self> {
        log::info!(
            "Current buffer: {:?}",
            self.current_buffers.get(&Buffer::Dummy)
        );
        Some(self)
    }
}
