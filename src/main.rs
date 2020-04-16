use actix_web::{middleware, web, App, HttpServer};
use golem;
use golem::controllers;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    golem::run_v8();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .service(controllers::index)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
