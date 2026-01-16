use clap::Parser;
use file_player::loader::Y4MLoader;
use file_player::types::{BufferType::*, FrameData, Stat::*};
use remotia::pipeline::registry::PipelineRegistry;
use remotia::profilation::loggers::console::ConsoleAverageStatsLogger;
use remotia::register;
use remotia::{
    buffers::pool_registry::PoolRegistry,
    pipeline::{Pipeline, component::Component},
    render::winit::WinitRenderer,
};

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
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Hello World!");

    let args = Args::parse();

    log::info!("Streaming at {}x{}", args.width, args.height);
    let mut renderer = WinitRenderer::new(RGBAFrameBuffer);
    let render_runner = renderer.allocate(args.width, args.height);
    let gui_handle = tokio::spawn(async move {
        render_runner.start();
    });

    let pixels_count = (args.width * args.height) as usize;
    let mut pools = PoolRegistry::new();

    pools
        .register(RGBAFrameBuffer, POOLS_SIZE, pixels_count * 4)
        .await;

    let mut pipelines = PipelineRegistry::<FrameData, Pipelines>::new();

    register!(
        pipelines,
        Pipelines::Main,
        Pipeline::<FrameData>::new()
            .link(
                Component::new()
                    .append(pools.get(RGBAFrameBuffer).borrower())
                    .append(Y4MLoader::from_file(""))
                    .append(renderer)
                    .append(pools.get(RGBAFrameBuffer).redeemer()),
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
