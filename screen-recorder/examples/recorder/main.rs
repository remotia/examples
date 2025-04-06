use data::{Buffers, RecorderData};
use remotia::{
    buffers::BufferAllocator,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{Pipeline, component::Component},
    processors::ticker::Ticker,
};
mod data;

#[tokio::main]
async fn main() {
    let pipeline = Pipeline::<RecorderData>::new().link(capturer());

    for handle in pipeline.run() {
        handle
            .await
            .expect(&format!("Error while awaiting for pipeline to finish"));
    }
}

fn capturer() -> Component<RecorderData> {
    let mut capturer = ScrapFrameCapturer::new_from_primary(Buffers::CapturedScreenBuffer);

    Component::new()
        .append(Ticker::new(1000))
        .append(BufferAllocator::new(
            Buffers::CapturedScreenBuffer,
            capturer.buffer_size(),
        ))
        .append(capturer)
}
