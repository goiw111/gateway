use std::{alloc::Layout, fmt};

pub struct LGError {
    inner:  Box<dyn LGEInfo>,
}

impl fmt::Display for LGError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.inner.status_code())
    }
}

impl fmt::Debug for LGError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",self.inner.status_code())
    }
}

impl LGError {
    pub fn log(&self) {
        log::error!("Error : {:?}", self);
    }
    #[inline]
    fn render(&self) -> impl std::fmt::Display + '_ {
        markup::new! {
            @markup::doctype()
                html {
                    head {
                        title { "Error: " @self.inner.status_code().as_str() }
                    }
                    body {
                        #main { 
                            .err { 
                                h1  { @self.inner.status_code().to_string() }
                                h2  { @self.inner.message_text() }
                                p   { @self.inner.description_text() }
                            }
                        }
                    }
                }
        }
    }
}

pub trait LGEInfo {
    fn status_code(&self) -> actix_web::http::StatusCode;
    fn message_text(&self) -> &str;
    fn description_text(&self) -> &str;
}

impl actix_web::ResponseError for LGError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        self.inner.status_code()
    }
    fn error_response(&self) -> actix_web::web::HttpResponse {
        self.log();
        actix_web::web::HttpResponse::build(self.status_code())
            .body(self.render().to_string())
    }
}

struct Error;

impl LGEInfo for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    }
    fn message_text(&self) -> &str {
        "//TODO"
    }
    fn description_text(&self) -> &str {
        "//TODO"
    }
}

impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Error
    }
}

impl<E: 'static + LGEInfo> From<E> for LGError {
    fn from(error: E) -> Self {
        LGError {
            inner:  Box::new(error),
        }
    }
}
