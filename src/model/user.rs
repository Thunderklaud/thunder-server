use serde::{Deserialize, Serialize};
use mongodb::{
    bson::{
        extjson::de::Error,
        oid::ObjectId,
    },
    results::{InsertOneResult},
    Collection,
};

use crate::database;
use crate::database::MyDBModel;

// Define a model. Simple as deriving a few traits.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub location: String,
    pub title: String,
}

impl MyDBModel for User {}
impl User {
    pub async fn create(&mut self) -> Result<InsertOneResult, Error> {
        //let db = database::establish_connection().await.unwrap();
        //let col: Collection<User> = db.collection("User");
        let col: Collection<User> = database::get_collection("User").await.clone_with_type();
        let user = col.insert_one(self, None)
            .await
            .expect("Error creating user");
        Ok(user)
    }
}
