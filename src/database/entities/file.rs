use crate::database::database::MyDBModel;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub parent_id: ObjectId,
    pub user_id: ObjectId,
    pub uuid: String,
    pub hash: String,
    pub mime: String,
    pub name: String,
    pub finished: bool,
    pub creation_date: DateTime,
}

impl MyDBModel for File {
    fn type_name() -> &'static str {
        "File"
    }
}
