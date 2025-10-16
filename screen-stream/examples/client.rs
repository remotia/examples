use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use clap::Parser;
use image::ImageReader;
use remotia::buffers::BufMut;
use remotia::buffers::BytesMut;
use remotia::pipeline::registry::PipelineRegistry;
use remotia::profilation::loggers::console::ConsoleAverageStatsLogger;
use remotia::profilation::time::add::TimestampAdder;
use remotia::register;
use remotia::traits::FrameError;
use remotia::traits::FrameProcessor;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    pipeline::{component::Component, Pipeline},
    processors::{error_switch::OnErrorSwitch, functional::Function},
    profilation::time::diff::TimestampDiffCalculator,
    render::winit::WinitRenderer,
};
use remotia_ffmpeg_codecs::{decoders::DecoderBuilder, ffi, scaling::ScalerBuilder};
use remotia_srt::options::ByteCount;
use remotia_srt::receiver::SRTFrameReceiver;

use remotia::traits::FrameProperties;

use remotia_srt::SrtSocket;
use screen_stream::renderers::png::PngRenderer;
use screen_stream::types::BufferType;
use screen_stream::types::Stat;
use screen_stream::types::{BufferType::*, FrameData, Stat::*};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    width: u32,

    #[arg(long)]
    height: u32,

    #[arg(long, default_value_t=String::from("127.0.0.1:9000"))]
    server_address: String,

    #[arg(long, default_value_t=String::from("h264"))]
    codec_id: String,
}

const POOLS_SIZE: usize = 1;

#[derive(PartialEq, Eq, Hash)]
enum Pipelines {
    Main,
    Error,
}

// fn load_png_to_bytesmut(file_path: &str) -> BytesMut {
//     // Open the file
//     let file = File::open(file_path).unwrap();
//     let reader = BufReader::new(file);

//     // Decode the image
//     let img = ImageReader::new(reader)
//         .with_guessed_format().unwrap()
//         .decode().unwrap();

//     // Convert the image to RGBA format
//     let rgba_img = img.to_rgba8();

//     // Get the raw pixel data
//     let pixels = rgba_img.into_raw();

//     // Create a BytesMut buffer and extend it with the pixel data
//     let mut bytes_mut = BytesMut::new();
//     bytes_mut.extend_from_slice(&pixels);

//     bytes_mut
// }

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World!");

    let args = Args::parse();

    log::info!("Streaming at {}x{}", args.width, args.height);
    let mut renderer = WinitRenderer::new(DecodedRGBAFrameBuffer);
    let render_runner = renderer.allocate(args.width, args.height);
    let gui_handle = tokio::spawn(async move {
        render_runner.start();
    });

    // let renderer = PngRenderer::new(
    //     DecodedRGBAFrameBuffer,
    //     ".local/test/".into(),
    //     (args.width, args.height)
    // );
    
    // let mut i = 0;
    // loop {
    //     i += 1;

    //     let mut frame_data = FrameData::default();
    //     frame_data.set(Stat::CaptureTime, i);
    //     // let mut buffer = BytesMut::zeroed((args.width * args.height * 4) as usize);
    //     // buffer.fill((i * 10 % 256) as u8);
    //     let buffer = load_png_to_bytesmut(".local/screen.png");
    //     frame_data
    //         .buffers
    //         .insert(BufferType::DecodedRGBAFrameBuffer, buffer);
    //     renderer.process(frame_data).await;

    //     tokio::time::sleep(Duration::from_millis(33)).await;
    // }

    let pixels_count = (args.width * args.height) as usize;
    let mut pools = PoolRegistry::new();

    pools
        .register(EncodedPacketBuffer, POOLS_SIZE, pixels_count * 4)
        .await;
    pools
        .register(DecodedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;

    let (decoder_pusher, decoder_puller) = DecoderBuilder::new()
        .codec_id(&args.codec_id)
        .scaler(
            ScalerBuilder::new()
                .input_width(args.width as i32)
                .input_height(args.height as i32)
                .input_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P)
                .output_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_BGRA)
                .build(),
        )
        .build();

    let mut pipelines = PipelineRegistry::<FrameData, Pipelines>::new();

    register!(
        pipelines,
        Pipelines::Error,
        Pipeline::<FrameData>::singleton(
            Component::new()
                .append(Function::new(|fd: FrameData| {
                    log::warn!("Dropped frame: {:?}", fd.get_error());
                    Some(fd)
                }))
                .append(pools.get(DecodedRGBAFrameBuffer).redeemer().soft()),
        )
        .feedable()
    );

    log::info!("Connecting...");
    let socket = SrtSocket::builder()
        .set(|options| options.receiver.buffer_size = ByteCount(10 * 1024 * 1024))
        .call(args.server_address.as_str(), None)
        .await
        .unwrap();

    register!(
        pipelines,
        Pipelines::Main,
        Pipeline::<FrameData>::new()
            .link(
                Component::new()
                    .append(pools.get(EncodedPacketBuffer).borrower())
                    .append(SRTFrameReceiver::from_socket(socket))
                    // .append(TimestampDiffCalculator::new(CaptureTime, ReceptionDelay))
                    .append(TimestampAdder::new(DecodePushTime))
                    .append(decoder_pusher)
                    .append(pools.get(EncodedPacketBuffer).redeemer())
                    .append(OnErrorSwitch::new(pipelines.get_mut(&Pipelines::Error))),
            )
            .link(
                Component::new()
                    .append(pools.get(DecodedRGBAFrameBuffer).borrower())
                    .append(decoder_puller)
                    .append(OnErrorSwitch::new(pipelines.get_mut(&Pipelines::Error)))
                    // .append(TimestampDiffCalculator::new(DecodePushTime, DecodeTime))
                    .append(renderer)
                    // .append(TimestampDiffCalculator::new(CaptureTime, FrameDelay))
                    .append(pools.get(DecodedRGBAFrameBuffer).redeemer()),
            )
            .link(
                Component::new().append(
                    ConsoleAverageStatsLogger::new()
                        .header("Statistics")
                        .log(ReceptionDelay)
                ),
            )
    );

    pipelines.run().await;
    gui_handle.await.unwrap();
}
