use golem;
use std::time::Instant;

#[tokio::main]
async fn main() {
    golem::run_v8().await;
}
