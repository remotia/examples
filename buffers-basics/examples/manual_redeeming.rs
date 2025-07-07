use log::info;
use remotia::{
    buffers::pool::BuffersPool,
    pipeline::{Pipeline, component::Component},
    processors::ticker::Ticker,
};

use buffers_basics::data::{Buffer, FrameData, FrameDataAppends, Phase};

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Hello World! I am an example of buffers manual redeeming.");

    let pool = BuffersPool::new(Buffer::Dummy, 1, 16).await;

    let handles = Pipeline::<FrameData>::new()
        .link(
            Component::new()
                .append(Ticker::new(1000))
                .set_phase(Phase::PreBorrow)
                .print_buffer()
                .append(pool.borrower())
                .set_phase(Phase::PostBorrow)
                .print_buffer(),
        )
        .link(
            Component::new()
                .set_phase(Phase::PreRedeem)
                .print_buffer()
                .append(pool.redeemer())
                .set_phase(Phase::PostRedeem)
                .print_buffer(),
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
