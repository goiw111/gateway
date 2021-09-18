use actix_web::{web, dev};
use crate::util::{LGError, LGEInfo};
use crate::Authako;
use crate::register::{APIRegister, Register};
use actix_web::HttpRequest;
use awc::Client;
#[cfg(feature = "oauth")]
mod oauth;
#[cfg(feature = "join")]
mod join;
mod seission;

use actix_web::HttpResponse;
use actix_http::http::StatusCode;
use forwarded::{Forwarded,ForwardedElement};
use crate::authako::Session;
use actix_web::http::header::Header;

use actix_web::error;
enum PError {
    Forwarded(error::ParseError),
    ParseForwarded(forwarded::ParseForwardedElementError),
    PayloadError(actix_web::client::PayloadError),
    SendRequest(actix_web::client::SendRequestError),
}

impl LGEInfo for PError {
    fn status_code(&self) -> StatusCode {
        use actix_web::ResponseError;
        match self {
            PError::Forwarded(_)    => StatusCode::BAD_REQUEST,
            PError::SendRequest(e)  => e.status_code(),
            _                       => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    fn message_text(&self) -> &str {
        match self {
            PError::Forwarded(_)        => "//TODO",
            PError::ParseForwarded(_)   => "//TODO",
            PError::PayloadError(_)     => "//TODO",
            PError::SendRequest(_)      => "//TODO",
        }
    }
    fn description_text(&self) -> &str {
        match self {
            PError::Forwarded(_)        => "//TODO",
            PError::ParseForwarded(_)   => "//TODO",
            PError::PayloadError(_)     => "//TODO",
            PError::SendRequest(_)      => "//TODO",

        }
    }
}

impl From<error::ParseError> for PError {
    fn from(err: error::ParseError) -> Self {
        PError::Forwarded(err)
    }
}
impl From<forwarded::ParseForwardedElementError> for PError {
    fn from(err: forwarded::ParseForwardedElementError) -> Self {
        PError::ParseForwarded(err)
    }
}

impl From<actix_web::client::PayloadError> for PError {
    fn from(err: actix_web::client::PayloadError) -> Self {
        PError::PayloadError(err)
    }
}

impl From<actix_web::client::SendRequestError> for PError {
    fn from(err: actix_web::client::SendRequestError) -> Self {
        PError::SendRequest(err)
    }
}

/*#[warn(unused_must_use)]
#[route("{path:[^{*}/]+}",
method="GET",
method="HEAD",
method="POST",
method="PUT",
method="DELETE",
wrap = "Authako",
wrap = "APIRegister")]*/
pub async fn proxy(
    req:        HttpRequest,
    pay:        web::Payload,
    reg:        Register,
    path:       web::Path<Vec<String>>,
    client:     web::Data<Client>,
    session:    Session)
    -> Result<HttpResponse, LGError> {
    let client_data = reg.get(&req, path.into_inner(), &session)?;
    let uri = format!("{}:{}",client_data.host(), client_data.port());
    //TODO: rewrite Forwarded parser 
    let mut forwarded  = Forwarded::parse(&req).map_err(PError::from)?;
    if let Some(sadds) = req.peer_addr() {
        let mut fe = ForwardedElement::new(sadds.ip().into());
        let appg = req.app_config();
        let conninfo = req.connection_info();
        if forwarded.is_empty() {
            fe.set_by(appg.local_addr().into());
            if let Some(sec) = client_data.data::<&str>() {
                fe.set_extensions(("secret",sec));
            }
        }
        fe.set_host(conninfo.host()).map_err(PError::from)?;
        forwarded.set_element(fe);
    }
    use futures::future::TryFutureExt;
    use actix_web::HttpMessage;
    client
        .request(req.method().clone(), uri)
        .no_decompress()
        .set(forwarded)
        .send_stream(pay)
        .map_ok( move |x| {
            use futures::stream::TryStreamExt;
            dev::HttpResponseBuilder::new(x.status())
                .content_type(x.content_type())
                .streaming(x.map_err(|e| LGError::from(PError::from(e))))
        })
        .map_err(|e| LGError::from(PError::from(e)))
        .await
}

pub fn config(cfg: &mut web::ServiceConfig) {
    use actix_web::guard;
    use actix_files::Files;

    cfg.service(seission::login)
        .service(seission::seission)
        .service(seission::logout);

    #[cfg(feature = "oauth")]
    cfg.service(web::scope("/oauth")
        .service(oauth::authorize)
        .service(oauth::token));

    #[cfg(feature = "join")]
    cfg.service(join::signup)
        .service(join::join);

    let proxy = web::scope("/")
        .wrap(APIRegister)
        .wrap(Authako)
        .default_service(web::route().to(proxy));
    let files = web::scope("/")
        .guard(guard::Host("media.localhost.local"))
        .service(Files::new("/css", "./css/"))
        .default_service(web::route().to(|| HttpResponse::SeeOther()
                .set_header(actix_web::http::header::LOCATION, "https://localhost.local:8443")
                .finish()));
    cfg.service(files)
        .service(proxy);
}
