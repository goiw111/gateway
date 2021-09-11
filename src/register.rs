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
use actix_http::Error;
use actix_service::{Service, Transform};
use std::task::{Context, Poll};
use futures::future::{ok,err, FutureExt, LocalBoxFuture, Ready};
use toml::de;
use core::cell::Cell;
use std::io::Read;
use std::os::linux::fs::MetadataExt;
use crate::authako::Resource;
use actix_web::http::StatusCode;
use crate::error::GatewayError;
use actix_web::http::header::Accept;
use actix_web::dev::ResourceDef;
use std::collections::HashSet;
use actix_web::http::Method;
use actix_web::http::header::Header;
use crate::error::IntoGatewayError;
use std::str::FromStr;
use std::fmt;
use crate::authako::Session;
use actix_web::ResponseError;
use serde::{Serialize, Deserialize};

type RegisterInner = BTreeMap<String, Res>;

pub struct Register (Rc<RefCell<RegisterInner>>);

impl Register {
    #[inline]
    pub fn get(&self, msg: &HttpRequest, mut path: Vec<String>, session: &Session) -> Result<Client, GatewayError> {
        path.retain(|x| !x.is_empty());
        let accept = Accept::parse(msg).unwrap_or(Accept::star());
        if let Some(s) = path.get(0) {
            if let Some(r) = self.0.borrow_mut().get_mut(s) {
                let slash = String::from("/");
                let path = path.get(1).unwrap_or(&slash);
                return r.get_resource(path, msg.method(), session.get_resource())
                    .err_to_gerr(&accept);
            }
            return  Err(ResError::NotFound).err_to_gerr(&accept);

        }
        return  Err(ResError::BadRequest).err_to_gerr(&accept);
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

#[derive(Debug)]
struct Dir {
    resource:  ResourceDef,
    method:     HashSet<Method>,
    scopes:     Resource
}

impl<'de> serde::Deserialize<'de> for Dir {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
            #[derive(Deserialize,Debug)]
            struct DirDe {
                resources:  String,
                methods:    HashSet<String>,
                scopes:     String
            }

            let dir = DirDe::deserialize(deserializer)?;
            Ok(Dir {
                resource:  ResourceDef::new(&dir.resources),
                method:     dir.methods.iter()
                    .filter_map(|x| Method::from_str(&x).ok())
                    .collect::<HashSet<Method>>(),
                scopes:     Resource::from_str(&dir.scopes).map_err(serde::de::Error::custom)?
            })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Client {
    host:   String,
    port:   u16,
    data:   Option<toml::value::Value>
}

impl Client {
    #[inline]
    pub fn host(&self) -> &str {
        self.host.as_str()
    }
    #[inline]
    pub fn port(&self) -> u16 {
        self.port
    }
    #[inline]
    pub fn data<'de, T: serde::Deserialize<'de>>(&self) -> Option<T> {
        if let Some(ref v) = self.data {
            use serde::de::IntoDeserializer;
            return T::deserialize(v.clone().into_deserializer()).ok();
        }
        None
    }
}

#[derive(Deserialize,Debug)]
struct Res {
    routers:    Vec<Dir>,
    clients:    cll::CLList<Client>
}

#[derive(Debug)]
enum ResError {
    Forbidden,
    ServiceUnavailable,
    BadRequest,
    NotFound,
    MethodNotAllowed,
    InternalServe
}

impl std::error::Error for ResError {}

impl ResponseError for ResError {
    fn status_code(&self) -> StatusCode {
        match self {
            ResError::Forbidden =>          StatusCode::FORBIDDEN,
            ResError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ResError::BadRequest =>         StatusCode::BAD_REQUEST,
            ResError::NotFound =>           StatusCode::NOT_FOUND,
            ResError::MethodNotAllowed =>   StatusCode::METHOD_NOT_ALLOWED,
            ResError::InternalServe     =>  StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Display for ResError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResError::Forbidden =>
                f.write_str("client authenticated but does not have permission to access the requested resource"),
            ResError::ServiceUnavailable => 
                f.write_str("the requested service is not available"),
            ResError::BadRequest =>
                f.write_str("client sent an invalid request, such as lacking required request body or parameter"),
            ResError::NotFound => 
                f.write_str("the requested resource does not exist"),
            ResError::MethodNotAllowed => 
                f.write_str("the method received in the request-line is known by the origin server but not supported by the target resource."),
            ResError::InternalServe     => 
                f.write_str("Internal Server Error"),
        }
    }
}

impl Res {
    #[inline]
    fn get_resource(&mut self, path: &str, method: &Method, res: Resource) -> Result<Client, ResError> {
        for item in self.routers.iter() {
            if item.resource.is_match(path) {
                if item.method.contains(method) {
                    if item.scopes.comfortable_with(&res) {
                        return Ok(self.clients.next()
                            .map(|x| x.clone())
                            .ok_or(ResError::ServiceUnavailable)?);
                    }
                    return  Err(ResError::Forbidden);
                }
                return  Err(ResError::MethodNotAllowed);
            }
        }
        Err(ResError::BadRequest)
    }
}

#[derive(Deserialize, Debug, Default)]
struct Serveses {
      serveses:   BTreeMap<String, Res>
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
        if let Ok(mut f) = file {
            if let Ok(m) = f.metadata() {
                let mut bufr = String::new();
                let size = f.read_to_string(&mut bufr);
                let register = match de::from_str::<Serveses>(&bufr) {
                    Ok(reg) => reg,
                    Err(e)  => {
                        match size {
                            Ok(s) => if s == 0 {Serveses::default()} else {
                                log::error!("some thing goes wrong while deserializing the string in api.tml file: {}",e);
                                return err(());
                            },
                            _     =>{
                                log::error!("some thing goes wrong while triying to get the size of the file");
                                return err(());
                            },
                        }
                    },
                };
                return ok(RegisterMiddleware {
                    service:    service,
                    inner:      InnerData {
                        mtime:      Cell::new(m.st_mtime()),
                        register:   Rc::new(RefCell::new(register.serveses))
                    }
                })
            }
        }
        log::error!("some thing goes wrong while opening the api.toml file");
        err(())
    }
}

struct  InnerData {
    register:   Rc<RefCell<RegisterInner>>,
    mtime:      Cell<i64>
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
        let accept = Accept::parse(&req).unwrap_or(Accept::star());
        if let Ok(md) = std::fs::metadata("api.toml") {
            if self.inner.mtime.get() != md.st_mtime() {
                let serveses = OpenOptions::new()
                    .read(true)
                    .append(true)
                    .open("api.toml")
                    .map(|mut file| {
                        let mut buff = String::new();
                        let _ = file.read_to_string(&mut buff);
                        if buff.trim().is_empty() {
                            return Ok(Serveses::default());
                        }
                        de::from_str::<Serveses>(&buff)
                    });
                if let Ok(Ok(data)) = serveses {
                    self.inner.register.replace(data.serveses);
                    self.inner.mtime.replace(md.st_mtime());
                } else if let Ok(Err(e)) = serveses {
                    log::error!("{}",e);
                } else {
                    return async move {
                        Err(ResError::InternalServe)
                            .err_to_gerr(&accept)
                            .map_err(|x| x.into())
                    }.boxed_local();
                }
            }
            Register::set_session(&mut *req.extensions_mut()
                ,Register(Rc::clone(&self.inner.register)));
            let fut = self.service.call(req);
            return async move {
                let res = fut.await;
                res
            }.boxed_local();
        }
        return async move {
            Err(ResError::InternalServe)
                .err_to_gerr(&accept)
                .map_err(|x| x.into())
        }.boxed_local();
    }
}
