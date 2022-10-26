use std;
use std::env;

use mongodb::{Client, options::ClientOptions, ThreadedClient};
use crate::SETTINGS;

pub async fn establish_connection() -> Client {
    let settings = SETTINGS.get().unwrap();
    let mut client_options = ClientOptions::parse(&settings.database.url).await?;

    let client = Client::with_options(client_options)?;

    client
}
