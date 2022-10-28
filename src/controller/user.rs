use std::ops::Add;
use actix_web::{Responder, web, Result, HttpResponse};
use tracing::{event, Level};
use serde::{Deserialize, Serialize};

use crate::model;
/*
pub fn login<'r>(req: &'r Request, data: Data<'r>) -> route::BoxFuture<'r> {
    let param = req
        .param::<&'r str>(1)
        .and_then(Result::ok)
        .unwrap_or("unnamed");

    route::Outcome::from(req, param).pin()
}
pub fn logout<'r>(req: &'r Request, data: Data<'r>) -> route::BoxFuture<'r> {
    let param = req
        .param::<&'r str>(0)
        .and_then(Result::ok)
        .unwrap_or("unnamed");

    route::Outcome::from(req, param).pin()
}*/

#[derive(Deserialize)]
pub struct FormData {
    name: String,
}

#[derive(Serialize)]
struct MyObj {
    name: String,
}

pub async fn register(form: web::Form<FormData>) -> Result<impl Responder> {
    event!(Level::WARN, "user controller called");
    let obj = MyObj {
        name: form.name.to_string(),
    };
    Ok(web::Json(obj))
    //HttpResponse::Ok().body(format!("username: {}", form.name))

    /*
    let mut new_user = model::user::User {
        id: None,
        email: "mail@example.com".to_string(),
        pw_hash: "".to_string(),
        role: "testrole".to_string(),
    };
    let user_save_result = tokio_runtime.handle().block_on(new_user.create());
    if user_save_result.is_ok() {
        event!(Level::INFO, "user creation successful");
    } else {
        event!(Level::WARN, "user creation failed");
    }
     */
}
