use std::net::SocketAddr;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::cell::RefCell;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::HttpMessage;
use actix_web::dev::Payload;
use actix_web::dev::Extensions;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::BufRead;
use std::time::SystemTime;
use actix_http::Error;
use actix_service::{Service, Transform};
use std::task::{Context, Poll};
use futures_util::future::{ok,err, FutureExt, LocalBoxFuture, Ready};
use std::fs::File;
use toml::de;
use serde_derive::Deserialize;

pub type RegisterInner = BTreeMap<String,SocketAddr>;

pub struct Register (Rc<RefCell<RegisterInner>>);

impl Register {
    pub fn get(&self, service:     &str) -> Option<SocketAddr> {
        if let Some(s) = self.0.borrow().get(service) {
            return Some(s.clone());
        }
        None
    }

    fn set_session(extensions: &mut Extensions, register:   Register) {
        if let None = extensions.get::<Rc<RefCell<RegisterInner>>>() {
            extensions.insert(Rc::clone(&register.0));
        }
    }

    fn get_session(extensions: &mut Extensions) -> Register {
        if let Some(s_impl) = extensions.get::<Rc<RefCell<RegisterInner>>>() {
            return Register(Rc::clone(&s_impl));
        }
        let inner = Rc::new(RefCell::new(RegisterInner::default()));
        extensions.insert(inner.clone());
        Register(inner)
    }
}

pub struct APIRegister;

#[derive(Deserialize,Debug)]
struct Serveses {
      serveses:   BTreeMap<String,SocketAddr>
}

impl FromRequest for Register {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Register::get_session(&mut *req.extensions_mut()))
    }
}


impl<S, B> Transform<S> for APIRegister
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RegisterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let file = OpenOptions::new()
            .read(true)
            .create(true)
            .append(true)
            .open("api.toml");
        if let Ok(f) = file {
            let mut reader = BufReader::new(f);
            let register = de::from_slice::<Serveses>(reader
                .fill_buf()
                .unwrap());
            if let Ok(r) = register {
                let time = reader.get_ref()
                    .metadata()
                    .unwrap()
                    .modified()
                    .unwrap();
                return ok(RegisterMiddleware {
                    service:    service,
                    inner:      InnerData {
                        buffer:     reader,
                        last_time:  time,
                        register:   Rc::new(RefCell::new(r.serveses))
                    }
                })
            }
        }
        err(())
    }
}

struct  InnerData {
    buffer:     BufReader<File>,
    register:   Rc<RefCell<RegisterInner>>,
    last_time:  SystemTime
}

pub struct RegisterMiddleware<S> {
    service:    S,
    inner:      InnerData
}

impl<S, B> Service for RegisterMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        if let Ok(md) = self.inner.buffer.get_ref().metadata() {
            if let Ok(time) = md.modified() {
                if time != self.inner.last_time {
                    let data = de::from_slice::<Serveses>(&self
                        .inner
                        .buffer
                        .buffer());
                        if let Ok(data) = data {
                            let _ = self.inner.register.replace(data.serveses);
                        }
                }
            }
        }
        Register::set_session(&mut *req.extensions_mut()
            ,Register(Rc::clone(&self.inner.register)));
        let fut = self.service.call(req);

        async move {
            let res = fut.await;
            res
        }.boxed_local()
    }
}
