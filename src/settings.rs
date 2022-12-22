use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use tracing::{event, instrument, Level};

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Database {
    pub url: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Server {
    pub url: String,
    pub port: u16,
    pub address: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub app_name: String,
    pub verbose: u8,
    pub debug: bool,
    pub database: Database,
    pub server: Server,
    pub jwt_secret: String,
    pub upload_path: String,
    pub enable_public_registration: bool,
}

impl Settings {
    #[instrument]
    pub fn new() -> Result<Self, ConfigError> {
        event!(Level::INFO, "generate new settings");

        let run_mode = String::from(if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        });

        let s = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name("config/default"))
            // Add in the current environment file
            // Default to 'development' env
            // Note that this file is _optional_
            .add_source(File::with_name(format!("config/{}", run_mode).as_str()).required(false))
            // Add in a local configuration file
            // This file shouldn't be checked in to git
            .add_source(File::with_name("config/local.example").required(false))
            .add_source(File::with_name("config/local").required(false))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(Environment::with_prefix("app"))
            // You may also programmatically change settings
            //.set_override("database.url", "postgres://")?
            .build()?;

        // Now that we're done, let's access our configuration
        //println!("debug: {:?}", s.get_bool("debug"));
        //println!("database: {:?}", s.get::<String>("database.url"));

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init() {
        let settings_result = Settings::new();
        assert_eq!(settings_result.is_ok(), true);
    }

    #[test]
    fn get_bool_value() {
        let settings = Settings::new().unwrap();
        assert_eq!(settings.debug, true);
    }
}
