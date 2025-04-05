use bytes::BytesMut;
use remotia::traits::{BorrowFrameProperties, BorrowMutFrameProperties, PullableFrameProperties};

#[derive(Copy, Clone, Debug)]
pub enum BufferType {
    RawFrameBuffer,
}

#[derive(Default, Debug)]
pub struct FrameData {
    raw_frame_buffer: BytesMut,
}

impl BorrowMutFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_mut_ref(&mut self, key: &BufferType) -> Option<&mut BytesMut> {
        match key {
            BufferType::RawFrameBuffer => Some(&mut self.raw_frame_buffer),
        }
    }
}

impl BorrowFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_ref(&self, key: &BufferType) -> Option<&BytesMut> {
        match key {
            BufferType::RawFrameBuffer => Some(&self.raw_frame_buffer),
        }
    }
}

impl PullableFrameProperties<BufferType, BytesMut> for FrameData {
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
}
