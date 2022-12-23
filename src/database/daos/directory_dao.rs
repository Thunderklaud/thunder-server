use std::borrow::Borrow;

use actix_jwt_authc::Authenticated;
use async_trait::async_trait;
use futures::StreamExt;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};
use tracing::{event, Level};

use crate::database::daos::dao::DAO;
use crate::database::daos::file_dao::FileDAO;
use crate::database::daos::syncstate_dao::SyncStateDAO;
use crate::database::entities::directory::{Directory, MinimalDirectoryObject};
use crate::database::entities::syncstate::{SyncState, SyncStateAction, SyncStateType};
use crate::jwt_utils::extract_user_oid;
use crate::storage::storage_provider::StorageProvider;
use crate::Claims;

static ROOT_DIR_NAME: &str = "/";

pub struct DirectoryDAO {}

#[async_trait]
impl DAO<Directory, ObjectId> for DirectoryDAO {
    async fn get(oid: ObjectId) -> actix_web::Result<Option<Directory>> {
        DirectoryDAO::get_collection()
            .await
            .find_one(
                doc! {
                    "_id": oid
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }

    async fn get_with_user(
        oid: ObjectId,
        user_id: ObjectId,
    ) -> actix_web::Result<Option<Directory>> {
        DirectoryDAO::get_collection()
            .await
            .find_one(
                doc! {
                    "_id": oid,
                    "user_id": user_id
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }

    async fn insert(dir: &mut Directory) -> actix_web::Result<ObjectId> {
        if Self::dir_by_name_exists_in(&dir.name, dir.parent_id.unwrap()).await? {
            return Err(actix_web::error::ErrorForbidden(
                "A directory with that name already exists in this folder",
            ));
        }

        let insert_result = DirectoryDAO::get_collection()
            .await
            .insert_one(dir.borrow(), None)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        dir.id = insert_result.inserted_id.as_object_id();
        if let Some(id) = dir.id {
            if let Some(parent_id) = dir.parent_id {
                DirectoryDAO::add_child_by_oid(parent_id, id, dir.user_id).await?;
            }

            let _ = SyncStateDAO::insert(&mut SyncState::new(
                SyncStateType::Directory,
                SyncStateAction::Create,
                id,
                dir.parent_id,
                dir.user_id,
            ))
            .await?;

            return Ok(id);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "failed converting inserted_id to ObjectId",
        ))
    }

    async fn update(dir: &Directory) -> actix_web::Result<u64> {
        if let Some(id) = dir.id {
            let update_result = DirectoryDAO::get_collection()
                .await
                .update_one(
                    doc! {
                        "_id": id
                    },
                    doc! {
                        "$set": {
                            "parent_id": dir.parent_id.to_owned(),
                            "name": dir.name.to_owned(),
                            "child_ids": dir.child_ids.to_owned(),
                        }
                    },
                    None,
                )
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            return Ok(update_result.modified_count);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "directory id not found",
        ))
    }

    async fn delete(dir: &Directory) -> actix_web::Result<u64> {
        if let Some(id) = dir.id {
            for file in dir.get_files().await {
                StorageProvider::delete_file(file.uuid.clone())?;
                FileDAO::delete(&file).await?;
            }

            let delete_result = Self::get_collection()
                .await
                .delete_one(
                    doc! {
                        "_id": id
                    },
                    None,
                )
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

            SyncStateDAO::delete_for_corresponding_id(id).await?;
            SyncStateDAO::delete_for_corresponding_parent_id(id).await?;

            let _ = SyncStateDAO::insert(&mut SyncState::new(
                SyncStateType::Directory,
                SyncStateAction::Delete,
                id,
                dir.parent_id,
                dir.user_id,
            ))
            .await?;

            return Ok(delete_result.deleted_count);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "directory id not found",
        ))
    }
}

impl DirectoryDAO {
    pub async fn get_all_with_parent_id(
        parent_id: Option<ObjectId>,
    ) -> actix_web::Result<Vec<MinimalDirectoryObject>> {
        let mut cursor = DirectoryDAO::get_collection()
            .await
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

    pub async fn dir_by_name_exists_in(
        name: &String,
        parent_id: ObjectId,
    ) -> actix_web::Result<bool> {
        let dir = DirectoryDAO::get_collection()
            .await
            .find_one(
                doc! {
                    "name": name,
                    "parent_id": parent_id
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        Ok(match dir {
            Some(_) => true,
            _ => false,
        })
    }

    pub async fn create_user_root_dir(user_id: ObjectId) -> actix_web::Result<ObjectId> {
        let dir = DirectoryDAO::get_collection()
            .await
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
        };

        Ok(DirectoryDAO::insert(&mut new_dir)
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("creating root dir failed"))?)
    }

    async fn add_child_by_oid(
        parent_oid: ObjectId,
        child_oid: ObjectId,
        user_id: ObjectId,
    ) -> actix_web::Result<()> {
        let parent = DirectoryDAO::get_with_user(parent_oid, user_id).await?;
        if let Some(mut parent) = parent {
            parent.child_ids.push(child_oid);
            if DirectoryDAO::update(&parent).await? > 0 {
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
        let parent = DirectoryDAO::get_with_user(parent_oid, user_id).await?;
        if let Some(mut parent) = parent {
            let index = parent
                .child_ids
                .iter()
                .position(|x| *x == child_oid)
                .unwrap();
            parent.child_ids.remove(index);
            if DirectoryDAO::update(&parent).await? > 0 {
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

    pub async fn has_user_permission(
        directory_id: ObjectId,
        user_id: ObjectId,
    ) -> actix_web::Result<()> {
        match DirectoryDAO::get_with_user(directory_id, user_id).await? {
            Some(dir) if dir.user_id == user_id => Ok(()),
            _ => Err(actix_web::error::ErrorForbidden("missing permission")),
        }
    }

    pub async fn rename(dir: &mut Directory, new_name: &String) -> actix_web::Result<()> {
        if Self::dir_by_name_exists_in(&new_name, dir.parent_id.unwrap()).await? {
            return Err(actix_web::error::ErrorForbidden(
                "A directory with that name already exists in this folder",
            ));
        }

        dir.name = new_name.to_string();

        let update_result = DirectoryDAO::update(dir).await?;
        if update_result <= 0 {
            event!(
                Level::DEBUG,
                "renaming directory failed {:?}",
                update_result
            );
            return Err(actix_web::error::ErrorInternalServerError(
                "Renaming directory failed",
            ));
        }

        let _ = SyncStateDAO::insert(&mut SyncState::new(
            SyncStateType::Directory,
            SyncStateAction::Rename,
            dir.id.unwrap(),
            dir.parent_id,
            dir.user_id,
        ))
        .await?;

        Ok(())
    }

    pub async fn move_to(
        dir: &mut Directory,
        new_parent_oid: ObjectId,
        _authenticated: &Authenticated<Claims>,
    ) -> actix_web::Result<()> {
        if let (Some(id), Some(parent_id)) = (dir.id, dir.parent_id) {
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

            DirectoryDAO::has_user_permission(new_parent_oid, extract_user_oid(_authenticated))
                .await
                .map_err(|_| {
                    actix_web::error::ErrorForbidden(
                        "no permission to access the requested parent directory",
                    )
                })?;

            if Self::dir_by_name_exists_in(&dir.name, new_parent_oid).await? {
                return Err(actix_web::error::ErrorForbidden(
                    "A directory with that name already exists in the destination",
                ));
            }

            // give dir the new parent id
            DirectoryDAO::get_collection()
                .await
                .update_one(
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
            DirectoryDAO::add_child_by_oid(new_parent_oid, id, dir.user_id).await?;

            // remove child id from old parent
            DirectoryDAO::remove_child_by_oid(parent_id, id, dir.user_id).await?;

            let _ = SyncStateDAO::insert(&mut SyncState::new(
                SyncStateType::Directory,
                SyncStateAction::Move,
                id,
                dir.parent_id,
                dir.user_id,
            ))
            .await?;

            dir.parent_id = Some(new_parent_oid);
            return Ok(());
        }
        Err(actix_web::error::ErrorInternalServerError(
            "no permission or directory does not exist",
        ))
    }
}
