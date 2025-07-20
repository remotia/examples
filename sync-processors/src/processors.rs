use std::fmt::Debug;

use derive_new::new;
use rand::{RngCore};
use remotia::{
    buffers::{BufMut, BytesMut}, sync_processor::SyncFrameProcessor, traits::PullableFrameProperties,
};

#[derive(new)]
pub struct RandomGenerator<K> {
    buffer_key: K,
}

impl<K, F> SyncFrameProcessor<F> for RandomGenerator<K>
where
    K: Clone + Copy + Debug,
    F: Send + 'static,
    F: PullableFrameProperties<K, BytesMut>,
{
    fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut buffer = frame_data
            .pull(&self.buffer_key)
            .expect("Missing expected buffer");

        buffer.put_bytes(0, buffer.capacity());

        rand::rng().fill_bytes(&mut buffer);

        frame_data.push(self.buffer_key, buffer);
        Some(frame_data)
    }
}
