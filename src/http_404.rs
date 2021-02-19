use actix_web::{Responder,HttpResponse};
use sailfish::TemplateOnce;

#[derive(TemplateOnce)]
#[template(path = "404.stpl")]
struct NotFound<'a> {
    title   : &'a str,
    msg     : &'a str,
}

pub async fn notfound() -> impl Responder {

    let body = NotFound {
        title: "404: Page not found . gateway",
        msg: "Sorry, we can’t find that page"}
    .render_once();

    let body = match body {
        Ok(s)   =>s,
        Err(_)  => String::from("sorry, we have an internal server issue"),
    };


    HttpResponse::NotFound()
       .content_type("text/html; charset=utf-8")
       .body(body)
}
