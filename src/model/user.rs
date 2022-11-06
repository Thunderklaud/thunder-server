use mongodb::{
    bson::{extjson::de::Error, oid::ObjectId},
    results::InsertOneResult,
    Collection,
};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::database;
use crate::database::MyDBModel;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub pw_hash: String,
    pub role: Option<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Admin,
    BaseUser,
}

impl MyDBModel for User {}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLogin {
    pub email: String,
    pub pw_hash: String,
}

impl User {
    pub async fn create(&mut self) -> Result<InsertOneResult, Error> {
        //let db = database::establish_connection().await.unwrap();
        //let col: Collection<User> = db.collection("User");
        let col: Collection<User> = database::get_collection("User").await.clone_with_type();
        let user = col
            .insert_one(self, None)
            .await
            .expect("Error creating user");
        Ok(user)
    }

    pub async fn get_by_email(email: &str) -> Option<User> {
        let col: Collection<User> = database::get_collection("User").await.clone_with_type();
        col.find_one(
            doc! {
                "email": email
            },
            None,
        ).await.expect("User not found")
    }

    pub async fn exists(email: &String) -> bool {
        User::get_by_email(email.to_owned().as_str()).await.is_some()
    }
}
