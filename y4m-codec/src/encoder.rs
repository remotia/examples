use std::sync::Arc;

use clap::Parser;
use remotia::buffers::BytesMut;
use remotia::pipeline::{component::Component, Pipeline};
use remotia::traits::{FrameProperties, PullableFrameProperties};
use remotia_ffmpeg_codecs::encoders::EncoderBuilder;
use remotia_ffmpeg_codecs::encoders::fillers::rgba::RGBAFrameFiller;
use remotia_ffmpeg_codecs::scaling::ScalerBuilder;
use remotia_ffmpeg_codecs::options::Options;
use remotia_ffmpeg_codecs::ffi;

use y4m_codec::processors::h264_packet_writer::H264PacketWriter;
use y4m_codec::{BufferType, FrameData, Stat};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output: String,

    #[arg(short, long, default_value_t = 23)]
    crf: u32,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();
    log::info!("Encoding {} -> {}", args.input, args.output);

    let y4m_file = std::fs::File::open(&args.input).expect("Unable to open Y4M file");
    let mut y4m_reader = y4m::decode(y4m_file).expect("Unable to parse Y4M file");
    let width = y4m_reader.get_width();
    let height = y4m_reader.get_height();
    log::info!("Video dimensions: {}x{}", width, height);

    let scaler = ScalerBuilder::new()
        .input_width(width as i32)
        .input_height(height as i32)
        .input_pixel_format(ffi::AV_PIX_FMT_RGBA)
        .output_width(width as i32)
        .output_height(height as i32)
        .output_pixel_format(ffi::AV_PIX_FMT_YUV420P)
        .build();

    let options = Options::new()
        .set("crf", &args.crf.to_string())
        .set("preset", "medium")
        .set("tune", "film");

    let (encoder_pusher, encoder_puller) = EncoderBuilder::new()
        .codec_id("libx264")
        .filler(RGBAFrameFiller::new(BufferType::RgbaFrame))
        .scaler(scaler)
        .options(options)
        .build();

    let h264_file = Arc::new(std::sync::Mutex::new(
        std::fs::File::create(&args.output).expect("Unable to create H264 output file"),
    ));

    let mut pipeline = Pipeline::<FrameData>::new()
        .tag("encoder")
        .feedable()
        .link(
            Component::new()
                .append(encoder_pusher)
                .tag("pusher"),
        )
        .link(
            Component::new()
                .append(encoder_puller)
                .append(H264PacketWriter::new(h264_file.clone()))
                .tag("puller"),
        );

    let feeder = pipeline.get_feeder();
    let handles = pipeline.run();

    let mut frame_id: u128 = 0;
    loop {
        let frame = match y4m_reader.read_frame() {
            Ok(f) => f,
            Err(_) => {
                log::info!("Y4M EOF after {} frames, sending EOF frame", frame_id);
                break;
            }
        };

        let y = frame.get_y_plane();
        let u = frame.get_u_plane();
        let v = frame.get_v_plane();

        let mut rgba = BytesMut::with_capacity(width * height * 4);
        yuv420_to_rgba_into(y, u, v, width, height, &mut rgba);

        let mut fd = FrameData::default();
        fd.push(BufferType::RgbaFrame, rgba);
        fd.set(Stat::FrameId, frame_id);
        frame_id += 1;

        feeder.feed(fd);
    }

    let mut eof_fd = FrameData::default();
    eof_fd.set(Stat::Eof, 1);
    feeder.feed(eof_fd);

    drop(feeder);

    for handle in handles {
        handle.await.unwrap();
    }

    let file_size = h264_file.lock().unwrap().metadata().unwrap().len();
    log::info!("Encoding complete: {} frames, {} bytes written", frame_id, file_size);
}

fn yuv420_to_rgba_into(y_plane: &[u8], u_plane: &[u8], v_plane: &[u8], width: usize, height: usize, out: &mut BytesMut) {
    let uv_width = width / 2;
    for row in 0..height {
        for col in 0..width {
            let y_idx = row * width + col;
            let uv_row = row / 2;
            let uv_col = col / 2;
            let uv_idx = uv_row * uv_width + uv_col;

            let y = y_plane[y_idx] as f32;
            let u = u_plane[uv_idx] as f32 - 128.0;
            let v = v_plane[uv_idx] as f32 - 128.0;

            let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
            let g = (y - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
            let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;

            out.extend_from_slice(&[r, g, b, 255]);
        }
    }
}
