mod data;
mod processors;

use remotia::pipeline::{component::Component, Pipeline};

use data::FrameData;
use processors::{Consumer, Producer};

#[tokio::main]
async fn main() {
    env_logger::init();

    log::info!("=== Safe Shutdown Example ===");

    let mut pipeline = Pipeline::<FrameData>::new()
        .tag("safe-shutdown");

    let pipeline_handle = pipeline.get_handle();

    let producer = Producer::new(5, pipeline_handle);
    let consumer = Consumer::new();

    let pipeline = pipeline
        .link(
            Component::new()
                .append(producer)
                .tag("producer"),
        )
        .link(
            Component::new()
                .append(consumer)
                .tag("consumer"),
        );

    let handles = pipeline.run();

    for handle in handles {
        handle.await.unwrap();
    }

    log::info!("=== Pipeline shut down gracefully ===");
}
