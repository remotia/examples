use clap::Parser;
use remotia::profilation::loggers::console::ConsoleAverageStatsLogger;
use remotia::profilation::time::add::TimestampAdder;
use remotia::serialization::bincode::BincodeDeserializer;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    pipeline::{component::Component, Pipeline},
    processors::{error_switch::OnErrorSwitch, functional::Function},
    profilation::time::diff::TimestampDiffCalculator,
    render::winit::WinitRenderer,
};
use remotia_ffmpeg_codecs::{decoders::DecoderBuilder, ffi, scaling::ScalerBuilder};

use remotia_srt::{
    receiver::SRTFrameReceiver,
    srt_tokio::{options::ByteCount, SrtSocket},
};
use screen_stream::types::{BufferType::*, Error::*, FrameData, Stat::*};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    width: u32,

    #[arg(long)]
    height: u32,

    #[arg(long, default_value_t=String::from("127.0.0.1:9000"))]
    server_address: String,
}

const POOLS_SIZE: usize = 1;

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World! I will mirror your screen encoding it using the H264 codec.");

    let args = Args::parse();

    log::info!("Streaming at {}x{}", args.width, args.height);

    let pixels_count = (args.width * args.height) as usize;
    let mut registry = PoolRegistry::new();
    registry
        .register(SerializedFrameData, POOLS_SIZE, pixels_count * 4)
        .await;
    registry
        .register(DecodedRGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;

    let (decoder_pusher, decoder_puller) = DecoderBuilder::new()
        .codec_id("h264")
        .encoded_buffer_key(EncodedFrameBuffer)
        .decoded_buffer_key(DecodedRGBAFrameBuffer)
        .scaler(
            ScalerBuilder::new()
                .input_width(args.width as i32)
                .input_height(args.height as i32)
                .input_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_YUV420P)
                .output_pixel_format(ffi::AVPixelFormat_AV_PIX_FMT_BGRA)
                .build(),
        )
        .drain_error(NoFrame)
        .codec_error(CodecError)
        .build();

    let mut error_pipeline = Pipeline::<FrameData>::singleton(
        Component::new()
            .append(Function::new(|fd| {
                log::warn!("Dropped frame");
                Some(fd)
            }))
            .append(registry.get(SerializedFrameData).redeemer().soft())
            .append(registry.get(DecodedRGBAFrameBuffer).redeemer().soft()),
    )
    .feedable();

    log::info!("Connecting...");
    let socket = SrtSocket::builder()
        .set(|options| options.receiver.buffer_size = ByteCount(1024 * 1024))
        .call(args.server_address.as_str(), None)
        .await
        .unwrap();

    let pipeline = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(registry.get(SerializedFrameData).borrower())
                .append(SRTFrameReceiver::new(SerializedFrameData, socket))
                .append(BincodeDeserializer::new(SerializedFrameData))
                .append(TimestampDiffCalculator::new(CaptureTime, ReceptionDelay))
                .append(registry.get(SerializedFrameData).redeemer())
                .append(TimestampAdder::new(DecodePushTime))
                .append(decoder_pusher)
                .append(OnErrorSwitch::new(&mut error_pipeline)),
        )
        .link(
            Component::new()
                .append(registry.get(DecodedRGBAFrameBuffer).borrower())
                .append(decoder_puller)
                .append(OnErrorSwitch::new(&mut error_pipeline))
                .append(TimestampDiffCalculator::new(DecodePushTime, DecodeTime))
                .append(WinitRenderer::new(
                    DecodedRGBAFrameBuffer,
                    args.width,
                    args.height,
                ))
                .append(TimestampDiffCalculator::new(CaptureTime, FrameDelay))
                .append(registry.get(DecodedRGBAFrameBuffer).redeemer()),
        )
        .link(
            Component::new().append(
                ConsoleAverageStatsLogger::new()
                    .header("Statistics")
                    .log(ReceptionDelay)
                    .log(FrameDelay)
                    .log(DecodeTime)
            ),
        );

    let mut handles = Vec::new();
    handles.extend(error_pipeline.run());
    handles.extend(pipeline.run());

    for handle in handles {
        handle.await.unwrap();
    }
}
