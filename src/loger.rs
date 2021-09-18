use std::task::{Context, Poll};
use core::cell::RefCell;

use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, FromRequest};
use actix_web::HttpMessage;
use actix_web::HttpRequest;
use actix_web::dev::Payload;
use actix_web::dev::RequestHead;
use actix_web::dev::Extensions;
use actix_web::http::Cookie;
use actix_web::http::HeaderValue;
use actix_web::http::header::SET_COOKIE;

use cookie::{Key, CookieJar};
use mongodb::bson::oid::ObjectId;

use std::rc::Rc;
use futures::future::{ok, FutureExt, Ready, LocalBoxFuture};

pub type LogedInner = CookieJar;
#[derive(Debug)]
pub struct Loged (Rc<RefCell<LogedInner>>);

pub trait UserSession {
    /// Extract the [`Session`] object
    fn get_session(&self) -> Loged;
}

impl UserSession for HttpRequest {
    fn get_session(&self) -> Loged {
        Loged::get_session(&mut *self.extensions_mut())
    }
}

impl UserSession for ServiceRequest {
    fn get_session(&self) -> Loged {
        Loged::get_session(&mut *self.extensions_mut())
    }
}

impl UserSession for RequestHead {
    fn get_session(&self) -> Loged {
        Loged::get_session(&mut *self.extensions_mut())
    }
}

impl Loged {
    pub fn get(&self, key: &Key) -> Option<ObjectId> {
        let mut jar = self.0.borrow_mut();
        if let Some(c) = jar.signed(key).get("GSID") {
            if let Some(id) = c.value().split("::").next() {
                return ObjectId::with_string(id).ok();
            }
        }
        None
    }

    fn get_session(extensions: &mut Extensions) -> Self {
        if let Some(s) = extensions.get::<Rc<RefCell<LogedInner>>>() {
            return Loged(Rc::clone(s));
        }
        let inner = Rc::new(RefCell::new(CookieJar::new()));
        extensions.insert(Rc::clone(&inner));
        Loged(inner)
    }

    fn set_session(extensions: &mut Extensions, loged: Loged) {
        if extensions.get::<Rc<RefCell<LogedInner>>>().is_none() {
            extensions.insert(Rc::clone(&loged.0));
        }
    }

    fn load(req: &ServiceRequest, name: String) -> Self {
        let mut jar = CookieJar::new();
        if let Some(mut cookie) = req.cookie(name.as_str()) {
            cookie.set_name("GSID");
            jar.add_original(cookie.clone());
            return Loged(Rc::new(RefCell::new(jar)));
        }
        Loged(Rc::new(RefCell::new(jar)))
    }

    pub fn login(
        &mut        self,
        key:        &Key,
        session:    ObjectId,
        user_id:    String) {
        let s = format!("{}::{}",session.to_hex(),user_id);
        let mut jar = self.0.borrow_mut();
        jar.signed(key).add(Cookie::new("GSID",s));
    }

    pub fn logout(&mut  self, key:  &Key) {
        let mut jar = self.0.borrow_mut();
        jar.signed(key).remove(Cookie::named("GSID"));
    }

}

impl FromRequest for Loged {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Loged::get_session(&mut *req.extensions_mut()))
    }
}

pub struct Loger;

impl<S, B> Transform<S> for Loger
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LogerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LogerMiddleware { 
            service,
            inner:      Rc::new(LogerInner::new())
        })
    }
}

struct LogerInner {
    name: String
}

impl LogerInner {
    fn new() -> Self {
        LogerInner {
            name:   std::env::var("SESSID").unwrap()
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

pub struct LogerMiddleware<S> {
    service:    S,
    inner:      Rc<LogerInner>
}

impl<S, B> Service for LogerMiddleware<S>
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
        let auth = Loged::load(&req,self.inner.name());
        Loged::set_session(&mut req.extensions_mut(), auth);
        let fut = self.service.call(req);
        let inner = Rc::clone(&self.inner);

        async move {
            let mut res = fut.await?;
            let req = res
                .request()
                .clone();
            if let Some(s) = req
                .extensions()
                .get::<Rc<RefCell<LogedInner>>>() {
                    let jar = s.borrow_mut();
                    for cookie in jar.delta() {
                        println!("{}",cookie.encoded().to_string());
                        let mut cookie = cookie.clone();
                        if cookie.name() == "GSID" {
                        cookie.set_name(inner.name());
                        cookie.set_http_only(true);
                        cookie.set_secure(true);
                        println!("{}",cookie.encoded().to_string());
                        let val = HeaderValue::from_str(&cookie
                            .encoded()
                            .to_string());
                            if let Ok(val) = val {
                                res.headers_mut()
                                    .append(SET_COOKIE,val);
                            }
                        }
                    }
                }
            Ok(res)
        }.boxed_local()
    }
}
