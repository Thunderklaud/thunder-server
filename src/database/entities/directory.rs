use crate::database::daos::file_dao::FileDAO;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

use crate::database::database::MyDBModel;
use crate::database::entities::file::File;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    // parent_id needs to be option because root dir has no parent_id
    pub parent_id: Option<ObjectId>,
    pub name: String,
    pub creation_date: DateTime,
    pub child_ids: Vec<ObjectId>,
}

impl MyDBModel for Directory {
    fn type_name() -> &'static str {
        "Directory"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryPost {
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryPatch {
    pub id: ObjectId,
    // the document id of the directory that should be updated
    pub name: Option<String>,
    // null or the new name
    pub parent_id: Option<String>, // null or the new parent directory document id
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryGet {
    pub id: Option<String>,
}

#[derive(Deserialize)]
pub struct GetDirectoryArchiveQueryParams {
    pub id: Option<String>,
    pub archive: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryDelete {
    pub id: String,
}

#[derive(Serialize)]
pub struct DirectoryGetResponse {
    pub dirs: Vec<DirectoryGetResponseObject>,
    pub files: Vec<File>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryGetResponseObject {
    pub id: ObjectId,
    pub name: String,
    pub child_dir_count: u64,
    pub child_file_count: u64,
    pub creation_date_ts: i64,
}

impl Directory {
    pub async fn get_files(&self) -> Vec<File> {
        if let Some(id) = self.id {
            return FileDAO::get_files_by_parent_id(id)
                .await
                .unwrap_or(Vec::new());
        }
        Vec::new()
    }
    pub async fn has_file_with_name(&self, name: &String) -> bool {
        for file in self.get_files().await {
            if file.name.eq(name) {
                return true;
            }
        }
        false
    }
}
