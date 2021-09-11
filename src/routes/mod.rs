use actix_web::web;
use crate::register::{APIRegister, Register};
use actix_web::HttpRequest;
use actix_web::route;
use awc::Client;
use actix_web::error::Error;

#[cfg(feature = "oauth")]
mod oauth;
#[cfg(feature = "join")]
mod join;
mod seission;

use crate::{Authako,error::{GatewayError, IntoGatewayError}};
use actix_web::{get, Responder, HttpResponse};
use actix_http::http::header::Accept;
use actix_http::{http, Response};
use futures::future::Ready;
use actix_web::http::header::Header;
use actix_web::http::StatusCode;
use serde::{Serialize, Serializer, ser::SerializeStruct};
use forwarded::{Forwarded,ForwardedElement};
use crate::authako::Session;
use futures::future::TryFutureExt;

#[get("/", wrap = "Authako")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
}

#[warn(unused_must_use)]
#[route("api/{res}{path:(/.*)*}",
method="GET",
method="HEAD",
wrap = "Authako",
wrap = "APIRegister")]
async fn proxy(
    req:        HttpRequest,
    pay:        web::Payload,
    reg:        Register,
    path:       web::Path<Vec<String>>,
    client:     web::Data<Client>,
    session:    Session) 
    -> Result<actix_web::Either<HttpRequest, GValue>, GError> {
    let accept = Accept::parse(&req).unwrap_or(Accept::star());
    let client_data = reg.get(&req, path.into_inner(), &session)?;
    let uri = format!("{}:{}",client_data.host(), client_data.port());
    //TODO: rewrite Forwarded parser 
    let mut forwarded  = Forwarded::parse(&req).err_to_gerr(&accept)?;
    if let Some(sadds) = req.peer_addr() {
        let mut fe = ForwardedElement::new(sadds.ip().into());
        let appg = req.app_config();
        let conninfo = req.connection_info();
        if forwarded.is_empty() == true {
            fe.set_by(appg.local_addr().into());
            if let Some(sec) = client_data.data::<&str>() {
                fe.set_extensions(("secret",sec));
            }
        }
        fe.set_host(conninfo.host())
            .err_with_status(&accept,StatusCode::INTERNAL_SERVER_ERROR)?;
        forwarded.set_element(fe);
    }
    let upstream_req = client
        .request(req.method(), uri)
        .no_decompress()
        .set(forwarded)
        .send_stream(pay)
        .and_then(|x| async move {
            match x.json::<GValue>().await {
                OK(v) => actix_web::Either::B(v),
                Err(_) => actix_web::Either::A(
                    dev::HttpResponseBuilder::new(x.status())
                    .content_type(x.content_type())
                    .streaming(x)),
            }
        })
        .map_err(|x| x.into_gerr(&accept))
        .await

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
