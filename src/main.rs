#[macro_use]
extern crate rocket;

mod settings;

use once_cell::sync::OnceCell;
use rocket::{Request, Route, route};
use rocket::http::{Method::{Get, Patch, Post, Put, Delete}};

static SETTINGS: OnceCell<settings::Settings> = OnceCell::new();

#[rocket::main]
#[allow(unused_must_use)]
async fn main() {
    let settings = settings::Settings::new().unwrap();
    SETTINGS.set(settings).unwrap();

    // Print out our settings
    println!("{:?}", SETTINGS);
    /*
    rocket::build()
        .mount("/v1/user", vec![
            Route::new(Post, "/login", controller::user::login),
            Route::new(Post, "/logout", controller::user::logout),
            Route::new(Post, "/registration", controller::user::register),
        ])
        .mount("/v1/data", vec![
            Route::new(Get, "/test", controller::file::test),
            Route::new(Post, "/file", controller::file::create),
            Route::new(Put, "/file", controller::file::upload),
            Route::new(Get, "/file", controller::file::download),
        ])
        .launch().await;*/
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
