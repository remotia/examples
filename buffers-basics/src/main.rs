use log::info;
use remotia::{
    buffers::pool::BuffersPool,
    pipeline::{Pipeline, component::Component},
    processors::ticker::Ticker,
};

use crate::data::{Buffer, FrameData, FrameDataAppends};

mod data;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Hello World! I am the screen-mirror example server.");

    let pool = BuffersPool::new(Buffer::Dummy, 1, 16).await;

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000))
                .set_phase(data::Phase::PreBorrow)
                .print_buffer()
                .append(pool.borrower())
                .set_phase(data::Phase::PostBorrow)
                .print_buffer(),
        )
        .link(
            Component::new()
                .set_phase(data::Phase::PreRedeem)
                .print_buffer()
                .append(pool.redeemer())
                .set_phase(data::Phase::PostRedeem)
                .print_buffer(),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
