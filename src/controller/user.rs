use actix_web::{web::{Json}, HttpResponse};
use mongodb::results::InsertOneResult;
use serde::Serialize;
use tracing::{event, Level};

use crate::model::user::User;

#[derive(Serialize)]
pub struct DefaultResponse {
    result: Option<InsertOneResult>,
    status: bool,
    error: String,
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
        Ok(user) => HttpResponse::Ok().json(DefaultResponse {
            result: Some(user),
            status: true,
            error: "".to_string(),
        }),
        Err(err) => HttpResponse::InternalServerError().json(DefaultResponse {
            result: None,
            status: true,
            error: err.to_string(),
        }),
    }
}
