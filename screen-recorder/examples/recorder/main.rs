use data::{Buffers, RecorderData};
use remotia::{
    buffers::BufferAllocator,
    pipeline::{Pipeline, component::Component},
    processors::ticker::Ticker,
};
use screen_recorder::{png_saver::PNGBufferSaver, wayshot_capturer::WayshotCapturer};
mod data;

const WIDTH: u32 = 1440;
const HEIGHT: u32 = 900;

#[tokio::main]
async fn main() {
    env_logger::init();

    let monitor_id = 0;

    let pipeline = Pipeline::<RecorderData>::new()
        .link(capturer(monitor_id))
        .link(saver(monitor_id));

    for handle in pipeline.run() {
        handle
            .await
            .expect(&format!("Error while awaiting for pipeline to finish"));
    }
}

fn capturer(monitor_id: usize) -> Component<RecorderData> {
    Component::new()
        .append(Ticker::new(1000))
        .append(BufferAllocator::new(
            Buffers::CapturedScreenBuffer,
            WIDTH as usize * HEIGHT as usize * 3,
        ))
        .append(
            WayshotCapturer::builder()
                .buffer_key(Buffers::CapturedScreenBuffer)
                .build(),
        )
}

fn saver(monitor_id: usize) -> Component<RecorderData> {
    Component::new().append(
        PNGBufferSaver::builder()
            .buffer_key(Buffers::CapturedScreenBuffer)
            .path("./screenshots/")
            .height(HEIGHT)
            .width(WIDTH)
            .build(),
    )
}
