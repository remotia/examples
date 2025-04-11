use capture::{capturer_processor, fetch_screen_resolution};
use data::{Buffers, SnapperData};
use platform_dependant_screen_snapper::png_saver::PNGBufferSaver;
use remotia::{
    buffers::BufferAllocator,
    pipeline::{Pipeline, component::Component},
    processors::ticker::Ticker,
};

mod capture;
mod data;

#[tokio::main]
async fn main() {
    env_logger::init();

    let (height, width) = fetch_screen_resolution();

    log::debug!("Detected display size: {}x{}", width, height);

    let pipeline = Pipeline::<SnapperData>::new()
        .link(capturer(height, width))
        .link(saver(height, width));

    for handle in pipeline.run() {
        handle
            .await
            .expect(&format!("Error while awaiting for pipeline to finish"));
    }
}

fn capturer(height: u32, width: u32) -> Component<SnapperData> {
    Component::new()
        .append(Ticker::new(1000))
        .append(BufferAllocator::new(
            Buffers::CapturedScreenBuffer,
            height as usize * width as usize * 3,
        ))
        .append(capturer_processor())
}

fn saver(height: u32, width: u32) -> Component<SnapperData> {
    Component::new().append(
        PNGBufferSaver::builder()
            .buffer_key(Buffers::CapturedScreenBuffer)
            .path("./screenshots/")
            .height(height)
            .width(width)
            .build(),
    )
}
