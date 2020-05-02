use golem;

#[tokio::main]
async fn main() {
    match golem::run_v8().await {
        Result::Ok(()) => println!("{}", "All good"),
        Result::Err(err) => println!("{}", err.to_string())
    };
}
