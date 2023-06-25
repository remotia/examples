use clap::Parser;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    capture::y4m::Y4MFrameCapturer,
    pipeline::{component::Component, registry::PipelineRegistry, Pipeline},
    register,
    render::winit::WinitRenderer, processors::functional::Function, traits::{BorrowFrameProperties, PullableFrameProperties},
};

use crate::{types::{BufferType::*, FrameData, Pipelines}, color_cast::ColorCaster};

mod types;
mod color_cast;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    file_path: String,

    #[arg(short, long)]
    width: u32,

    #[arg(short, long)]
    height: u32,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World! I am the screen-mirror example server.");

    let args = Args::parse();

    let mut pools = PoolRegistry::new();
    let mut pipelines = PipelineRegistry::<FrameData, Pipelines>::new();

    pools
        .register(YUVBuffer, 1, (args.width * args.height * 2) as usize)
        .await;
    pools
        .register(
            DecodedRGBABuffer,
            1,
            (args.width * args.height * 4) as usize,
        )
        .await;

    let width = args.width;
    let height = args.height;

    register!(
        pipelines,
        Pipelines::Main,
        Pipeline::<FrameData>::new().link(
            Component::new()
                .append(pools.get(YUVBuffer).borrower())
                .append(Y4MFrameCapturer::new(YUVBuffer, &args.file_path))
                .append(pools.get(DecodedRGBABuffer).borrower())
                .append(ColorCaster::new(width, height))
                .append(pools.get(YUVBuffer).redeemer())
                .append(WinitRenderer::new(
                    DecodedRGBABuffer,
                    args.width,
                    args.height,
                ))
                .append(pools.get(DecodedRGBABuffer).redeemer())
        )
    );

    pipelines.run().await;
}
