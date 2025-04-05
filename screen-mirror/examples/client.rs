use clap::Parser;
use remotia::{
    buffers::BufferAllocator,
    pipeline::{component::Component, Pipeline},
    processors::functional::Function,
    render::winit::WinitRenderer,
    traits::BorrowMutFrameProperties,
    transmission::receiver::TcpFrameReceiver,
};
use screen_mirror::{BufferType, FrameData};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    server_address: String,

    #[arg(long)]
    width: u32,

    #[arg(long)]
    height: u32,
}

async fn establish_connection(address: String) -> TcpStream {
    TcpStream::connect(address).await.unwrap()
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World! I am the screen-mirror example client.");

    let args = Args::parse();

    let socket = establish_connection(args.server_address).await;

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(BufferAllocator::new(
                    BufferType::RawFrameBuffer,
                    (args.width * args.height * 4) as usize,
                ))
                .append(TcpFrameReceiver::new(BufferType::RawFrameBuffer, socket))
                .append(Function::new(|mut frame_data: FrameData| {
                    log::debug!("Received frame data, changing channels order");

                    let frame_buffer = frame_data.get_mut_ref(&BufferType::RawFrameBuffer).unwrap();
                    for pixel in frame_buffer.chunks_mut(4) {
                        let (r, g, b) = (pixel[0], pixel[1], pixel[2]);

                        pixel[0] = b;
                        pixel[1] = g;
                        pixel[2] = r;
                        pixel[3] = 255;
                    }

                    Some(frame_data)
                }))
                .append(WinitRenderer::new(
                    BufferType::RawFrameBuffer,
                    args.width,
                    args.height,
                )),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
