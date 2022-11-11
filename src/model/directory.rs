use std::borrow::Borrow;
use std::str::FromStr;

use mongodb::{
    bson::{extjson::de::Error, oid::ObjectId},
    Collection,
    results::InsertOneResult,
};
use mongodb::bson::{DateTime, doc};
use mongodb::results::UpdateResult;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::{Claims, database};
use crate::database::MyDBModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub parent_id: Option<ObjectId>,
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
    pub id: ObjectId,                   // the document id of the directory that should be updated
    pub name: Option<String>,           // null or the new name
    pub parent_id: Option<String>,      // null or the new parent directory document id
}

impl Directory {
    pub async fn create(&mut self) -> Result<InsertOneResult, Error> {
        let col: Collection<Directory> = database::get_collection("Directory").await.clone_with_type();
        let dir = col
            .insert_one(self.borrow(), None)
            .await
            .expect("Error creating directory");

        self.id = dir.inserted_id.as_object_id();

        if self.id.is_some() && self.parent_id.is_some() {
            Directory::add_child_by_oid(self.parent_id.to_owned().unwrap(), self.id.unwrap()).await;
        }

        Ok(dir)
    }
    async fn add_child_by_oid(parent_oid: ObjectId, child_oid: ObjectId) -> bool {
        let parent = Directory::get_by_oid(parent_oid).await;
        if parent.is_some() {
            let mut parent = parent.unwrap();
            parent.child_ids.push(child_oid);
            if parent.update().await.modified_count > 0 {
                return true;
            }
            event!(Level::INFO, "error adding child to directory {}?", parent.id.unwrap());
        }
        false
    }
    async fn remove_child_by_oid(parent_oid: ObjectId, child_oid: ObjectId) -> bool {
        let parent = Directory::get_by_oid(parent_oid).await;
        if parent.is_some() {
            let mut parent = parent.unwrap();
            let index = parent.child_ids.iter().position(|x| *x == child_oid).unwrap();
            parent.child_ids.remove(index);
            if parent.update().await.modified_count > 0 {
                return true;
            }
            event!(Level::INFO, "error removing child from directory {}?", parent.id.unwrap());
        }
        false
    }
    pub async fn get_by_oid_str(oid: &str) -> Option<Directory> {
        Directory::get_by_oid(ObjectId::from_str(oid).unwrap()).await
    }
    pub async fn get_by_oid(oid: ObjectId) -> Option<Directory> {
        let col: Collection<Directory> = database::get_collection("Directory").await.clone_with_type();
        col.find_one(
            doc! {
                "_id": oid
            },
            None,
        )
            .await
            .expect("Directory not found")
    }
    pub async fn update(&mut self) -> UpdateResult {
        let col: Collection<Directory> = database::get_collection("Directory").await.clone_with_type();
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
    pub async fn move_to(&mut self, new_parent_oid: Option<ObjectId>) {
        let col: Collection<Directory> = database::get_collection("Directory").await.clone_with_type();

        if self.id.is_some() && (self.parent_id.is_some() || new_parent_oid.is_some()) {
            // do not move if parent_id and new_parent_id are equal
            // todo: does this check really work?
            if self.parent_id == new_parent_oid {
                return;
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
            if new_parent_oid.is_some() {
                Directory::add_child_by_oid(new_parent_oid.unwrap(), self.id.unwrap()).await;
            }

            // remove child id from old parent
            if self.parent_id.is_some() {
                Directory::remove_child_by_oid(self.parent_id.unwrap(), self.id.unwrap()).await;
            }

            self.parent_id = new_parent_oid;
        }
    }
}
