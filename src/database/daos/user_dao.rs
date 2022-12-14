use crate::database::daos::dao::DAO;
use crate::database::daos::syncstate_dao::SyncStateDAO;
use crate::database::entities::syncstate::{SyncState, SyncStateAction, SyncStateType};
use crate::database::entities::user::User;
use crate::jwt_utils::extract_user_oid;
use crate::Claims;
use actix_jwt_authc::Authenticated;
use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use std::borrow::Borrow;
use tracing::{event, Level};

pub struct UserDAO {}

#[async_trait]
impl DAO<User, ObjectId> for UserDAO {
    async fn get(oid: ObjectId) -> actix_web::Result<Option<User>> {
        UserDAO::get_collection()
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

    async fn get_with_user(_: ObjectId, _user_id: ObjectId) -> actix_web::Result<Option<User>> {
        unimplemented!()
    }

    async fn insert(user: &mut User) -> actix_web::Result<ObjectId> {
        let insert_result = UserDAO::get_collection()
            .await
            .insert_one(user.borrow(), None)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        user.id = insert_result.inserted_id.as_object_id();
        if let Some(id) = user.id {
            let _ = SyncStateDAO::insert(&mut SyncState::new(
                SyncStateType::User,
                SyncStateAction::Create,
                id,
                None,
                id,
            ))
            .await?;

            return Ok(id);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "failed converting inserted_id to ObjectId",
        ))
    }

    async fn update(user: &User) -> actix_web::Result<u64> {
        if let Some(id) = user.id {
            let update_result = UserDAO::get_collection()
                .await
                .update_one(
                    doc! {
                        "_id": id
                    },
                    doc! {
                        "$set": {
                            "firstname": user.firstname.to_owned(),
                            "lastname": user.lastname.to_owned(),
                            "email": user.email.to_owned(),
                            "pw_hash": user.pw_hash.to_owned(),
                            "role": user.role.as_ref(),
                            "root_dir_id": user.root_dir_id.to_owned(),
                        }
                    },
                    None,
                )
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            return Ok(update_result.modified_count);
        }

        Err(actix_web::error::ErrorInternalServerError(
            "user id not found",
        ))
    }

    async fn delete(_: &User) -> actix_web::Result<u64> {
        todo!()
    }
}

// custom methods
impl UserDAO {
    pub async fn get_authenticated(
        authenticated: &Authenticated<Claims>,
    ) -> actix_web::Result<Option<User>> {
        event!(
            Level::INFO,
            "get_authenticated: {}",
            authenticated.claims.sub.as_str()
        );
        UserDAO::get(extract_user_oid(&authenticated)).await
    }

    pub async fn get_by_email(email: &str) -> actix_web::Result<Option<User>> {
        UserDAO::get_collection()
            .await
            .find_one(
                doc! {
                    "email": email
                },
                None,
            )
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }

    pub async fn exists(email: &String) -> actix_web::Result<bool> {
        Ok(UserDAO::get_by_email(email.to_owned().as_str())
            .await?
            .is_some())
    }
}
