use actix_web::{web, middleware::Logger};
use env_logger::Env;
use crate::authako::Authako;
use cookie::Key;
use std::sync::Mutex;
use mongodb::{Client, options::ClientOptions, Database};

mod routes;
mod http_404;
mod preserver;
mod config;
mod authako;
mod loger;
mod register;

pub struct AppData {
    key:            Key,
    appname:        String,
    description:    String,
    mongodb_db:     Database,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_web::{App, HttpServer};

    env_logger::init_from_env(Env::default()
        .default_filter_or("info"));

    let config  =   config::Config::init()
        .expect("Server configuration");
    let acceptor=   config::ssl::get_acceptor(config.pk,config.cc);
    let mut options = ClientOptions::parse("mongodb://localhost:27017")
        .await
        .unwrap();
    options.app_name = Some(config.appname.clone());
    let client = Client::with_options(options)
        .unwrap();
    let db = client.database("seissions");
    let key = Key::from(config.key.as_bytes());
    let data = web::Data::new(Mutex::new(AppData {
        key:            key.clone(),
        appname:        config.appname.clone(),
        description:    config.description.clone(),
        mongodb_db:     db.clone(),
    }));

    HttpServer::new(
        move || {
            App::new()
            .app_data(data.clone())
            .wrap(Logger::default())
            .wrap(preserver::Preserver)
            .configure(routes::config)
            .default_service(web::to(http_404::notfound))
        })
        .bind(format!("{}:{}",config.host,config.port))?
        .bind_openssl(format!("{}:{}",config.host,config.sport),acceptor)?
        .run()
        .await
}
