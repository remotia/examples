use std::fmt::Debug;

use derive_new::new;
use itertools::izip;
use rand::RngCore;
use remotia::{
    buffers::{BufMut, BytesMut},
    sync_processor::SyncFrameProcessor,
    traits::PullableFrameProperties,
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

pub struct DeltaEncoder<K> {
    values_buffer_key: K,
    target_buffer_key: K,
    last_values: BytesMut,
}

impl<K> DeltaEncoder<K> {
    pub fn new(source_buffer_key: K, target_buffer_key: K, len: usize) -> Self {
        Self {
            values_buffer_key: source_buffer_key,
            target_buffer_key,
            last_values: BytesMut::zeroed(len),
        }
    }
}

impl<K, F> SyncFrameProcessor<F> for DeltaEncoder<K>
where
    K: Clone + Copy + Debug,
    F: Send + 'static,
    F: PullableFrameProperties<K, BytesMut>,
{
    fn process(&mut self, mut frame_data: F) -> Option<F> {
        let values_buffer = frame_data
            .pull(&self.values_buffer_key)
            .expect("Missing expected values buffer");

        let mut target_buffer = frame_data
            .pull(&self.target_buffer_key)
            .expect("Missing expected target buffer");

        log::debug!("Performing delta encoding");
        log::debug!("Current: {:?}", &values_buffer[..]);
        log::debug!("Last: {:?}", &self.last_values[..]);

        for (current, last) in izip!(values_buffer.iter(), self.last_values.iter()) {
            let delta_value = {
                // let current = *current as i32;
                // let last = *last as i32;
                // (current - last).clamp(0, 255) as u8
                current - last
            };
            target_buffer.put_u8(delta_value);
        }

        self.last_values.clear();
        self.last_values.put_slice(&values_buffer);

        frame_data.push(self.values_buffer_key, values_buffer);
        frame_data.push(self.target_buffer_key, target_buffer);

        Some(frame_data)
    }
}
