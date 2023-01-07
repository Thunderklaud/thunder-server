use std::borrow::Borrow;

use async_trait::async_trait;
use futures_util::StreamExt;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;

use crate::database::daos::dao::DAO;
use crate::database::entities::share::Share;

pub struct ShareDAO {}

#[async_trait]
impl DAO<Share, ObjectId> for ShareDAO {
    async fn get(oid: ObjectId) -> actix_web::Result<Option<Share>> {
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

    async fn get_with_user(oid: ObjectId, user_id: ObjectId) -> actix_web::Result<Option<Share>> {
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

    async fn insert(share: &mut Share) -> actix_web::Result<ObjectId> {
        let insert_result = Self::get_collection()
            .await
            .insert_one(share.borrow(), None)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        share.id = insert_result.inserted_id.as_object_id();
        if let Some(id) = share.id {
            return Ok(id);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "share insert failed converting inserted_id to ObjectId",
        ))
    }

    async fn update(_: &Share) -> actix_web::Result<u64> {
        unimplemented!()
    }

    async fn delete(share: &Share) -> actix_web::Result<u64> {
        if let Some(id) = share.id {
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

            return Ok(delete_result.deleted_count);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "share id not found",
        ))
    }
}

// custom methods
impl ShareDAO {
    pub async fn delete_for_corresponding_id(
        corresponding_id: ObjectId,
    ) -> actix_web::error::Result<()> {
        Self::get_collection()
            .await
            .delete_many(
                doc! {
                    "corresponding_id": corresponding_id
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        Ok(())
    }

    pub async fn delete_for_corresponding_parent_id(
        corresponding_parent_id: ObjectId,
    ) -> actix_web::error::Result<()> {
        Self::get_collection()
            .await
            .delete_many(
                doc! {
                    "corresponding_parent_id": corresponding_parent_id
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        Ok(())
    }

    pub async fn get_all_for_user(user_id: ObjectId) -> actix_web::Result<Vec<Share>> {
        let mut shares: Vec<Share> = Vec::new();

        let mut cursor = Self::get_collection()
            .await
            .find(
                doc! {
                    "user_id": user_id,
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        while let Some(share) = cursor.next().await {
            if let Ok(share) = share {
                shares.push(share);
            }
        }

        Ok(shares)
    }

    pub async fn get_all_for_corresponding_id(
        corresponding_id: ObjectId,
    ) -> actix_web::Result<Vec<Share>> {
        let mut shares: Vec<Share> = Vec::new();

        let mut cursor = Self::get_collection()
            .await
            .find(
                doc! {
                    "corresponding_id": corresponding_id,
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        while let Some(share) = cursor.next().await {
            if let Ok(share) = share {
                shares.push(share);
            }
        }

        Ok(shares)
    }
}
