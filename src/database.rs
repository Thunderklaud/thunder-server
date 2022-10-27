use crate::SETTINGS;
use wither::mongodb::{Client, Database};

pub async fn establish_connection() -> Option<Database> {
    let settings = SETTINGS.get().unwrap();
    let client = Client::with_uri_str(&settings.database.url).await;
    if client.is_err() {
        return None;
    }
    Some(client.unwrap().database(&settings.database.name))
}
