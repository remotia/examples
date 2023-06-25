use remotia::{buffers::BytesMut, traits::{BorrowMutFrameProperties, PullableFrameProperties}};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BufferType {
    YUVBuffer,
    DecodedRGBABuffer,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Pipelines {
    Main,
}

#[derive(Debug, Default)]
pub struct FrameData {
    yuv_buffer: Option<BytesMut>,
    decoded_rgba_buffer: Option<BytesMut>,
}

impl FrameData {
    fn get_buffer_slot(&mut self, key: &BufferType) -> &mut Option<BytesMut> {
        match key {
            BufferType::YUVBuffer => &mut self.yuv_buffer,
            BufferType::DecodedRGBABuffer => &mut self.decoded_rgba_buffer,
        }
    }
}

impl BorrowMutFrameProperties<BufferType, BytesMut> for FrameData {
    fn get_mut_ref(&mut self, key: &BufferType) -> Option<&mut BytesMut> {
        self.get_buffer_slot(key).as_mut()
    }
}

impl PullableFrameProperties<BufferType, BytesMut> for FrameData {
    fn push(&mut self, key: BufferType, value: BytesMut) {
        let _ = self.get_buffer_slot(&key).insert(value);
    }

    fn pull(&mut self, key: &BufferType) -> Option<BytesMut> {
        let buffer = self.get_buffer_slot(&key).take().unwrap();
        Some(buffer)
    }
}
