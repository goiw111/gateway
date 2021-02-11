use actix_web::{Responder,HttpResponse};

pub async fn notfound() -> impl Responder {
   HttpResponse::Ok()
}
