use actix_web::{web ,get, post, Responder, HttpResponse};
use crate::loger::Loger;
use crate::loger::Loged;
use actix_web::web::Data;
use crate::AppData;
use actix_web::http;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use sailfish::TemplateOnce;
use std::sync::Mutex;
use actix_web::HttpRequest;
use actix_web::HttpMessage;
use mongodb::bson::{doc};
use serde::{Serialize, Deserialize};
use bcrypt::verify;

#[derive(TemplateOnce)]
#[template(path = "login.stpl")]
struct LoginTemplate {
    appname:                String,
    description:            String,
    authenticity_token:     String
}

#[get("/login", wrap = "Loger")]
async fn login(loged: Loged, data: Data<Mutex<AppData>>) -> impl Responder {

    if let Ok(data) = data.lock() {
    if let Some(_) = loged.get(&data.key) {
            return HttpResponse::Found()
                .header(http::header::LOCATION, "/")
                .finish();
    }
    let appname = data.appname.clone();
    let description = data.description.clone();
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    let ctx = LoginTemplate {
        appname:                appname.clone(),
        description:            description.clone(),
        authenticity_token:     rand_string.clone(),
    };

    let res = ctx.render_once();

    if let Ok(s) = res {

        return HttpResponse::Ok()
            .cookie(
                http::Cookie::build("authenticity_token", 
                    rand_string.clone())
                .secure(true)
                .http_only(true)
                .finish()
            )
            .body(s);
    }}

    HttpResponse::InternalServerError()
        .finish()
}

#[derive(Deserialize)]
struct Credentials {
    authenticity_token:     String,
    login:                  String,
    password:               String
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    #[serde(rename = "_id")]
    user_name:  String,
    user_pass:  String
}

#[derive(Serialize, Deserialize, Debug)]
struct Seiss {
    user_id:    String,
}

#[post("/seission", wrap = "Loger")]
async fn seission(
    form:   web::Form<Credentials>,
    req:    HttpRequest,
    mut loged:  Loged,
    data:   Data<Mutex<AppData>>) -> impl Responder {

    if let Ok(data) = data.lock() {
        if let None = loged.get(&data.key) {
            if let Some(c) = req.cookie("authenticity_token") {
                if c.value() == form.authenticity_token {
                    let db = data.mongodb_db.clone();
                    let res = db.collection_with_type::<User>("users")
                        .find_one(doc! {
                            "_id":  form.login.clone(),
                        }, None).await;
                    if let Ok(r) = res {
                        if let Some(d) = r {
                            let ver = verify(form.password.clone()
                                ,d.user_pass.as_str());
                            if let Ok(b) = ver {
                                if b == true {
                                    if let Ok(id) = db
                                    .collection_with_type::<Seiss>("seission")
                                        .insert_one(Seiss {
                                            user_id: form.login.clone()
                                        },None).await {
                                    if let Some(obj_id) = id
                                        .inserted_id
                                            .as_object_id() {
                                        loged.login(&data.key,
                                            obj_id.clone(),
                                            form.login.clone());
                                        return HttpResponse::Found()
                                            .header(http::header::LOCATION,"/")
                                            .finish();
                                    }
                                }
                                }
                            }
                        }
                        return HttpResponse::Found()
                            .header(http::header::LOCATION,"/login#Incorrect")
                            .finish();
                    }
                }
            }
        } else {
            return HttpResponse::Found()
                .header(http::header::LOCATION, "/")
                .finish();
        }
    }
    HttpResponse::InternalServerError()
        .finish()
}

#[get("/logout", wrap = "Loger")]
async fn logout(
    mut loged:  Loged,
    data:       Data<Mutex<AppData>>)
    -> impl Responder {
        if let Ok(data) = data.lock() {
        loged.logout(&data.key);
        return HttpResponse::Found()
            .header(http::header::LOCATION,"/")
            .finish()
        }
        HttpResponse::InternalServerError()
            .finish()
}
