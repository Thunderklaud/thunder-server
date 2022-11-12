use actix_jwt_authc::Authenticated;
use anyhow::anyhow;
use anyhow::Result;
use std::borrow::Borrow;
use std::str::FromStr;

use futures::StreamExt;
use mongodb::bson::{doc, DateTime};
use mongodb::results::UpdateResult;
use mongodb::{
    bson::{extjson::de::Error, oid::ObjectId},
    results::InsertOneResult,
    Collection,
};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::database::MyDBModel;
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
    pub files: Vec<String>,
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
    pub async fn create_user_root_dir(user_id: ObjectId) -> Option<ObjectId> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();
        let dir = col
            .find_one(
                doc! {
                    "name": ROOT_DIR_NAME.to_owned(),
                    "user_id": user_id
                },
                None,
            )
            .await;

        if dir.is_err() || (dir.is_ok() && dir.as_ref().unwrap().is_none()) {
            // root dir for user does not exist yet
            let mut new_dir = Directory {
                id: None,
                user_id,
                parent_id: None,
                name: ROOT_DIR_NAME.to_owned(),
                creation_date: DateTime::now(),
                child_ids: vec![],
                files: vec![],
            };

            let dir_detail = new_dir.create().await;

            if dir_detail.is_ok() {
                return dir_detail.unwrap().inserted_id.as_object_id();
            }
            return None;
        }

        // root dir for user already exists
        return Some(dir.unwrap().unwrap().id.unwrap());
    }
    pub async fn create(&mut self) -> Result<InsertOneResult, Error> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();
        let dir = col
            .insert_one(self.borrow(), None)
            .await
            .expect("Error creating directory");

        self.id = dir.inserted_id.as_object_id();

        if self.id.is_some() && self.parent_id.is_some() {
            Directory::add_child_by_oid(
                self.parent_id.to_owned().unwrap(),
                self.id.unwrap(),
                self.user_id,
            )
            .await;
        }

        Ok(dir)
    }
    async fn add_child_by_oid(
        parent_oid: ObjectId,
        child_oid: ObjectId,
        user_id: ObjectId,
    ) -> bool {
        let parent = Directory::get_by_oid(parent_oid, user_id).await;
        if parent.is_some() {
            let mut parent = parent.unwrap();
            parent.child_ids.push(child_oid);
            if parent.update().await.modified_count > 0 {
                return true;
            }
            event!(
                Level::INFO,
                "error adding child to directory {}?",
                parent.id.unwrap()
            );
        }
        false
    }
    async fn remove_child_by_oid(
        parent_oid: ObjectId,
        child_oid: ObjectId,
        user_id: ObjectId,
    ) -> bool {
        let parent = Directory::get_by_oid(parent_oid, user_id).await;
        if parent.is_some() {
            let mut parent = parent.unwrap();
            let index = parent
                .child_ids
                .iter()
                .position(|x| *x == child_oid)
                .unwrap();
            parent.child_ids.remove(index);
            if parent.update().await.modified_count > 0 {
                return true;
            }
            event!(
                Level::INFO,
                "error removing child from directory {}?",
                parent.id.unwrap()
            );
        }
        false
    }
    pub async fn get_by_oid(oid: ObjectId, user_id: ObjectId) -> Option<Directory> {
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
        .expect("Directory not found")
    }
    pub async fn get_all_with_parent_id(
        parent_id: Option<ObjectId>,
    ) -> Vec<MinimalDirectoryObject> {
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
            .expect("Directories by parent_id not found");

        let mut dir_names: Vec<MinimalDirectoryObject> = vec![];
        while let Some(dir) = cursor.next().await {
            if dir.is_ok() {
                dir_names.push(MinimalDirectoryObject {
                    id: dir.to_owned().unwrap().id.unwrap(),
                    name: dir.to_owned().unwrap().name,
                });
            }
        }
        dir_names
    }
    pub async fn update(&mut self) -> UpdateResult {
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
        .expect("Error updating directory")
    }
    pub async fn has_user_permission(directory_id: ObjectId, user_id: ObjectId) -> bool {
        Directory::get_by_oid(directory_id, user_id)
            .await
            .unwrap()
            .user_id
            == user_id
    }
    pub async fn move_to(
        &mut self,
        new_parent_oid: ObjectId,
        _authenticated: &Authenticated<Claims>,
    ) -> Result<()> {
        let col: Collection<Directory> = database::get_collection("Directory")
            .await
            .clone_with_type();

        if self.id.is_some() && self.parent_id.is_some() {
            // do not move if parent_id and new_parent_id are equal or if someone tries to move root
            // todo: does this check really work?
            if self.parent_id.unwrap() == new_parent_oid {
                return Err(anyhow!(
                    "moving from current parent to current parent is not allowed"
                ));
            } else if self.id.unwrap() == new_parent_oid {
                return Err(anyhow!("moving directory into it self is not allowed"));
            } else if _authenticated.claims.thunder_root_dir_id == self.id.unwrap() {
                return Err(anyhow!("moving user root directory is not allowed"));
            } else if !Directory::has_user_permission(
                new_parent_oid,
                ObjectId::from_str(_authenticated.claims.sub.as_str()).unwrap(),
            )
            .await
            {
                return Err(anyhow!(
                    "no permission to access the requested parent directory"
                ));
            }

            // give dir the new parent id
            col.update_one(
                doc! {
                    "_id": self.id.unwrap()
                },
                doc! {
                    "$set": {
                        "parent_id": new_parent_oid
                    }
                },
                None,
            )
            .await
            .expect("Error giving dir a new parent_id");

            // add dir as child id to the new parent
            Directory::add_child_by_oid(new_parent_oid, self.id.unwrap(), self.user_id).await;

            // remove child id from old parent
            Directory::remove_child_by_oid(self.parent_id.unwrap(), self.id.unwrap(), self.user_id)
                .await;

            self.parent_id = Some(new_parent_oid);
            return Ok(());
        }
        Err(anyhow!("no permission or directory does not exist"))
    }
}
