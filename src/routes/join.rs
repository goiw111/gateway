use actix_web::{get, post, Responder, HttpResponse};

#[get("/join")]
pub async fn signup() -> impl Responder {
    HttpResponse::Ok()
 }

#[post("/join")]
pub async fn join() -> impl Responder {
    HttpResponse::Ok()
 }
