mod data;
mod processors;

use remotia::pipeline::{component::Component, Pipeline};

use data::FrameData;
use processors::{BufferingProcessor, Consumer, Producer};

/// # Safe Shutdown Example
///
/// This example demonstrates the remotia pipeline's safe shutdown mechanism,
/// which allows processors to gracefully drain buffered data before the
/// pipeline terminates.
///
/// ## How Safe Shutdown Works
///
/// 1. **`PipelineHandle::request_shutdown()`** — any processor can request
///    that the pipeline shut down. This sets a shared `AtomicBool` that all
///    components check periodically.
///
/// 2. **Drain mode** — when a component's input channel closes (upstream
///    component exited), it doesn't immediately shut down. Instead, it
///    enters drain mode: it feeds `F::default()` frames to its processors
///    and yields to the runtime between iterations to avoid busy-spinning.
///
/// 3. **Shutdown condition** — a component only exits when ALL of the
///    following are true:
///    - Its input channel has closed (drain mode active)
///    - A shutdown signal has been requested
///    - Its processors returned `None` (no more data to produce)
///
///    This ensures that processors with buffered data (like decoders) can
///    fully drain their buffers before the pipeline terminates.
///
/// ## Pipeline Topology
///
/// ```text
/// [Producer] ──► [BufferingProcessor] ──► [Consumer]
///     │                  │                     │
///  Generates        1-to-N expansion        Logs each
///  5 items          (2 items per input)     received item
///  then EOF         buffers all, drains
///  then signals     them one-by-one during
///  shutdown         drain mode
/// ```
#[tokio::main]
async fn main() {
    env_logger::init();

    log::info!("=== Safe Shutdown Example ===");

    let mut pipeline = Pipeline::<FrameData>::new()
        .tag("safe-shutdown")
        .feedable();

    let pipeline_handle = pipeline.get_handle();

    let items: Vec<String> = (1..=5).map(|i| format!("item-{}", i)).collect();

    let producer = Producer::new(items, Some(pipeline_handle.clone()));
    let buffer = BufferingProcessor::new();
    let consumer = Consumer::new("sink");

    let mut pipeline = pipeline
        .link(
            Component::new()
                .append(producer)
                .tag("producer"),
        )
        .link(
            Component::new()
                .append(buffer)
                .tag("buffer"),
        )
        .link(
            Component::new()
                .append(consumer)
                .tag("consumer"),
        );

    let feeder = pipeline.get_feeder();
    let handles = pipeline.run();

    // Seed the pipeline with an initial frame to kick off the producer.
    // The producer ignores its input and generates items from its internal buffer.
    feeder.feed(FrameData::default());

    // Drop the feeder — this closes the producer's input channel,
    // which triggers the cascade: producer exits → buffer's channel closes →
    // buffer enters drain mode → consumer's channel closes → consumer enters drain mode.
    drop(feeder);

    for handle in handles {
        handle.await.unwrap();
    }

    log::info!("=== Pipeline shut down gracefully ===");
}
