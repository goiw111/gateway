use actix_web::web;

#[cfg(feature = "oauth")]
mod oauth;
#[cfg(feature = "join")]
mod join;
mod seission;

pub fn config(cfg: &mut web::ServiceConfig) {

    cfg.service(seission::login)
        .service(seission::seission);

    #[cfg(feature = "oauth")]
    cfg.service(web::scope("/oauth")
        .service(oauth::authorize)
        .service(oauth::token));

    #[cfg(feature = "join")]
    cfg.service(join::signup)
        .service(join::join);
            
}
