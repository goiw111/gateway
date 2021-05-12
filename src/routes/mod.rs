use actix_web::web;

#[cfg(feature = "oauth")]
mod oauth;
#[cfg(feature = "join")]
mod join;
mod seission;

use crate::Authako;
use actix_web::{get, Responder, HttpResponse};

#[get("/", wrap = "Authako")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
 }

pub fn config(cfg: &mut web::ServiceConfig) {

    cfg.service(seission::login)
        .service(seission::seission);

    #[cfg(feature = "oauth")]
    cfg.service(web::scope("/oauth")
        .service(oauth::authorize)
        .service(oauth::token));

    #[cfg(feature = "join")]
    cfg.service(join::signup)
        .service(join::join);

    cfg.service(index);
}
