use std::ffi::CString;
use std::sync::Arc;

use clap::Parser;
use cstr::cstr;
use remotia::buffers::BytesMut;
use remotia::pipeline::{component::Component, Pipeline};
use remotia::traits::{FrameProperties, PullableFrameProperties};
use rsmpeg::avcodec::{AVCodec, AVCodecContext};
use rsmpeg::avutil::{AVDictionary, AVFrame, AVRational};
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;
use tokio::sync::Mutex;

use y4m_codec::processors::{
    encoder_puller::EncoderPuller, encoder_pusher::EncoderPusher,
    h264_packet_writer::H264PacketWriter,
};
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

    let encoder = AVCodec::find_encoder_by_name(cstr!("libx264")).expect("libx264 encoder not found");
    let mut encode_context = AVCodecContext::new(&encoder);
    encode_context.set_width(width as i32);
    encode_context.set_height(height as i32);
    encode_context.set_pix_fmt(ffi::AV_PIX_FMT_YUV420P);
    encode_context.set_time_base(AVRational { num: 1, den: 60 * 1000 });
    encode_context.set_framerate(AVRational { num: 60, den: 1 });

    let crf_cstr = CString::new(format!("{}", args.crf)).unwrap();
    let dict = AVDictionary::new(cstr!("crf"), crf_cstr.as_c_str(), 0)
        .set(cstr!("preset"), cstr!("medium"), 0)
        .set(cstr!("tune"), cstr!("film"), 0);

    let _remaining = encode_context.open(Some(dict)).expect("Unable to open encoder");
    let encode_context = Arc::new(Mutex::new(encode_context));

    let scaler = SwsContext::get_context(
        width as i32,
        height as i32,
        ffi::AV_PIX_FMT_RGBA,
        width as i32,
        height as i32,
        ffi::AV_PIX_FMT_YUV420P,
        ffi::SWS_BILINEAR,
        None,
        None,
        None,
    )
    .expect("Failed to create SwsContext");

    let input_avframe = {
        let mut f = AVFrame::new();
        f.set_width(width as i32);
        f.set_height(height as i32);
        f.set_format(ffi::AV_PIX_FMT_RGBA);
        f.alloc_buffer().expect("Failed to alloc input AVFrame buffer");
        f
    };

    let scaled_avframe = {
        let mut f = AVFrame::new();
        f.set_width(width as i32);
        f.set_height(height as i32);
        f.set_format(ffi::AV_PIX_FMT_YUV420P);
        f.alloc_buffer().expect("Failed to alloc scaled AVFrame buffer");
        f
    };

    let pusher = EncoderPusher::new(encode_context.clone(), scaler, input_avframe, scaled_avframe);

    let h264_file = Arc::new(std::sync::Mutex::new(
        std::fs::File::create(&args.output).expect("Unable to create H264 output file"),
    ));

    let mut pipeline = Pipeline::<FrameData>::new()
        .tag("encoder")
        .feedable()
        .link(
            Component::new()
                .append(pusher)
                .tag("pusher"),
        )
        .link(
            Component::new()
                .append(EncoderPuller::new(encode_context.clone()))
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
