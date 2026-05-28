use async_trait::async_trait;
use remotia::pipeline::PipelineHandle;
use remotia::traits::FrameProcessor;

use crate::data::FrameData;

/// A producer that generates items from an internal buffer.
///
/// On each invocation it yields one item. When the internal buffer is
/// exhausted, it signals shutdown through the pipeline handle and returns
/// `None` so the component can shut down.
pub struct Producer {
    items: Vec<String>,
    index: usize,
    pipeline_handle: Option<PipelineHandle>,
}

impl Producer {
    pub fn new(items: Vec<String>, pipeline_handle: Option<PipelineHandle>) -> Self {
        Self {
            items,
            index: 0,
            pipeline_handle,
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for Producer {
    async fn process(&mut self, _frame_data: FrameData) -> Option<FrameData> {
        if self.index < self.items.len() {
            self.index += 1;
            let item = &self.items[self.index - 1];
            log::info!("Produced: {}", item);
            Some(FrameData::with_payload(self.index as u128, item))
        } else {
            log::info!("Producer: all items emitted, requesting shutdown");
            if let Some(handle) = &self.pipeline_handle {
                handle.request_shutdown();
            }
            None
        }
    }
}

/// A consumer that logs each item it receives.
///
/// When the input has no payload (drain mode with empty default frames),
/// it returns `None`, allowing the component to check the shutdown condition.
pub struct Consumer {
    label: String,
}

impl Consumer {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for Consumer {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if let Some(payload) = frame_data.payload() {
            log::info!("{} consumed: {}", self.label, payload);
            Some(frame_data)
        } else {
            None
        }
    }
}

/// A buffering processor that demonstrates the drain-mode mechanism.
///
/// This processor simulates a one-to-many relationship between input and
/// output frames — the same pattern as an FFmpeg decoder where a single
/// input chunk can produce multiple decoded frames.
///
/// **How it works:**
/// - For each input item, it buffers two expanded items (e.g. `"foo"` →
///   `"foo-alpha"` and `"foo-beta"`)
/// - Each `process()` call yields ONE buffered item. If no items are
///   buffered, it returns `None` (signaling "no output for now")
/// - When the upstream channel closes (drain mode), the component feeds
///   `F::default()` frames. The processor keeps yielding buffered items
///   one by one until the buffer is empty
/// - Once the buffer is empty AND the shutdown signal is active, the
///   component's exit condition triggers
///
/// This ensures that **all buffered items are drained before shutdown**,
/// which is the core guarantee of the safe shutdown mechanism.
pub struct BufferingProcessor {
    buffer: Vec<String>,
}

impl BufferingProcessor {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for BufferingProcessor {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if let Some(payload) = frame_data.payload() {
            self.buffer.push(format!("{}-alpha", payload));
            self.buffer.push(format!("{}-beta", payload));
            log::debug!(
                "BufferingProcessor: buffered 2 items from '{}' (total: {})",
                payload,
                self.buffer.len()
            );
        }

        if let Some(item) = self.buffer.pop() {
            let mut fd = FrameData::default();
            fd.set_payload(item);
            Some(fd)
        } else {
            None
        }
    }
}
