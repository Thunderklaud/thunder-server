use std::borrow::Borrow;

use actix_jwt_authc::Authenticated;
use futures::StreamExt;
use mongodb::bson::{doc, DateTime};
use mongodb::results::UpdateResult;
use mongodb::{bson::oid::ObjectId, results::InsertOneResult, Collection};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::database::MyDBModel;
use crate::jwt_utils::extract_user_oid;
use crate::model::virtfile::VirtualFile;
use crate::{database, Claims};

static ROOT_DIR_NAME: &str = "/";

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
    pub async fn create_user_root_dir(user_id: ObjectId) -> actix_web::Result<ObjectId> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();
        let dir = col
            .find_one(
                doc! {
                    "name": ROOT_DIR_NAME,
                    "user_id": user_id
                },
                None,
            )
            .await;

        if let Ok(dir_opt) = dir {
            if let Some(dir) = dir_opt {
                // root dir for user already exists
                return Ok(dir
                    .id
                    .expect("could not extract id from database directory"));
            }
        }

        // root dir for user does not exist yet
        let mut new_dir = Directory {
            id: None,
            user_id,
            parent_id: None,
            name: ROOT_DIR_NAME.parse()?,
            creation_date: DateTime::now(),
            child_ids: vec![],
            files: vec![],
        };

        let dir_detail = new_dir.create().await?;
        Ok(dir_detail.inserted_id.as_object_id().ok_or_else(|| {
            actix_web::error::ErrorInternalServerError("creating root dir failed")
        })?)
    }
    pub async fn create(&mut self) -> actix_web::Result<InsertOneResult> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();
        let dir = col
            .insert_one(self.borrow(), None)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        self.id = dir.inserted_id.as_object_id();

        if let (Some(id), Some(parent_id)) = (self.id, self.parent_id) {
            Directory::add_child_by_oid(parent_id, id, self.user_id).await?;
        }

        Ok(dir)
    }
    async fn add_child_by_oid(
        parent_oid: ObjectId,
        child_oid: ObjectId,
        user_id: ObjectId,
    ) -> actix_web::Result<()> {
        let parent = Directory::get_by_oid(parent_oid, user_id).await?;
        if let Some(mut parent) = parent {
            parent.child_ids.push(child_oid);
            if parent.update().await?.modified_count > 0 {
                return Ok(());
            }
            event!(
                Level::INFO,
                "error adding child to directory {}?",
                parent_oid
            );
        }
        Err(actix_web::error::ErrorNotFound(
            "could not get parent directory",
        ))
    }
    async fn remove_child_by_oid(
        parent_oid: ObjectId,
        child_oid: ObjectId,
        user_id: ObjectId,
    ) -> actix_web::Result<()> {
        let parent = Directory::get_by_oid(parent_oid, user_id).await?;
        if let Some(mut parent) = parent {
            let index = parent
                .child_ids
                .iter()
                .position(|x| *x == child_oid)
                .unwrap();
            parent.child_ids.remove(index);
            if parent.update().await?.modified_count > 0 {
                return Ok(());
            }
            event!(
                Level::INFO,
                "error removing child from directory {}?",
                parent_oid
            );
        }
        Err(actix_web::error::ErrorNotFound(
            "could not get parent directory",
        ))
    }
    pub async fn get_by_oid(
        oid: ObjectId,
        user_id: ObjectId,
    ) -> actix_web::Result<Option<Directory>> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();
        col.find_one(
            doc! {
                "_id": oid,
                "user_id": user_id
            },
            None,
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }
    pub async fn get_all_with_parent_id(
        parent_id: Option<ObjectId>,
    ) -> actix_web::Result<Vec<MinimalDirectoryObject>> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();
        let mut cursor = col
            .find(
                doc! {
                    "parent_id": parent_id
                },
                None,
            )
            .await
            .map_err(|_| actix_web::error::ErrorNotFound("Directories by parent_id not found"))?;

        let mut dir_names: Vec<MinimalDirectoryObject> = vec![];
        while let Some(dir) = cursor.next().await {
            if let Ok(dir) = dir {
                dir_names.push(MinimalDirectoryObject {
                    id: dir.id.unwrap(),
                    name: dir.name,
                });
            }
        }
        Ok(dir_names)
    }
    pub async fn update(&mut self) -> actix_web::Result<UpdateResult> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();
        col.update_one(
            doc! {
                "_id": self.id.unwrap()
            },
            doc! {
                "$set": {
                    "parent_id": self.parent_id.to_owned(),
                    "name": self.name.to_owned(),
                    "child_ids": self.child_ids.to_owned(),
                    "files": self.files.to_owned(),
                }
            },
            None,
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }
    pub async fn has_user_permission(
        directory_id: ObjectId,
        user_id: ObjectId,
    ) -> actix_web::Result<()> {
        match Directory::get_by_oid(directory_id, user_id).await? {
            Some(dir) if dir.user_id == user_id => Ok(()),
            _ => Err(actix_web::error::ErrorForbidden("missing permission")),
        }
    }
    pub async fn move_to(
        &mut self,
        new_parent_oid: ObjectId,
        _authenticated: &Authenticated<Claims>,
    ) -> actix_web::Result<()> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();

        if let (Some(id), Some(parent_id)) = (self.id, self.parent_id) {
            // do not move if parent_id and new_parent_id are equal or if someone tries to move root
            // todo: does this check really work?
            if parent_id == new_parent_oid {
                return Err(actix_web::error::ErrorInternalServerError(
                    "moving from current parent to current parent is not allowed",
                ));
            } else if id == new_parent_oid {
                return Err(actix_web::error::ErrorInternalServerError(
                    "moving directory into it self is not allowed",
                ));
            } else if id == _authenticated.claims.thunder_root_dir_id {
                return Err(actix_web::error::ErrorInternalServerError(
                    "moving user root directory is not allowed",
                ));
            }

            Directory::has_user_permission(new_parent_oid, extract_user_oid(_authenticated))
                .await
                .map_err(|_| {
                    actix_web::error::ErrorForbidden(
                        "no permission to access the requested parent directory",
                    )
                })?;

            // give dir the new parent id
            col.update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$set": {
                        "parent_id": new_parent_oid
                    }
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            // add dir as child id to the new parent
            Directory::add_child_by_oid(new_parent_oid, id, self.user_id).await?;

            // remove child id from old parent
            Directory::remove_child_by_oid(parent_id, id, self.user_id).await?;

            self.parent_id = Some(new_parent_oid);
            return Ok(());
        }
        Err(actix_web::error::ErrorInternalServerError(
            "no permission or directory does not exist",
        ))
    }
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
