use actix_jwt_authc::Authenticated;
use actix_web::{web::Json, HttpResponse};
use actix_web::web::Data;
use jsonwebtoken::EncodingKey;
use mongodb::results::InsertOneResult;
use serde::Serialize;
use tracing::{event, Level};
use crate::Claims;
use crate::jwt_utils::JWTTtl;

use crate::model::user::{Role, User};

#[derive(Serialize)]
pub struct DefaultResponse {
    result: Option<InsertOneResult>,
    status: bool,
    error: String,
}

pub async fn register(new_user: Json<User>) -> HttpResponse {
    if User::exists(&new_user.email).await {
        return HttpResponse::InternalServerError().json(DefaultResponse {
            result: None,
            status: false,
            error: "User with email already exists".parse().unwrap(),
        })
    }

    let mut data = User {
        id: None,
        firstname: new_user.firstname.to_owned(),
        lastname: new_user.lastname.to_owned(),
        email: new_user.email.to_owned(),
        pw_hash: new_user.pw_hash.to_owned(),
        role: Some(Role::BaseUser)
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
            status: false,
            error: err.to_string(),
        }),
    }
}
