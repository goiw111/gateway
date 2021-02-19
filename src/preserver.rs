use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{http, Error, HttpResponse};
use futures::future::{ok, Either, Ready};

use log::info;
use actix_http::Response;


// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct Preserver;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S> for Preserver
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = PreserverMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(PreserverMiddleware { service })
    }
}

pub struct PreserverMiddleware<S> {
    service: S,
}

impl<S, B> Service for PreserverMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Self::Error>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {


        info!("{:#?}", req.head());

        if req.app_config().secure() {
            Either::Left(self.service.call(req))
        } else {
            let res = http_redir(&req);
            Either::Right(ok(req.into_response(res.into_body())))
        }
    }
}

fn http_redir(req: &ServiceRequest) -> Response {
   
    let location = format!("https://localhost.local:8443{}",
        req.uri()
        .path_and_query()
        .unwrap());

    #[allow(non_snake_case)]
    let STRICT_TRANSPORT_SECURITY: &'static str = 
        "strict-transport-security";


    HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .header(STRICT_TRANSPORT_SECURITY, "max-age=31536000")
        .finish()
}
