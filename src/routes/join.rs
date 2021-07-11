use actix_web::{
    web,
    get,
    post,
    Responder,
    HttpResponse,
    HttpRequest,
    HttpMessage};
use crate::loger::Loged;
use crate::loger::Loger;
use actix_web::web::Data;
use rand::distributions::Alphanumeric;
use crate::AppData;
use sailfish::TemplateOnce;
use std::sync::Mutex;
use actix_web::http;
use rand::{thread_rng, Rng};
use serde::{Serialize, Deserialize};
use mongodb::bson::{doc};
use bcrypt::hash;
use validator::{Validate, ValidationError};

#[derive(TemplateOnce)]
#[template(path = "join.stpl")]
struct JoinTemplate {
    appname:                String,
    description:            String,
    authenticity_token:     String
}

#[get("/join", wrap = "Loger")]
pub async fn signup(loged: Loged, data: Data<Mutex<AppData>>) -> impl Responder {

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
    let ctx = JoinTemplate {
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

fn validate_username(username:  &str) ->  Result<(), ValidationError> {
    if username
        .chars()
        .all(|x|char::is_ascii_alphanumeric(&x) || x == '.' || x == '_') {
            return Ok(());
    }
    Err(ValidationError::new("username"))
}

#[derive(Deserialize, Validate, Debug)]
pub struct CredentialsJoin {
    authenticity_token:     String,
    #[validate(length(min = 5, max = 25), custom = "validate_username")]
    username:               String,
    #[validate(email)]
    email:                  String,
    #[validate(length(min = 8))]
    password:               String,
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    #[serde(rename = "_id")]
    user_name:  String,
    user_pass:  String,
    emails:     Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
struct Seiss {
    user_id:    String,
}

#[post("/join", wrap = "Loger")]
pub async fn join(
    data:       Data<Mutex<AppData>>,
    mut loged:  Loged,
    req:        HttpRequest,
    form:       web::Form<CredentialsJoin>) -> impl Responder {
    let mut vec = Vec::new();
    if let Ok(data) = data.lock() {
        if let None = loged.get(&data.key) {
            if let Some(c) = req.cookie("authenticity_token") {
                if c.value() == form.authenticity_token {
                    match form.validate() {
                        Err(e)  =>{
                            for (key,_) in e.field_errors() {
                                match key {
                                    "username"  =>{
                                        if form.username.is_empty() {
                                            //msg: username field is empty
                                            vec.push(1);
                                        } else {
                                            //msg: pleas enter a username
                                            //countain chars [A-Z], [a-z], _,.
                                            //and [1-9]
                                            vec.push(4);
                                        }
                                    },
                                    "email"     =>{
                                        if form.email.is_empty() {
                                            //msg: email field is empty
                                            vec.push(2);
                                        } else {
                                            //msg: pleas enter a valid email
                                            vec.push(5);
                                        }
                                    },
                                    "password"  =>{
                                        if form.password.is_empty() {
                                            //msg: password field is empty
                                            vec.push(3);
                                        } else {
                                            //msg: pleas enter a powerfull
                                            //password
                                            vec.push(6);
                                        }
                                    },
                                    //msg: something happened wrong
                                    _           =>vec.push(0),
                                }
                            }
                        },
                        Ok(_)   =>{
                            let db = data.mongodb_db.clone();
                            let res = db.collection_with_type::<User>("users")
                                .find_one(doc! {
                                    "_id":  form.username.clone(),
                                }, None)
                            .await;
                            if let Ok(r) = res {
                                if let Some(_) = r {
                                    //msg: unavalibel usernamen
                                    vec.push(7);
                                } else {
                                    let hash = hash(form.password.clone(),12);
                                    if let Ok(h) = hash {
                                        if let Ok(_) = db
                                            .collection_with_type::<User>("users")
                                            .insert_one(User {
                                                user_name:  form.username.clone(),
                                                user_pass:  h,
                                                emails:     vec![form.email.clone()],
                                            },None).await {
                                                if let Ok(id) = db
                                                    .collection_with_type::<Seiss>("seission")
                                                        .insert_one(Seiss {
                                                            user_id: form.username.clone()
                                                        },None).await {
                                                            if let Some(obj_id) = id
                                                                .inserted_id
                                                                    .as_object_id() {
                                                                        loged.login(&data.key,obj_id.clone(),form.username.clone());
                                                                        return HttpResponse::Found()
                                                                            .header(http::header::LOCATION, "/")
                                                                            .finish();
                                                                    } else {
                                                                        vec.push(0);
                                                            }
                                                        } else {
                                                            vec.push(0);
                                                }
                                            } else {
                                                vec.push(0);
                                        }
                                    } else {
                                        vec.push(0);
                                    }
                                }
                            }
                        },
                    }
                } else {
                    vec.push(0);
                }
            } else {
                vec.push(0);
            }
        } else {
            vec.push(0);
        }
    } else {
        vec.push(0);
    }
    return HttpResponse::Found()
        .header(http::header::LOCATION, format!("/join#e:{:?}",vec))
        .finish();
}
