use std::borrow::Borrow;
use std::str::FromStr;

use actix_jwt_authc::Authenticated;
use mongodb::bson::doc;
use mongodb::results::UpdateResult;
use mongodb::{
    bson::{extjson::de::Error, oid::ObjectId},
    results::InsertOneResult,
    Collection,
};
use ring::test::from_hex;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::database::MyDBModel;
use crate::{database, Claims};

use strum_macros::AsRefStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub pw_hash: String,
    pub role: Role,
    pub root_dir_id: Option<ObjectId>,
}

impl MyDBModel for User {}

#[derive(Debug, Serialize, Deserialize, AsRefStr)]
pub enum Role {
    Admin,
    BaseUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLogin {
    pub email: String,
    pub pw_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRegister {
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub pw_hash: String,
}

impl User {
    pub async fn create(&mut self) -> Result<InsertOneResult, Error> {
        let col: Collection<User> = database::get_collection("User").await.clone_with_type();
        let user = col
            .insert_one(self.borrow(), None)
            .await
            .expect("Error creating user");

        self.id = user.inserted_id.as_object_id();
        Ok(user)
    }

    pub async fn get_authenticated(authenticated: &Authenticated<Claims>) -> Option<User> {
        event!(
            Level::INFO,
            "get_authenticated: {}",
            authenticated.claims.sub.as_str()
        );
        User::get_by_oid(authenticated.claims.sub.as_str()).await
    }

    pub async fn get_by_oid(oid: &str) -> Option<User> {
        let col: Collection<User> = database::get_collection("User").await.clone_with_type();
        col.find_one(
            doc! {
                "_id": ObjectId::from_str(oid).unwrap()
            },
            None,
        )
        .await
        .expect("User not found")
    }

    pub async fn get_by_email(email: &str) -> Option<User> {
        let col: Collection<User> = database::get_collection("User").await.clone_with_type();
        col.find_one(
            doc! {
                "email": email
            },
            None,
        )
        .await
        .expect("User not found")
    }

    pub async fn exists(email: &String) -> bool {
        User::get_by_email(email.to_owned().as_str())
            .await
            .is_some()
    }

    pub async fn update(&mut self) -> UpdateResult {
        let col: Collection<User> = database::get_collection("User").await.clone_with_type();
        col.update_one(
            doc! {
                "_id": self.id.unwrap()
            },
            doc! {
                "$set": {
                    "firstname": self.firstname.to_owned(),
                    "lastname": self.lastname.to_owned(),
                    "email": self.email.to_owned(),
                    "pw_hash": self.pw_hash.to_owned(),
                    "role": self.role.as_ref(),
                    "root_dir_id": self.root_dir_id.to_owned(),
                }
            },
            None,
        )
        .await
        .expect("Error updating user")
    }

    pub fn is_valid_hash_design(hash: &str) -> bool {
        let pw_bytes_res = from_hex(hash);

        // sha256 requires 32 bytes = 256 bit
        // sha512 requires 64 bytes = 512 bit
        pw_bytes_res.is_ok() && pw_bytes_res.to_owned().unwrap().len() >= 32
    }
}
