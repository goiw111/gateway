use actix_web::{get, post, Responder, HttpResponse};

#[get("/login")]
async fn login() -> impl Responder {
    HttpResponse::Ok()
 }
 
#[post("/seission")]
async fn seission() -> impl Responder {
    HttpResponse::Ok()
 }

