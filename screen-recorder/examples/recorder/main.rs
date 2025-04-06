use data::{Buffers, RecorderData};
use remotia::{
    buffers::BufferAllocator,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{Pipeline, component::Component},
    processors::{functional::Function, ticker::Ticker},
};
use screen_recorder::png_saver::PNGBufferSaver;
mod data;

struct DisplaySize {
    height: usize,
    width: usize,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let (capturer, display_size) = capturer();
    let saver = saver(&display_size);

    let pipeline = Pipeline::<RecorderData>::new().link(capturer).link(saver);

    for handle in pipeline.run() {
        handle
            .await
            .expect(&format!("Error while awaiting for pipeline to finish"));
    }
}

fn capturer() -> (Component<RecorderData>, DisplaySize) {
    let mut capturer = ScrapFrameCapturer::new_from_primary(Buffers::CapturedScreenBuffer);

    let display_size = DisplaySize {
        height: capturer.height(),
        width: capturer.width(),
    };

    let component = Component::new()
        .append(Ticker::new(1000))
        .append(BufferAllocator::new(
            Buffers::CapturedScreenBuffer,
            capturer.buffer_size(),
        ))
        .append(capturer)
        .append(Function::new(|frame_data: RecorderData| {
            let sum = frame_data
                .screen_buffer
                .clone()
                .unwrap()
                .iter()
                .map(|value| *value as usize)
                .sum::<usize>();

            log::info!("Buffer sum: {}", sum);
            Some(frame_data)
        }));

    (component, display_size)
}

fn saver(display_size: &DisplaySize) -> Component<RecorderData> {
    Component::new().append(
        PNGBufferSaver::builder()
            .buffer_key(Buffers::CapturedScreenBuffer)
            .path("./screenshots/")
            .height(display_size.height)
            .width(display_size.width)
            .build(),
    )
}
