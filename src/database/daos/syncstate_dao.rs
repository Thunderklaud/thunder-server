use crate::database::daos::dao::DAO;
use crate::database::entities::syncstate::{SyncState, SyncStateAction, SyncStateType};
use crate::database::entities::user::User;
use crate::jwt_utils::extract_user_oid;
use crate::Claims;
use actix_jwt_authc::Authenticated;
use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};
use std::borrow::Borrow;
use tracing::{event, Level};

pub struct SyncStateDAO {}

#[async_trait]
impl DAO<SyncState, ObjectId> for SyncStateDAO {
    async fn get(oid: ObjectId) -> actix_web::Result<Option<SyncState>> {
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

    async fn get_with_user(
        _: ObjectId,
        _user_id: ObjectId,
    ) -> actix_web::Result<Option<SyncState>> {
        unimplemented!()
    }

    async fn insert(state: &mut SyncState) -> actix_web::Result<ObjectId> {
        let insert_result = Self::get_collection()
            .await
            .insert_one(state.borrow(), None)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        state.id = insert_result.inserted_id.as_object_id();
        if let Some(id) = state.id {
            return Ok(id);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "syncstate insert failed converting inserted_id to ObjectId",
        ))
    }

    async fn update(_: &SyncState) -> actix_web::Result<u64> {
        unimplemented!()
    }

    async fn delete(_: &SyncState) -> actix_web::Result<u64> {
        todo!()
    }
}

// custom methods
impl SyncStateDAO {}
