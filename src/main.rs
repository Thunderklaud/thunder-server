use actix_web::{web, App, HttpServer};
use anyhow::Result;
use once_cell::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing::{event, Level};

mod controller;
mod database;
mod model;
mod settings;

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

    HttpServer::new(|| {
        App::new().service(web::scope("/v1").service(
            web::scope("/user").route("/registration", web::post().to(controller::user::register)),
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
