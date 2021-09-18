use uuid::Uuid;
use futures::future::{ok, FutureExt, LocalBoxFuture, Ready};
use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::env;
use std::collections::BTreeMap;
use actix_web::dev::Extensions;

use cookie::{Key, CookieJar};

use actix_service::{
    Service,
    Transform};
use actix_web::{
    dev::ServiceRequest,
    dev::ServiceResponse,
    Error,
    HttpMessage};
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::dev::Payload;
use serde::{Deserialize, Serialize, ser::SerializeSeq};
use bitflags::bitflags;

pub struct AuthakoInner {
    key:        Key,
    name:       String,
}

bitflags!{
    #[derive(Default)]
    pub struct Per: u8 {
        const   C = 0b0001 | Self::R.bits;
        const   R = 0b0010;
        const   U = 0b0100 | Self::R.bits;
        const   D = 0b1000 | Self::R.bits;
    }
}

impl Serialize for Per {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
    {
        use std::collections::HashSet;
        let mut vec = HashSet::new();
        for (x, v) in [(0b0001, "c"), (0b0010, "r"), (0b0100, "u"), (0b1000, "d")]
            .iter() {
            if x & self.bits == *x {
                vec.insert(*v);
            }
            if (vec.contains(&"u") || vec.contains(&"c") || vec.contains(&"d")) 
                && vec.contains(&"r") {
                let _ = vec.remove(&"r");
            }
        }
        let mut seq = serializer.serialize_seq(Some(vec.len()))?;
        for element in vec {
            seq.serialize_element(element)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Per {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        let vec = std::vec::Vec::deserialize(deserializer)?;
        let mut p = Per::empty();
        for element in vec {
            match element {
                "c" => p.bits |= 0b0001,
                "r" => p.bits |= 0b0010,
                "u" => p.bits |= 0b0100,
                "d" => p.bits |= 0b1000,
                _   => (),
            }
        }
        Ok(p)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Resource {
    P(Per),
    R(BTreeMap<String, Resource>),
}

impl PartialEq for Resource {
    fn eq(&self, other: &Resource) -> bool {
        match self {
            Resource::P(p1) => {
                if let Resource::P(p2) = other {
                    if p2.bits & p1.bits == p2.bits {
                        return  true;
                    }
                }
            },
            Resource::R(m1) => {
                if let Resource::R(m2) = other {
                    let mut res = true;
                    for (l,r1) in m2.iter() {
                        if let Some(r2) = m1.get(l) {
                            res &= r1.eq(r2);
                        } else {
                            return false;
                        }
                    }
                    return  res;
                }
            },
        }
        return  false;
    }
}

impl Resource {
    pub fn from(s: &str) -> Self {
        //TODO make it Result<self, LGError>
        if let Ok(r) = serde_json::from_str(s) {
            return  r;
        }
        Resource::default()
    }
    pub fn comfortable_with(&self, other: &Resource) -> bool {
        self.eq(other)
    }
}

impl Default for Resource {
    fn default() -> Self {
        let mut res = BTreeMap::new();
        res.insert(String::from("gateway"), Resource::P(Per::R));
        Resource::R(res)
    }
}

#[derive(Default, PartialEq, Debug)]
pub struct Roler {
    suid:       Option<(Uuid,Uuid)>,
    roles:      Resource,
}

pub struct Session (Rc<RefCell<Roler>>);

impl Session {
    fn get_session(extensions: &mut Extensions) -> Session {
        if let Some(s) = extensions.get::<Rc<RefCell<Roler>>>() {
            return Session(Rc::clone(s));
        }
        let inner = Rc::new(RefCell::new(Roler::default()));
        extensions.insert(Rc::clone(&inner));
        Session(inner)
    }
    pub fn get_resource(&self) -> Resource {
        self.0.borrow().roles.clone()
    }
}

impl FromRequest for Session {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ok(Session::get_session(&mut *req.extensions_mut()))
    }
}


impl Roler {
    fn set_ruler(self, req: &mut ServiceRequest) {
        let extensions = &mut *req.extensions_mut();
        if let None = extensions.get::<Rc<RefCell<Roler>>>() {
            let inner = Rc::new(RefCell::new(self));
            extensions.insert(inner.clone());
        }
    }
}

impl AuthakoInner {
    fn new(key: &[u8]) -> AuthakoInner {
        AuthakoInner {
            key:    Key::derive_from(key),
            name:   String::from("GSID")
        }
    }

    fn load(&self, req: &ServiceRequest) -> Roler {
        if let Some(cookie) = req.cookie(self.name.as_str()) {
            let mut jar = CookieJar::new();
            jar.add_original(cookie.clone());
            if let Some(cookie) = jar.signed(&self.key)
                .get(&self.name) {
                    let value: Vec<&str> = cookie
                        .value().split("::").collect();
                    if value.len() == 3 {
                        return Roler {
                            suid:    Some((Uuid::parse_str(value[0]).unwrap()
                                         ,Uuid::parse_str(value[1]).unwrap())),
                            roles:  Resource::from(value[2]),
                        }
                    }
                }
        }
        Default::default()
    }
}

pub struct Authako;

impl<S, B> Transform<S> for Authako
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthakoMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let key = env::var("KEY")
            .expect("KEY env var notfound");
        ok(AuthakoMiddleware {
            service,
            inner: Arc::new(AuthakoInner::new(key.as_bytes())),
        })
    }
}

pub struct AuthakoMiddleware<S> {
    service:    S,
    inner:      Arc<AuthakoInner>
}

impl<S, B> Service for AuthakoMiddleware<S>
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

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let auth = self.inner.load(&req);
        auth.set_ruler(&mut req);

        let fut = self.service.call(req);

        async move {
            let res = fut.await;
            res
        }.boxed_local()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(Resource::default(), Default::default());

        assert_eq!(Roler {
            suid:   None,
            roles:  Default::default(),
        }, Roler {
            suid:   None,
            roles:  Default::default(),
        })
    }
}
