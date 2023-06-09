use clap::Parser;
use log::info;
use remotia::{
    buffers::BufferAllocator,
    capture::scrap::ScrapFrameCapturer,
    pipeline::{component::Component, Pipeline},
    processors::ticker::Ticker, transmission::sender::TcpFrameSender,
};
use screen_mirror::{BufferType, FrameData};

use tokio::net::{TcpListener, TcpStream};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    binding_address: String,

    #[arg(short, long, default_value_t = 60)]
    framerate: u64
}

async fn establish_connection(address: &str) -> TcpStream {
    let listener = TcpListener::bind(address).await.unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    socket
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Hello World! I am the screen-mirror example server.");

    let args = Args::parse();

    let mut capturer = ScrapFrameCapturer::new_from_primary(BufferType::RawFrameBuffer);

    info!("Streaming at {}x{}", capturer.width(), capturer.height());

    let socket = establish_connection(&args.binding_address).await;

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000 / args.framerate))
                .append(BufferAllocator::new(
                    BufferType::RawFrameBuffer,
                    capturer.buffer_size(),
                ))
                .append(capturer)
                .append(TcpFrameSender::new(BufferType::RawFrameBuffer, socket)),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
