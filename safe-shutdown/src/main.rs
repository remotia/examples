mod data;
mod processors;

use remotia::pipeline::{component::Component, Pipeline, registry::PipelineRegistry};
use remotia::register;

use data::FrameData;
use processors::{Consumer, Producer};

#[derive(Eq, Hash, PartialEq, Clone)]
enum PipelineId {
    Main,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    log::info!("=== Safe Shutdown Example ===");

    let mut registry = PipelineRegistry::<FrameData, PipelineId>::new();

    register!(
        registry,
        PipelineId::Main,
        Pipeline::<FrameData>::new()
            .tag("safe-shutdown")
            .link(
                Component::new()
                    .tag("producer")
                    .append(Producer::new(5, registry.lazy_handle(PipelineId::Main)))
            )
            .link(
                Component::new()
                    .tag("consumer")
                    .append(Consumer::new())
            )
    );

    registry.run().await;

    log::info!("=== Pipeline shut down gracefully ===");
}
