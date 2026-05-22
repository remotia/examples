use clap::Parser;
use cstr::cstr;
use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket};
use rsmpeg::avutil::AVFrame;
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;

use y4m_codec::{h264_reader::H264Reader, png_writer::PNGWriter};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output_dir: String,

    #[arg(short, long)]
    width: usize,

    #[arg(short, long)]
    height: usize,
}

fn decode_available_frames(
    ctx: &mut AVCodecContext,
    scaler: &mut SwsContext,
    scaled_avframe: &mut AVFrame,
    png_writer: &mut PNGWriter,
) {
    loop {
        match ctx.receive_frame() {
            Ok(codec_avframe) => {
                scaler
                    .scale_frame(&codec_avframe, 0, codec_avframe.height, scaled_avframe)
                    .expect("Scaling failed");

                let linesize = scaled_avframe.linesize[0] as usize;
                let h = scaled_avframe.height as usize;
                let data = unsafe {
                    std::slice::from_raw_parts(scaled_avframe.data[0], h * linesize)
                };
                png_writer.write_frame(data);
            }
            Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => break,
            Err(_) => break,
        }
    }
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    log::info!("Decoding {} -> {} ({}x{})", args.input, args.output_dir, args.width, args.height);

    let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).expect("h264 decoder not found");
    let mut decode_context = AVCodecContext::new(&decoder);
    decode_context.open(None).expect("Unable to open decoder");

    let mut parser_context = AVCodecParserContext::init(decoder.id).expect("h264 parser not found");
    let mut packet = AVPacket::new();

    let mut scaler = SwsContext::get_context(
        args.width as i32,
        args.height as i32,
        ffi::AV_PIX_FMT_YUV420P,
        args.width as i32,
        args.height as i32,
        ffi::AV_PIX_FMT_RGBA,
        ffi::SWS_BILINEAR,
        None,
        None,
        None,
    )
    .expect("Failed to create SwsContext");

    let mut scaled_avframe = {
        let mut f = AVFrame::new();
        f.set_width(args.width as i32);
        f.set_height(args.height as i32);
        f.set_format(ffi::AV_PIX_FMT_RGBA);
        f.alloc_buffer().expect("Failed to alloc scaled AVFrame buffer");
        f
    };

    let mut h264_reader = H264Reader::from_file(&args.input);
    let all_data = h264_reader.read_all();
    log::info!("Read {} bytes from H264 file", all_data.len());

    let mut png_writer = PNGWriter::new(args.output_dir.into(), args.width as u32, args.height as u32);

    let mut offset = 0usize;

    while offset < all_data.len() {
        let result = parser_context.parse_packet(&mut decode_context, &mut packet, &all_data[offset..]);

        match result {
            Ok((packet_ready, consumed)) => {
                offset += consumed;
                if packet_ready {
                    if let Err(e) = decode_context.send_packet(Some(&packet)) {
                        log::debug!("Decoder send_packet error: {:?}", e);
                    }
                    decode_available_frames(&mut decode_context, &mut scaler, &mut scaled_avframe, &mut png_writer);
                }
            }
            Err(e) => {
                log::warn!("Parser error: {:?}, skipping", e);
                break;
            }
        }
    }

    log::info!("Parsing complete, flushing decoder");

    if let Err(e) = decode_context.send_packet(None) {
        log::warn!("Decoder flush send_packet error: {:?}", e);
    }

    decode_available_frames(&mut decode_context, &mut scaler, &mut scaled_avframe, &mut png_writer);

    log::info!("Decoding complete: {} frames written", png_writer.frame_count());
}
