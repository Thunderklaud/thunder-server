use futures::StreamExt;
use std::borrow::Borrow;

use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};

use crate::database::daos::dao::DAO;
use crate::database::daos::syncstate_dao::SyncStateDAO;
use crate::database::entities::file::File;
use crate::database::entities::syncstate::{SyncState, SyncStateAction, SyncStateType};

pub struct FileDAO {}

#[async_trait]
impl DAO<File, ObjectId> for FileDAO {
    async fn get(oid: ObjectId) -> actix_web::Result<Option<File>> {
        Self::get_collection()
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

    async fn get_with_user(oid: ObjectId, user_id: ObjectId) -> actix_web::Result<Option<File>> {
        Self::get_collection()
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

    async fn insert(file: &mut File) -> actix_web::Result<ObjectId> {
        let insert_result = Self::get_collection()
            .await
            .insert_one(file.borrow(), None)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        file.id = insert_result.inserted_id.as_object_id();

        if let Some(id) = file.id {
            let _ = SyncStateDAO::insert(&mut SyncState::add(
                SyncStateType::File,
                SyncStateAction::Create,
                id,
                file.user_id,
            ));

            return Ok(id);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "failed converting inserted_id to ObjectId",
        ))
    }

    async fn update(file: &File) -> actix_web::Result<u64> {
        if let Some(id) = file.id {
            let update_result = Self::get_collection()
                .await
                .update_one(
                    doc! {
                        "_id": id
                    },
                    doc! {
                        "$set": {
                            "parent_id": file.parent_id.to_owned(),
                            "user_id": file.user_id.to_owned(),
                            "uuid": file.uuid.to_owned(),
                            "hash": file.hash.to_owned(),
                            "mime": file.mime.to_owned(),
                            "name": file.name.to_owned(),
                            "finished": file.finished.to_owned(),
                            "creation_date": DateTime::now(),
                        }
                    },
                    None,
                )
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            return Ok(update_result.modified_count);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "file id not found",
        ))
    }

    async fn delete(file: &File) -> actix_web::Result<u64> {
        if let Some(id) = file.id {
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

            let _ = SyncStateDAO::insert(&mut SyncState::add(
                SyncStateType::File,
                SyncStateAction::Delete,
                id,
                file.user_id,
            ));

            return Ok(delete_result.deleted_count);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "file id not found",
        ))
    }
}

impl FileDAO {
    /*pub async fn get_file_by_uuid(uuid: &String) -> actix_web::Result<Option<File>> {
        //todo: add uuid -> oid cache map to use DAO get function
        Self::get_collection()
            .await
            .find_one(
                doc! {
                    "uuid": uuid
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }*/
    pub async fn get_file_by_uuid_for_user(
        uuid: &String,
        user_id: ObjectId,
    ) -> actix_web::Result<Option<File>> {
        //todo: add uuid -> oid cache map to use DAO get_with_user function
        Self::get_collection()
            .await
            .find_one(
                doc! {
                    "uuid": uuid,
                    "user_id": user_id
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }
    pub async fn get_files_by_parent_id(parent_id: ObjectId) -> actix_web::Result<Vec<File>> {
        let mut files: Vec<File> = Vec::new();

        let mut cursor = Self::get_collection()
            .await
            .find(
                doc! {
                    "parent_id": parent_id
                },
                None,
            )
            .await
            .map_err(|_| actix_web::error::ErrorNotFound("getting files by parent_id failed"))?;

        while let Some(file) = cursor.next().await {
            if let Ok(file) = file {
                files.push(file);
            }
        }

        Ok(files)
    }
}
