use actix_web::{web, middleware::Logger};
use env_logger::Env;

mod routes;
mod http_404;
mod preserver;
mod config;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_web::{App, HttpServer};

    env_logger::init_from_env(Env::default()
        .default_filter_or("info"));

    let config  =   config::Config::init()
        .expect("Server configuration");
    let acceptor=   config::ssl::get_acceptor(config.pk,config.cc);

    HttpServer::new(
        || App::new()
            .wrap(Logger::default())
            .wrap(preserver::Preserver)
            .configure(routes::config)
            .default_service(web::to(http_404::notfound))
            )
        .bind(format!("{}:{}",config.host,config.port))?
        .bind_openssl(format!("{}:{}",config.host,config.sport),acceptor)?
        .run()
        .await
}
