use log::info;
use remotia::{
    buffers::pool::BuffersPool, pipeline::{component::Component, Pipeline}, processors::{functional::Function, ticker::Ticker}
};

use crate::data::{Buffer, FrameData};

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
                .append(Function::new(FrameData::print_buffer))
                .append(pool.borrower())
                .append(Function::new(FrameData::print_buffer))
        ).link(
            Component::new()
                .append(Function::new(FrameData::print_buffer))
                .append(pool.redeemer())
                .append(Function::new(FrameData::print_buffer))
        )
        .run();

    for handle in handles {
        handle.await.unwrap();
    }
}
