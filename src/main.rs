use actix_web::{web, middleware::Logger};
use env_logger::Env;

mod routes;
mod http_404;
mod preserver;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_web::{App, HttpServer};
    use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

    env_logger::init_from_env(Env::default()
        .default_filter_or("info"));

    let mut acceptor = SslAcceptor::mozilla_modern_v5(SslMethod::tls())
        .unwrap();
    acceptor.set_private_key_file("192.168.1.6+4-key.pem", SslFiletype::PEM).unwrap();
    acceptor.set_certificate_chain_file("192.168.1.6+4.pem").unwrap();

    HttpServer::new(
        || App::new()
            .wrap(Logger::default())
            .wrap(preserver::Preserver)
            .configure(routes::config)
            .default_service(web::to(http_404::notfound))
            )
        .bind("127.0.0.1:80")?
        .bind_openssl("127.0.0.1:443",acceptor)?
        .run()
        .await
}
