use log::info;
use remotia::{
    buffers::pool::BuffersPool,
    pipeline::{Pipeline, component::Component},
    processors::{functional::ClosureAppends, ticker::Ticker},
    sync_processor::SyncProcessorWrapper,
};

use crate::{
    processors::{DeltaEncoder, RandomGenerator},
    types::{Buffer, FrameData},
};

mod processors;
mod types;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Hello World! I am a simple sync processors example.");

    let buffers_size = 4;

    let full_pool = BuffersPool::new(Buffer::Full, 2, buffers_size).await;
    let delta_pool = BuffersPool::new(Buffer::Delta, 2, buffers_size).await;

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000))
                .append(full_pool.borrower())
                .append(SyncProcessorWrapper::new(RandomGenerator::new(
                    Buffer::Full,
                )))
                .closure(|fd: FrameData| {
                    log::info!("Random generation performed");
                    fd.print_buffers();
                    Some(fd)
                })
                .append(delta_pool.borrower())
                .append(SyncProcessorWrapper::new(DeltaEncoder::new(
                    Buffer::Full,
                    Buffer::Delta,
                    buffers_size,
                )))
                .closure(|fd: FrameData| {
                    log::info!("Delta encoding performed");
                    fd.print_buffers();
                    Some(fd)
                })
                .append(full_pool.redeemer()),
        )
        .link(
            Component::new()
                .append(Ticker::new(1000))
                .append(full_pool.borrower())
                // .append(DeltaDecoder)
                .append(delta_pool.redeemer())
                .append(full_pool.redeemer()),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
