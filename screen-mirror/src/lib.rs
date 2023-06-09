use bytes::BytesMut;
use remotia::traits::BorrowableFrameProperties;

#[derive(Copy, Clone, Debug)]
pub enum BufferType {
    RawFrameBuffer,
}

#[derive(Default, Debug)]
pub struct FrameData {
    raw_frame_buffer: BytesMut
}

impl BorrowableFrameProperties<BufferType, BytesMut> for FrameData {
    fn push(&mut self, key: BufferType, value: BytesMut) {
        match key {
            BufferType::RawFrameBuffer => self.raw_frame_buffer = value,
        }
    }

    fn pull(&mut self, key: &BufferType) -> Option<BytesMut> {
        match key {
            BufferType::RawFrameBuffer => Some(self.raw_frame_buffer.clone()),
        }
    }

    fn get_ref(&self, key: &BufferType) -> Option<&BytesMut> {
        match key {
            BufferType::RawFrameBuffer => Some(&self.raw_frame_buffer),
        }
    }

    fn get_mut_ref(&mut self, key: &BufferType) -> Option<&mut BytesMut> {
        match key {
            BufferType::RawFrameBuffer => Some(&mut self.raw_frame_buffer),
        }
    }
}
