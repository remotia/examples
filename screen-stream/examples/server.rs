use clap::Parser;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{component::Component, Pipeline},
    processors::{error_switch::OnErrorSwitch, functional::Function, ticker::Ticker}, traits::BorrowFrameProperties,
};
use remotia_ffmpeg_codecs::{
    encoders::EncoderBuilder, ffi, options::Options, scaling::ScalerBuilder,
};

use remotia_srt::{srt_tokio::SrtSocket, sender::SRTFrameSender};
use screen_stream::types::{BufferType::*, FrameData};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 60)]
    framerate: u64,

    #[arg(long, default_value_t = 9000)]
    port: u16,
}

const POOLS_SIZE: usize = 1;

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World! I will mirror your screen encoding it using the H264 codec.");

    let args = Args::parse();

    let capturer = ScrapFrameCapturer::new_from_primary(CapturedRGBAFrameBuffer);

    log::info!("Streaming at {}x{}", capturer.width(), capturer.height());

    let width = capturer.width() as usize;
    let height = capturer.height() as usize;
    let pixels_count = width * height;
    let mut registry = PoolRegistry::new();
    registry
        .register(CapturedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;
    registry
        .register(EncodedFrameBuffer, POOLS_SIZE, pixels_count)
        .await;

    let (encoder_pusher, encoder_puller) = EncoderBuilder::new()
        .codec_id("libx264")
        .rgba_buffer_key(CapturedRGBAFrameBuffer)
        .encoded_buffer_key(EncodedFrameBuffer)
        .scaler(
            ScalerBuilder::new()
                .input_width(width as i32)
                .input_height(height as i32)
                .input_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_RGBA)
                .output_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P)
                .build(),
        )
        .options(Options::new()
            .set("crf", "26")
            .set("tune", "zerolatency")
            .set("keyint", "1"))
        .build();

    let mut error_pipeline = Pipeline::<FrameData>::singleton(
        Component::new()
            .append(Function::new(|fd| {
                log::warn!("Dropped frame");
                Some(fd)
            }))
            .append(registry.get(CapturedRGBAFrameBuffer).redeemer().soft())
            .append(registry.get(EncodedFrameBuffer).redeemer().soft()),
    )
    .feedable();

    log::info!("Waiting for connection...");
    let socket = SrtSocket::builder().listen_on(args.port).await.unwrap();

    let pipeline = Pipeline::<FrameData>::new().link(
        Component::new()
            .append(Ticker::new(1000 / args.framerate))
            .append(registry.get(CapturedRGBAFrameBuffer).borrower())
            .append(capturer)
            .append(encoder_pusher)
            .append(registry.get(CapturedRGBAFrameBuffer).redeemer())
            .append(registry.get(EncodedFrameBuffer).borrower())
            .append(encoder_puller)
            .append(OnErrorSwitch::new(&mut error_pipeline))
            .append(SRTFrameSender::new(EncodedFrameBuffer, socket))
            .append(registry.get(EncodedFrameBuffer).redeemer()),
    );

    let mut handles = Vec::new();
    handles.extend(error_pipeline.run());
    handles.extend(pipeline.run());

    for handle in handles {
        handle.await.unwrap();
    }
}
