use std::ops::Add;

use actix_jwt_authc::Authenticated;
use actix_web::web::Data;
use actix_web::{web::Json, HttpResponse};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use time::OffsetDateTime;
use tracing::{event, Level};

use crate::database::daos::dao::DAO;
use crate::database::daos::directory_dao::DirectoryDAO;
use crate::database::daos::user_dao::UserDAO;
use crate::database::entities::user::{Role, User, UserLogin, UserRegister};
use crate::jwt_utils::{JWTTtl, JWT_SIGNING_ALGO};
use crate::{Claims, InvalidatedJWTStore};

#[derive(Serialize)]
struct LoginResponse {
    jwt: String,
}

#[derive(Serialize)]
struct LogoutResponse {
    status: bool,
}

#[derive(Serialize)]
struct TestResponse {
    session_info: Authenticated<Claims>,
    email: String,
}

pub async fn login(
    login_user: Json<UserLogin>,
    jwt_encoding_key: Data<EncodingKey>,
    jwt_ttl: Data<JWTTtl>,
) -> actix_web::Result<HttpResponse> {
    event!(Level::INFO, "login_user: {}", login_user.email);

    let user = UserDAO::get_by_email(login_user.email.as_str()).await?;

    if let Some(user) = user {
        if let (Some(id), Some(root_dir_id)) = (user.id, user.root_dir_id) {
            let sub = id.to_string();
            let thunder_root_dir_id = root_dir_id;
            let iat = OffsetDateTime::now_utc().unix_timestamp() as usize;
            let expires_at = OffsetDateTime::now_utc().add(jwt_ttl.0);
            let exp = expires_at.unix_timestamp() as usize;

            let jwt_claims = Claims {
                iat,
                exp,
                sub,
                thunder_root_dir_id,
            };
            let jwt_token = encode(
                &Header::new(JWT_SIGNING_ALGO),
                &jwt_claims,
                &jwt_encoding_key,
            )
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            return Ok(HttpResponse::Ok().json(LoginResponse { jwt: jwt_token }));
        }
    }

    return Err(actix_web::error::ErrorNotFound(
        "User with email does not exist",
    ));
}

pub async fn test(_authenticated: Authenticated<Claims>) -> actix_web::Result<HttpResponse> {
    if let Some(user) = UserDAO::get_authenticated(&_authenticated).await? {
        return Ok(HttpResponse::Ok().json(TestResponse {
            session_info: _authenticated.clone(),
            email: user.email,
        }));
    }
    Err(actix_web::error::ErrorExpectationFailed(
        "authenticated user not found in database",
    ))
}

pub async fn logout(
    _authenticated: Authenticated<Claims>,
    invalidated_jwts: Data<InvalidatedJWTStore>,
) -> HttpResponse {
    HttpResponse::Ok().json(LogoutResponse {
        status: invalidated_jwts.add_to_invalidated(_authenticated).await,
    })
}

pub async fn register(new_user: Json<UserRegister>) -> actix_web::Result<HttpResponse> {
    if !User::is_valid_hash_design(new_user.pw_hash.to_owned().as_str()) {
        // not a hex encoded hash or less than 256 bit size
        return Err(actix_web::error::ErrorExpectationFailed(
            "Please provide at least a hex encoded sha256 hash",
        ));
    }

    if UserDAO::exists(&new_user.email).await? {
        return Err(actix_web::error::ErrorExpectationFailed(
            "User with email already exists",
        ));
    }

    let mut data = User {
        id: None,
        firstname: new_user.firstname.to_owned(),
        lastname: new_user.lastname.to_owned(),
        email: new_user.email.to_owned(),
        pw_hash: new_user.pw_hash.to_owned(),
        role: Role::BaseUser,
        root_dir_id: None,
    };

    let inserted_user_id = UserDAO::insert(&mut data)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let root_dir_id = DirectoryDAO::create_user_root_dir(inserted_user_id).await?;

    data.root_dir_id = Some(root_dir_id);
    UserDAO::update(&data).await?;

    Ok(HttpResponse::Ok().json(inserted_user_id))
}
