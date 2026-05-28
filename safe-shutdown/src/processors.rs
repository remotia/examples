use async_trait::async_trait;
use remotia::pipeline::PipelineHandle;
use remotia::traits::FrameProcessor;

use crate::data::FrameData;

pub struct Producer {
    count: usize,
    limit: usize,
    pipeline_handle: PipelineHandle,
}

impl Producer {
    pub fn new(limit: usize, pipeline_handle: PipelineHandle) -> Self {
        Self {
            count: 0,
            limit,
            pipeline_handle,
        }
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for Producer {
    async fn process(&mut self, _frame_data: FrameData) -> Option<FrameData> {
        self.count += 1;
        if self.count <= self.limit {
            log::info!("Produced item {}", self.count);
            Some(FrameData::with_payload(self.count as u128, &format!("item-{}", self.count)))
        } else {
            log::info!("Producer: done, requesting shutdown");
            self.pipeline_handle.request_shutdown();
            None
        }
    }
}

pub struct Consumer;

impl Consumer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FrameProcessor<FrameData> for Consumer {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if let Some(payload) = frame_data.payload() {
            log::info!("Consumed: {}", payload);
            Some(frame_data)
        } else {
            None
        }
    }
}
