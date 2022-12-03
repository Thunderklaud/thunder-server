use std::borrow::Borrow;
use std::str::FromStr;

use actix_jwt_authc::Authenticated;
use actix_web::{web::Json, HttpResponse};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::Serialize;
use tracing::{event, Level};

use crate::controller::utils::extract_object_id;
use crate::database::daos::dao::DAO;
use crate::database::daos::directory_dao::DirectoryDAO;
use crate::database::entities::directory::{
    DirFile, Directory, DirectoryGet, DirectoryPatch, DirectoryPost, MinimalDirectoryObject,
};
use crate::jwt_utils::extract_user_oid;
use crate::Claims;

#[derive(Serialize)]
struct DirectoryGetResponse {
    dirs: Vec<MinimalDirectoryObject>,
    files: Vec<DirFile>,
}

pub async fn create(
    _authenticated: Authenticated<Claims>,
    dir_post_data: Json<DirectoryPost>,
) -> actix_web::Result<HttpResponse> {
    let user_id = extract_user_oid(&_authenticated);
    let parent_id = extract_object_id(
        dir_post_data.parent_id.as_ref(),
        _authenticated.claims.thunder_root_dir_id,
    )?;

    DirectoryDAO::has_user_permission(parent_id, user_id).await?;

    if dir_post_data.name.is_empty() {
        return Err(actix_web::error::ErrorBadRequest(
            "Directory name cannot be empty",
        ));
    }

    let mut dir = Directory {
        id: None,
        user_id,
        parent_id: Some(parent_id),
        name: dir_post_data.name.to_owned().to_string(),
        creation_date: DateTime::now(),
        child_ids: vec![],
        files: vec![],
    };

    let dir_detail = DirectoryDAO::insert(&mut dir)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(HttpResponse::Ok().json(dir_detail))
}

pub async fn update(
    _authenticated: Authenticated<Claims>,
    dir_post_data: Json<DirectoryPatch>,
) -> actix_web::Result<HttpResponse> {
    let dir =
        DirectoryDAO::get_with_user(dir_post_data.id, extract_user_oid(&_authenticated)).await?;

    let mut dir = dir.ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("Directory could not be found")
    })?;

    if let Some(parent_id) = &dir_post_data.parent_id {
        // move dir if parent_id changes
        event!(
            Level::TRACE,
            "move dir if parent_id changes, parent_id: '{}'",
            parent_id
        );

        DirectoryDAO::move_to(
            &mut dir,
            extract_object_id(Some(parent_id), _authenticated.claims.thunder_root_dir_id)?,
            _authenticated.borrow(),
        )
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    }

    if let Some(name) = &dir_post_data.name {
        if name.is_empty() {
            return Err(actix_web::error::ErrorBadRequest(
                "Directory name cannot be empty",
            ));
        }

        dir.name = name.to_string();

        let update_result = DirectoryDAO::update(&mut dir).await?;
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
    }

    Ok(HttpResponse::Ok().finish())
}

pub async fn get(
    _authenticated: Authenticated<Claims>,
    dir_get_data: Json<DirectoryGet>,
) -> actix_web::Result<HttpResponse> {
    let id = match &dir_get_data.id {
        Some(id) if !id.is_empty() => {
            ObjectId::from_str(id).map_err(|e| actix_web::error::ErrorBadRequest(e))?
        }
        _ => _authenticated.claims.thunder_root_dir_id,
    };

    let dir = DirectoryDAO::get_with_user(id, extract_user_oid(&_authenticated)).await?;
    match dir {
        Some(dir) => Ok(HttpResponse::Ok().json(DirectoryGetResponse {
            dirs: DirectoryDAO::get_all_with_parent_id(dir.id).await?,
            files: dir.get_files().await,
        })),
        _ => Err(actix_web::error::ErrorInternalServerError(
            "Could not get requested directory",
        )),
    }
}
