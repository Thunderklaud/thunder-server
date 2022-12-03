
use mongodb::bson::{doc, DateTime};
use mongodb::{bson::oid::ObjectId};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::database::database::MyDBModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub parent_id: Option<ObjectId>,
    // needs to be option because root dir has no parent_id
    pub name: String,
    pub creation_date: DateTime,
    pub child_ids: Vec<ObjectId>,
    pub files: Vec<String>, //serde serialized DirFile in Vec
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirFile {
    pub uuid: String,
    pub hash: String,
    pub mime: String,
    pub name: String,
    pub finished: bool,
    pub creation_date: DateTime,
}

impl MyDBModel for Directory {}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct MinimalDirectoryObject {
    pub id: ObjectId,
    pub name: String,
}

impl Directory {
    pub async fn get_files(&self) -> Vec<DirFile> {
        let mut dir_files: Vec<DirFile> = Vec::new();

        let iter = self.files.iter();
        for filestr in iter {
            let dir_file: Result<DirFile, _> = serde_json::from_value(filestr.parse().unwrap());
            if let Ok(dir_file) = dir_file {
                dir_files.push(dir_file);
            }
        }

        dir_files
    }

    pub async fn get_dirfile_by_uuid(&mut self, uuid: String) -> Option<DirFile> {
        let dir_files = self.get_files().await;
        for dir_file in dir_files {
            println!("{:?}", dir_file);

            if uuid.eq(&dir_file.uuid) {
                return Some(dir_file);
            }
        }

        return None;
    }

    pub async fn get_files_index_by_file_uuid(&self, uuid: &String) -> Option<usize> {
        self.files.iter().position(|x| {
            let dir_file: Result<DirFile, _> = serde_json::from_value(x.parse().unwrap());
            if let Ok(dir_file) = dir_file {
                return dir_file.uuid.eq(uuid);
            }
            false
        })
    }

    pub async fn get_first_file_index_by_file_name(&self, name: &String) -> Option<usize> {
        self.files.iter().position(|x| {
            let dir_file: Result<DirFile, _> = serde_json::from_value(x.parse().unwrap());
            if let Ok(dir_file) = dir_file {
                return dir_file.name.eq(name);
            }
            false
        })
    }

    /*
   pub async fn find_virtfile_by_name(&mut self, name: String) -> Option<VirtualFile> {
       if let Some(id) = self.id {
           let dir_files = self.get_files().await;
           for dir_file in dir_files {
               println!("{:?}", dir_file);

               if name.eq(&dir_file.name) {
                   return Some(VirtualFile {
                       parent_id: id,
                       user_id: self.user_id,
                       uuid: dir_file.uuid,
                       hash: dir_file.hash,
                       name: dir_file.name,
                       finished: dir_file.finished,
                       creation_date: dir_file.creation_date
                   });
               }
           }
       }

       return None;
   }
   */
}