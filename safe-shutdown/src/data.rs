use std::collections::HashMap;

use remotia::traits::FrameProperties;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stat {
    FrameId,
}

#[derive(Default, Debug)]
pub struct FrameData {
    stats: HashMap<Stat, u128>,
    payload: Option<String>,
}

impl FrameProperties<Stat, u128> for FrameData {
    fn set(&mut self, key: Stat, value: u128) {
        self.stats.insert(key, value);
    }

    fn get(&self, key: &Stat) -> Option<u128> {
        self.stats.get(key).copied()
    }
}

impl FrameData {
    pub fn with_payload(id: u128, payload: &str) -> Self {
        let mut fd = Self::default();
        fd.set(Stat::FrameId, id);
        fd.payload = Some(payload.to_string());
        fd
    }

    pub fn payload(&self) -> Option<&str> {
        self.payload.as_deref()
    }
}
