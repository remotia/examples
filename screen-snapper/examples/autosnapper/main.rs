use data::{Buffers, RecorderData};
use remotia::{
    buffers::BufferAllocator,
    pipeline::{Pipeline, component::Component},
    processors::ticker::Ticker,
};
use screen_snapper::{
    png_saver::PNGBufferSaver,
    xcap_capturer::{XCapCapturer, xcap_utils},
};
mod data;

#[tokio::main]
async fn main() {
    env_logger::init();

    let monitor_id = 0;
    let (height, width) = xcap_utils::display_size(monitor_id);

    let pipeline = Pipeline::<RecorderData>::new()
        .link(capturer(monitor_id, height, width))
        .link(saver(height, width));

    for handle in pipeline.run() {
        handle
            .await
            .expect(&format!("Error while awaiting for pipeline to finish"));
    }
}

fn capturer(monitor_id: usize, height: u32, width: u32) -> Component<RecorderData> {
    Component::new()
        .append(Ticker::new(1000))
        .append(BufferAllocator::new(
            Buffers::CapturedScreenBuffer,
            height as usize * width as usize * 3,
        ))
        .append(
            XCapCapturer::builder()
                .buffer_key(Buffers::CapturedScreenBuffer)
                .monitor_id(monitor_id)
                .build(),
        )
}

fn saver(height: u32, width: u32) -> Component<RecorderData> {
    Component::new().append(
        PNGBufferSaver::builder()
            .buffer_key(Buffers::CapturedScreenBuffer)
            .path("./screenshots/")
            .height(height)
            .width(width)
            .build(),
    )
}
