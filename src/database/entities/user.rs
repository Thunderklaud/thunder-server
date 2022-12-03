use mongodb::bson::{doc, DateTime};
use mongodb::{bson::oid::ObjectId};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use strum_macros::AsRefStr;
use crate::database::database::MyDBModel;

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
