use actix_web::{get, Responder, HttpResponse};

#[get("/authorize")]
async fn authorize() -> impl Responder {
     HttpResponse::Ok()
 }
 
 #[get("/token")]
async fn token() -> impl Responder {
     HttpResponse::Ok()
 }
