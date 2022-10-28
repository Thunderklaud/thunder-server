use std::ops::Add;
use actix_web::{Responder, web::{Data, Json}, Result, HttpResponse};
use tracing::{event, Level};
use serde::{Deserialize, Serialize};

use crate::model;
use crate::model::user::User;
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

pub async fn register(new_user: Json<User>) -> HttpResponse {
    event!(Level::WARN, "user controller called");

    let mut data = User {
        id: None,
        name: new_user.name.to_owned(),
        location: new_user.location.to_owned(),
        title: new_user.title.to_owned(),
    };
    let user_detail = data.create().await;
    match user_detail {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }

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
