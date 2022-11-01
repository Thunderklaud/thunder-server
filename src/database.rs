use mongodb::{Client, Collection, Database};

use crate::SETTINGS;

pub trait MyDBModel {}

pub async fn establish_connection() -> Option<Database> {
    let settings = SETTINGS.get().unwrap();
    let client = Client::with_uri_str(&settings.database.url).await;
    if client.is_err() {
        return None;
    }
    Some(client.unwrap().database(&settings.database.name))
}

pub async fn get_collection(cname: &str) -> Collection<Box<dyn MyDBModel>> {
    let db = establish_connection().await.unwrap();
    db.collection(cname)
}
