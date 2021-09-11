use actix_web::{
    HttpResponse,
    ResponseError,
    http::StatusCode};
use std::fmt;
use log::{warn};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Serialize};
use std::error::Error;
use actix_web::http::header::Accept;
use actix_web::http;
use std::ops::Deref;
use actix_web::error::{InternalError, ParseError};

#[derive(Debug)]
pub struct GatewayError {
    error:      Box<dyn ResponseError>,
    accept:     Accept
}
#[derive(Serialize, Debug)]
struct GResult<'a> {
    error:  &'a GatewayError
}

impl From<ParseError> for GatewayError {
    fn from(error: ParseError) -> Self {
        GatewayError {
            error:      Box::new(error),
            accept:     Accept::json()
        }
    }
}

pub trait IntoGatewayError {
    type Item;
    type Error;
    type Output;

    fn err_to_gerr(self, accept:    &Accept) -> Self::Output
        where 
            Self::Error: ResponseError;
    fn err_with_status(self, accept: &Accept, status: StatusCode) -> Self::Output;
}

impl<T, E: fmt::Debug + fmt::Display + 'static> IntoGatewayError for Result<T, E> {
    type Item  = T;
    type Error = E;
    type Output = Result<Self::Item, GatewayError>;

    fn err_to_gerr(self, accept: &Accept) -> Self::Output
        where 
            Self::Error: ResponseError {
        match self {
            Ok(i)    =>Ok(i),
            Err(e)   =>{
                Err(GatewayError {
                    error:      Box::new(e),
                    accept:     accept.clone()
                })},
        }
    }
    fn err_with_status(self, accept: &Accept, status: StatusCode) -> Self::Output {
        match self {
            Ok(i)    =>Ok(i),
            Err(e)   =>Err(GatewayError {
                error:      Box::new(InternalError::new(e, status)),
                accept:     accept.clone()
            }),
        }
    }
}


impl Error for GatewayError {}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status_code().as_u16() {
            400 => write!(f,"Oh, sorry something wrong with your provided information"),
            404 => write!(f,"Oh, that thing you are looking for is not found"),
            500 => write!(f,"Oops, something went wrong"),
            501 => write!(f,"Oh, sorry we don't have a service with that name"),
            504 => write!(f,"Oops, something wrong with the service that you looking for"),
            _   => fmt::Display::fmt(&self.error, f),
        }
    }
}

impl ResponseError for GatewayError {
    fn error_response(&self) -> HttpResponse {
        warn!("{}", self);
        let mut repb = HttpResponse::build(self.status_code());
        let accept = self.accept.clone();
        for i in self.accept.deref().iter() {
            match &*i.item.to_string() {
                "application/ron"   => {
                    let body = match serde_any::ser::to_string(&GResult{error: self},
                        serde_any::format::Format::Ron)
                        .err_with_status(&accept, StatusCode::INTERNAL_SERVER_ERROR) {
                            Ok(b)   => b,
                            Err(e)  => return e.error_response(),
                        };
                    return repb.header(http::header::CONTENT_TYPE, "application/ron")
                        .body(body);
                },
                "application/yaml"   => {
                    let body = match serde_any::ser::to_string(&GResult{error: self}, 
                        serde_any::format::Format::Yaml)
                        .err_with_status(&accept, StatusCode::INTERNAL_SERVER_ERROR) {
                            Ok(b)   => b,
                            Err(e)  => return e.error_response(),
                    };
                    return repb.header(http::header::CONTENT_TYPE, "application/yaml")
                        .body(body);
                }
                _   => (),
            }
        }
        repb.json(&GResult{error: self})
    }
    fn status_code(&self) -> StatusCode {
        self.error.status_code()
    }
}
