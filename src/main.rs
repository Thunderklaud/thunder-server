use actix_jwt_authc::AuthenticateMiddlewareFactory;
use actix_web::{web, App, HttpServer};
use actix_web::web::Data;
use anyhow::Result;
use once_cell::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};
use crate::jwt_utils::{Claims, get_auth_middleware_settings, get_jwt_ttl, InvalidatedJWTStore, JwtSigningKeys};

mod controller;
mod database;
mod model;
mod settings;
mod jwt_utils;

static SETTINGS: OnceCell<settings::Settings> = OnceCell::new();

#[actix_web::main]
async fn main() -> Result<()> {
    let settings = settings::Settings::new()?;
    SETTINGS.set(settings).unwrap();
    let settings = SETTINGS.get().unwrap();

    tracing_subscriber::fmt()
        .with_max_level(match &settings.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        })
        .init();

    event!(Level::INFO, "tracing_subscriber initialized in main");

    // Print out our settings
    println!("{:?}", SETTINGS);

    let jwt_signing_keys = JwtSigningKeys::generate().unwrap();
    let auth_middleware_settings = get_auth_middleware_settings(&jwt_signing_keys);

    let (invalidated_jwts_store, stream) = InvalidatedJWTStore::new_with_stream();
    let auth_middleware_factory = AuthenticateMiddlewareFactory::<Claims>::new(
        stream,
        auth_middleware_settings.clone(),
    );

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(invalidated_jwts_store.clone()))
            .app_data(Data::new(jwt_signing_keys.encoding_key.clone()))
            .app_data(Data::new(get_jwt_ttl()))
            .wrap(auth_middleware_factory.clone())
            .service(web::scope("/v1").service(
            web::scope("/user")
                .route("/login", web::post().to(controller::user::login))
                .route("/registration", web::post().to(controller::user::register))
                .route("/test", web::get().to(controller::user::test)),
        ))
    })
    .workers(2)
    .bind((settings.server.address.as_str(), settings.server.port))?
    .run()
    .await
    .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_settings() {
        let settings_result = settings::Settings::new();
        assert_eq!(settings_result.is_ok(), true);
    }
}
