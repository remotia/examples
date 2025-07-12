use remotia::{
    buffers::{BuffersMap, BytesMut, buffers_map},
    pipeline::component::Component,
    processors::functional::{ClosureAppends, Function},
};

#[derive(Debug, Default)]
#[buffers_map(current_buffers)]
pub struct FrameData {
    current_buffers: BuffersMap<Buffer>,
    phase: Phase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Unknown,
    PreBorrow,
    PostBorrow,
    PreRedeem,
    PostRedeem,
}

impl Default for Phase {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Buffer {
    Dummy,
}

pub trait FrameDataAppends {
    fn print_buffer(self) -> Self;
    fn set_phase(self, phase: Phase) -> Self;
}

impl FrameDataAppends for Component<FrameData> {
    fn print_buffer(self) -> Self {
        self.append(Function::new(|frame_data: FrameData| {
            log::info!(
                "[{:?}] Current buffer: {:?}",
                frame_data.phase,
                frame_data.current_buffers.get(&Buffer::Dummy)
            );
            Some(frame_data)
        }))
    }

    fn set_phase(self, phase: Phase) -> Self {
        self.closure(move |mut frame_data: FrameData| {
            frame_data.phase = phase;
            Some(frame_data)
        })
    }
}
