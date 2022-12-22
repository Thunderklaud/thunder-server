use crate::cmd::update_user_role;
use crate::database::entities::user::Role;
use crate::jwt_utils::{
    get_auth_middleware_settings, get_jwt_ttl, Claims, InvalidatedJWTStore, JwtSigningKeys,
};
use crate::storage::storage_provider::StorageProvider;
use actix_jwt_authc::AuthenticateMiddlewareFactory;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use getopt::Opt;
use once_cell::sync::OnceCell;
use std::env;
use std::process::exit;
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

    // cmd here
    let args: Vec<String> = env::args().collect();
    let mut opts = getopt::Parser::new(&args, "ra:u:h");
    let mut run_server_after_cmd_execution = false;

    if args.len() <= 1 {
        // Print out our settings
        println!("{:#?}", SETTINGS);

        run_server_after_cmd_execution = true;
    }

    loop {
        match opts.next().transpose()? {
            None => {
                break;
            }
            Some(opt) => match opt {
                Opt('r', None) => run_server_after_cmd_execution = true,
                Opt('a', Some(string)) => {
                    update_user_role(string.clone(), Role::Admin).await.unwrap()
                }
                Opt('u', Some(string)) => update_user_role(string.clone(), Role::BaseUser)
                    .await
                    .unwrap(),
                Opt('h', None) => {
                    print_help();
                    return Ok(());
                }
                _ => {
                    print_help();
                    return Ok(());
                }
            },
        }
    }

    if !run_server_after_cmd_execution {
        exit(0);
    }

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
        App::new()
            .app_data(Data::new(invalidated_jwt_store.clone()))
            .app_data(Data::new(jwt_signing_keys.encoding_key.clone()))
            .app_data(Data::new(get_jwt_ttl()))
            .wrap(auth_middleware_factory.clone())
            .service(
                web::scope("/v1")
                    .service(
                        web::scope("/user")
                            .route("/login", web::post().to(controller::user::login))
                            .route("/logout", web::post().to(controller::user::logout))
                            .route("/registration", web::post().to(controller::user::register))
                            .route("/test", web::get().to(controller::user::test))
                            .route("/syncstate", web::get().to(controller::syncstate::get)),
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
                    ),
            )
    })
    .workers(2)
    .bind((settings.server.address.as_str(), settings.server.port))?
    .run()
    .await
    .map_err(anyhow::Error::from)
}

fn print_help() {
    println!("Run command without options to start run server");

    println!("Command line options:");
    println!(
        "-r        run server after cmd execution (default: false = stop after cmd execution)"
    );
    println!("-a id     add administrator role to user by id");
    println!("-u id     remove administrator role from user by id");
    println!("-h        print this help block");
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
