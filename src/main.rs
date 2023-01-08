use crate::jwt_utils::{
    get_auth_middleware_settings, get_jwt_ttl, Claims, InvalidatedJWTStore, JwtSigningKeys,
};
use crate::storage::storage_provider::StorageProvider;
use actix_cors::Cors;
use actix_jwt_authc::AuthenticateMiddlewareFactory;
use actix_web::web::Data;
use actix_web::{http, web, App, HttpServer};
use anyhow::Result;
use once_cell::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};

extern crate strum_macros;

mod cmd;
mod controller;
mod database;
mod jwt_utils;
mod settings;
mod storage;

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

    StorageProvider::init(settings)?;

    cmd::process().await;

    let jwt_signing_keys = if (&settings).jwt_secret.len() > 20 {
        JwtSigningKeys::parse((&settings).jwt_secret.as_str()).unwrap()
    } else {
        JwtSigningKeys::generate().unwrap()
    };
    let auth_middleware_settings = get_auth_middleware_settings(&jwt_signing_keys);

    let (invalidated_jwt_store, stream) = InvalidatedJWTStore::new_with_stream();
    let auth_middleware_factory =
        AuthenticateMiddlewareFactory::<Claims>::new(stream, auth_middleware_settings.clone());

    HttpServer::new(move || {
        let allowed_cors_origins = (&settings).allowed_cors_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                let bytes_origin = origin.as_bytes();
                allowed_cors_origins
                    .iter()
                    .any(|allowed_origin| bytes_origin.eq((*allowed_origin).as_bytes()))
            })
            .allowed_methods(vec!["GET", "POST", "DELETE", "PUT", "PATCH", "OPTIONS"])
            .allowed_headers(vec![
                http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                http::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                http::header::ACCEPT,
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
                http::header::VARY,
            ])
            .supports_credentials()
            .max_age(60); // see https://fetch.spec.whatwg.org/#http-access-control-max-age

        App::new()
            .app_data(Data::new(invalidated_jwt_store.clone()))
            .app_data(Data::new(jwt_signing_keys.encoding_key.clone()))
            .app_data(Data::new(get_jwt_ttl()))
            .wrap(cors)
            .wrap(auth_middleware_factory.clone())
            .service(
                web::scope("/v1")
                    .service(
                        web::scope("/user")
                            .route("/login", web::post().to(controller::user::login))
                            .route("/logout", web::post().to(controller::user::logout))
                            .route("/registration", web::post().to(controller::user::register))
                            .route("/test", web::get().to(controller::user::test))
                            .route("/syncstate", web::get().to(controller::syncstate::get))
                            .route(
                                "/shares",
                                web::get().to(controller::share::get_share_infos_for_user),
                            ),
                    )
                    .service(
                        web::scope("/data")
                            .route("/directory", web::post().to(controller::directory::create))
                            .route("/directory", web::patch().to(controller::directory::update))
                            .route(
                                "/directory",
                                web::delete().to(controller::directory::delete),
                            )
                            .route("/directory", web::get().to(controller::directory::get))
                            .route("/file", web::put().to(controller::file::multi_upload))
                            .route("/file", web::patch().to(controller::file::update))
                            .route("/file", web::delete().to(controller::file::delete))
                            .service(
                                web::scope("/download")
                                    .route("/file", web::get().to(controller::file::get_single)),
                            ),
                    )
                    .service(
                        web::scope("/share")
                            .route("/", web::get().to(controller::share::get_share_info))
                            .route("/", web::delete().to(controller::share::delete_share))
                            .route("/download", web::get().to(controller::share::download))
                            .route(
                                "/file",
                                web::post().to(controller::share::create_file_share),
                            )
                            .route(
                                "/file",
                                web::get().to(controller::share::get_share_infos_for_file),
                            ),
                    ),
            )
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
