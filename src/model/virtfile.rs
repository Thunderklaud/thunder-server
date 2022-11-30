use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

use crate::model::directory::DirFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualFile {
    pub parent_id: ObjectId,
    pub user_id: ObjectId,
    pub uuid: String,
    pub hash: String,
    pub mime: String,
    pub name: String,
    pub finished: bool,
    pub creation_date: DateTime,
}

impl VirtualFile {
    pub fn as_dir_file(&self) -> DirFile {
        DirFile {
            uuid: self.uuid.to_owned(),
            hash: self.hash.to_owned(),
            mime: self.mime.to_owned(),
            name: self.name.to_owned(),
            finished: self.finished.to_owned(),
            creation_date: self.creation_date.to_owned(),
        }
    }
    pub fn as_serialized_dir_file(&self) -> String {
        serde_json::to_string(&self.as_dir_file()).unwrap()
    }
}
