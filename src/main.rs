use golem;
use std::time::Instant;

// #[tokio::main]
fn main() {
    let start_time = Instant::now();
    { let _ = golem::run_v8(); }
    let end_time = Instant::now();
    let delta_time = end_time - start_time;
    println!("Run Time: {}ms", delta_time.as_millis());
}
