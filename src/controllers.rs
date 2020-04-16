use actix_web::{post, web, Responder};
use serde::Serialize;

#[post("/actor/{id}")]
pub async fn index(info: web::Path<String>, body: web::Json<serde_json::Value>) -> impl Responder {
    format!("Hello {}!\n You sent the following body: {}", info, body)
}

#[derive(Serialize)]
struct CreateActorRequest {}

#[post("/actor")]
pub async fn createActor(body: web::Json<serde_json::Value>) -> impl Responder {
    format!("Hello!\n You sent the following body: {}", body)
}
