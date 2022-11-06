use std::ops::Add;

use actix_jwt_authc::Authenticated;
use actix_web::{HttpResponse, web::Json};
use actix_web::web::Data;
use jsonwebtoken::{encode, EncodingKey, Header};
use mongodb::results::InsertOneResult;
use serde::Serialize;
use time::OffsetDateTime;
use tracing::{event, Level};

use crate::Claims;
use crate::jwt_utils::{JWT_SIGNING_ALGO, JWTTtl};
use crate::model::user::{Role, User, UserLogin};

#[derive(Serialize)]
pub struct DefaultResponse {
    #[serde(flatten)]
    result: Option<ResultDataType>,
    status: bool,
    error: String,
}

#[derive(Serialize)]
enum ResultDataType {
    #[serde(rename(serialize = "result"))]
    LoginResponse(LoginResponse),
    #[serde(rename(serialize = "result"))]
    InsertOneResult(InsertOneResult),
    #[serde(rename(serialize = "result"))]
    TestResponse(TestResponse),
}

#[derive(Serialize)]
struct LoginResponse {
    jwt: String,
    claims: Claims,
}

#[derive(Serialize)]
struct TestResponse {
    session_info: Authenticated<Claims>,
    email: String,
}

pub async fn login(login_user: Json<UserLogin>,
                   jwt_encoding_key: Data<EncodingKey>,
                   jwt_ttl: Data<JWTTtl>) -> HttpResponse {
    event!(Level::INFO, "login_user: {}", login_user.email);

    let user = User::get_by_email(login_user.email.to_owned().as_str()).await;
    if user.is_some() {
        let sub = user.unwrap().id.unwrap().to_string();
        let iat = OffsetDateTime::now_utc().unix_timestamp() as usize;
        let expires_at = OffsetDateTime::now_utc().add(jwt_ttl.0);
        let exp = expires_at.unix_timestamp() as usize;

        let jwt_claims = Claims { iat, exp, sub };
        let jwt_token = encode(
            &Header::new(JWT_SIGNING_ALGO),
            &jwt_claims,
            &jwt_encoding_key,
        ).unwrap();
        let login_response = LoginResponse {
            jwt: jwt_token,
            claims: jwt_claims,
        };

        return HttpResponse::Ok().json(DefaultResponse {
            result: Some(ResultDataType::LoginResponse(login_response).into()),
            status: true,
            error: "".to_string(),
        });
    }

    HttpResponse::InternalServerError().json(DefaultResponse {
        result: None,
        status: false,
        error: "User with email does not exist".parse().unwrap(),
    })
}

pub async fn test(authenticated: Authenticated<Claims>) -> HttpResponse {
    HttpResponse::Ok().json(DefaultResponse {
        result: Some(ResultDataType::TestResponse(TestResponse {
            session_info: authenticated.clone(),
            email: User::get_authenticated(&authenticated).await.unwrap().email,
        }).into()),
        status: true,
        error: "".to_string(),
    })
}

pub async fn register(new_user: Json<User>) -> HttpResponse {
    if User::exists(&new_user.email).await {
        return HttpResponse::InternalServerError().json(DefaultResponse {
            result: None,
            status: false,
            error: "User with email already exists".parse().unwrap(),
        });
    }

    let mut data = User {
        id: None,
        firstname: new_user.firstname.to_owned(),
        lastname: new_user.lastname.to_owned(),
        email: new_user.email.to_owned(),
        pw_hash: new_user.pw_hash.to_owned(),
        role: Some(Role::BaseUser),
    };
    let user_detail = data.create().await;
    match user_detail {
        Ok(user) => HttpResponse::Ok().json(DefaultResponse {
            result: Some(ResultDataType::InsertOneResult(user)),
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
