use actix_web::web;
use crate::loger::Loger;
use crate::register::{APIRegister, Register};
use actix_web::HttpRequest;
use actix_web::route;

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

#[route("/{api:api(/.*)*}",
method="GET",
method="POST",
wrap = "Loger",
wrap = "APIRegister")]
async fn proxy(
    _req:   HttpRequest,
    reg:    Register,
    api:    web::Path<String>) -> impl Responder {
    if let Some(a) = api.split('/').nth(1) {
        if let Some(s) = reg.get(a) {

        }
    }

    HttpResponse::Ok()
}

pub fn config(cfg: &mut web::ServiceConfig) {

    cfg.service(seission::login)
        .service(seission::seission)
        .service(seission::logout)
        .service(proxy);

    #[cfg(feature = "oauth")]
    cfg.service(web::scope("/oauth")
        .service(oauth::authorize)
        .service(oauth::token));

    #[cfg(feature = "join")]
    cfg.service(join::signup)
        .service(join::join);

    cfg.service(index);
}
