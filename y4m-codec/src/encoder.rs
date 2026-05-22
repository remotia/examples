use std::ffi::CString;

use clap::Parser;
use cstr::cstr;
use rsmpeg::avcodec::{AVCodec, AVCodecContext};
use rsmpeg::avutil::{AVDictionary, AVFrame, AVRational};
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;

use y4m_codec::{h264_writer::H264Writer, y4m_reader::Y4MReader};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output: String,

    #[arg(short, long, default_value_t = 23)]
    crf: u32,
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    log::info!("Encoding {} -> {}", args.input, args.output);

    let mut y4m_reader = Y4MReader::from_file(&args.input);
    let width = y4m_reader.width();
    let height = y4m_reader.height();

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

    let input_avframe = {
        let mut f = AVFrame::new();
        f.set_width(width as i32);
        f.set_height(height as i32);
        f.set_format(ffi::AV_PIX_FMT_RGBA);
        f.alloc_buffer().expect("Failed to alloc input AVFrame buffer");
        f
    };

    let mut scaled_avframe = {
        let mut f = AVFrame::new();
        f.set_width(width as i32);
        f.set_height(height as i32);
        f.set_format(ffi::AV_PIX_FMT_YUV420P);
        f.alloc_buffer().expect("Failed to alloc scaled AVFrame buffer");
        f
    };

    let mut scaler = SwsContext::get_context(
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

    let mut h264_writer = H264Writer::from_file(&args.output);
    let mut frame_id: i64 = 0;
    let mut rgba_buf = vec![0u8; width * height * 4];

    loop {
        match y4m_reader.read_next_frame(&mut rgba_buf) {
            Ok(true) => {}
            Ok(false) => {
                log::info!("Y4M EOF after {} frames, flushing encoder", frame_id);
                break;
            }
            Err(e) => {
                log::error!("Y4M read error: {:?}", e);
                break;
            }
        }

        let linesize = input_avframe.linesize[0] as usize;
        let h = input_avframe.height as usize;
        let data = unsafe {
            std::slice::from_raw_parts_mut(input_avframe.data[0], h * linesize)
        };
        data.copy_from_slice(&rgba_buf);

        scaler
            .scale_frame(&input_avframe, 0, input_avframe.height, &mut scaled_avframe)
            .expect("Scaling failed");

        scaled_avframe.set_pts(frame_id);
        frame_id += 1;

        if let Err(e) = encode_context.send_frame(Some(&scaled_avframe)) {
            log::warn!("Encoder send_frame error: {:?}", e);
        }

        loop {
            match encode_context.receive_packet() {
                Ok(packet) => {
                    let data = unsafe {
                        std::slice::from_raw_parts(packet.data, packet.size as usize)
                    };
                    h264_writer.write_packet(data);
                }
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => break,
                Err(e) => panic!("Encoder receive_packet error: {:?}", e),
            }
        }
    }

    encode_context.send_frame(None).expect("Failed to flush encoder");

    loop {
        match encode_context.receive_packet() {
            Ok(packet) => {
                let data = unsafe {
                    std::slice::from_raw_parts(packet.data, packet.size as usize)
                };
                h264_writer.write_packet(data);
            }
            Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => break,
            Err(e) => panic!("Encoder flush receive_packet error: {:?}", e),
        }
    }

    log::info!("Encoding complete: {} frames, {} bytes written", frame_id, h264_writer.bytes_written);
}
