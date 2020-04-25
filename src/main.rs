// use actix_web::{middleware, web, App, HttpServer};
use golem;
// use golem::controllers;

fn main() {
    // // // HttpServer::new(|| {
    // // //     App::new()
    // // //         .wrap(middleware::Compress::default())
    // // //         .service(controllers::index)
    // // // })
    // // // .bind("127.0.0.1:8080")?
    // // // .run()
    // // // .await
    // ()
    golem::run_v8();
}
