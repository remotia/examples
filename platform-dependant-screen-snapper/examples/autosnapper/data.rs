use remotia::{
    buffers::BytesMut,
    traits::{BorrowMutFrameProperties, PullableFrameProperties},
};

#[derive(Default, Debug)]
pub struct SnapperData {
    pub(crate) screen_buffer: Option<BytesMut>,
}

#[derive(Clone, Copy)]
pub enum Buffers {
    CapturedScreenBuffer,
}

impl BorrowMutFrameProperties<Buffers, BytesMut> for SnapperData {
    fn get_mut_ref(&mut self, key: &Buffers) -> Option<&mut BytesMut> {
        match key {
            Buffers::CapturedScreenBuffer => self.screen_buffer.as_mut(),
        }
    }
}

impl PullableFrameProperties<Buffers, BytesMut> for SnapperData {
    fn push(&mut self, key: Buffers, value: BytesMut) {
        match key {
            Buffers::CapturedScreenBuffer => self.screen_buffer.replace(value),
        };
    }

    fn pull(&mut self, key: &Buffers) -> Option<BytesMut> {
        match key {
            Buffers::CapturedScreenBuffer => self.screen_buffer.take(),
        }
    }
}
