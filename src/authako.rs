use uuid::Uuid;
use serde::Deserialize;

use futures_util::future::{ok, FutureExt, LocalBoxFuture, Ready};

use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::env;

use cookie::{Key, CookieJar};

use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};

pub struct AuthakoInner {
    key:        Key,
    name:       String,
}

#[derive(Deserialize, Debug, PartialEq)]
enum Resource {
    Permission(u8),
    Resources(Vec<Resources>),
}

#[derive(Deserialize, Debug, PartialEq)]
struct Resources {
    name:   String,
    role:   Resource
}

impl Resources {
    /*pub fn has_right() -> bool {
    }*/

    fn from(res: &str) -> Self {
        serde_json::from_str(res)
            .unwrap_or(Default::default())
    }
}

impl Default for Resources {
    fn default() -> Self {
        Resources {
            name:   String::from("None"),
            role:   Resource::Permission(0),
        }
    }
}

#[derive(Default, PartialEq, Debug)]
pub struct Roler {
    suid:   Option<(Uuid,Uuid)>,
    roles:  Resources,
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
                            roles:  Resources::from(value[2]),
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
        assert_eq!(Resources {
                name:   String::from("None"),
                role:   Resource::Permission(0),
        },Default::default());

        assert_eq!(Roler {
            suid:   None,
            roles:  Default::default(),
        }, Roler {
            suid:   None,
            roles:  Default::default(),
        })
    }
}
