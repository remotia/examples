use log::info;
use remotia::{
    buffers::pool::BuffersPool,
    pipeline::{Pipeline, component::Component},
    processors::{functional::ClosureAppends, ticker::Ticker},
    sync_processor::SyncProcessorWrapper,
};

use crate::{
    processors::RandomGenerator,
    types::{Buffer, FrameData},
};

mod processors;
mod types;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Hello World! I am a simple sync processors example.");

    let full_pool = BuffersPool::new(Buffer::Full, 2, 4).await;
    let delta_pool = BuffersPool::new(Buffer::Delta, 2, 4).await;

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000))
                .append(full_pool.borrower())
                .append(SyncProcessorWrapper::new(RandomGenerator::new(
                    Buffer::Full,
                )))
                .closure(|fd: FrameData| {
                    fd.print_buffers();
                    Some(fd)
                })
                .append(full_pool.redeemer())
                .append(delta_pool.borrower()), // .append(DeltaEncoder)
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
