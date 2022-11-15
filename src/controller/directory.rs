use std::borrow::Borrow;
use std::str::FromStr;

use actix_jwt_authc::Authenticated;
use actix_web::{web::Json, HttpResponse};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::Serialize;
use tracing::{event, Level};

use crate::controller::utils::{
    extract_object_id, get_empty_success_response, DefaultStringResponse,
};
use crate::jwt_utils::extract_user_oid;
use crate::model::directory::{
    Directory, DirectoryGet, DirectoryPatch, DirectoryPost, MinimalDirectoryObject,
};
use crate::Claims;

#[derive(Serialize)]
pub struct DefaultResponse {
    #[serde(flatten)]
    result: Option<ResultDataType>,
    status: bool,
    error: String,
}

#[derive(Serialize)]
enum ResultDataType {
    #[serde(rename(serialize = "result"))]
    DirectoryGetResponse(DirectoryGetResponse),
}

#[derive(Serialize)]
struct DirectoryGetResponse {
    dirs: Vec<MinimalDirectoryObject>,
}

pub async fn create(
    _authenticated: Authenticated<Claims>,
    dir_post_data: Json<DirectoryPost>,
) -> actix_web::Result<HttpResponse> {
    let user_id = extract_user_oid(&_authenticated);
    let parent_id = extract_object_id(
        &dir_post_data.parent_id,
        _authenticated.claims.thunder_root_dir_id,
    )?;

    Directory::has_user_permission(parent_id, user_id).await?;

    let mut dir = Directory {
        id: None,
        user_id,
        parent_id: Some(parent_id),
        name: dir_post_data.name.to_owned().to_string(),
        creation_date: DateTime::now(),
        child_ids: vec![],
        files: vec![],
    };

    let dir_detail = dir
        .create()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    Ok(HttpResponse::Ok().json(dir_detail))
}

pub async fn update(
    _authenticated: Authenticated<Claims>,
    dir_post_data: Json<DirectoryPatch>,
) -> HttpResponse {
    let dir = Directory::get_by_oid(dir_post_data.id, extract_user_oid(&_authenticated)).await;

    if dir.is_some() {
        let mut dir = dir.unwrap();
        if dir_post_data.parent_id.is_some() {
            // move dir if parent_id changes
            event!(
                Level::TRACE,
                "move dir if parent_id changes, parent_id: '{}'",
                dir_post_data.parent_id.to_owned().unwrap().to_string()
            );

            let move_result = if dir_post_data
                .parent_id
                .to_owned()
                .unwrap()
                .to_string()
                .eq("")
            {
                dir.move_to(
                    _authenticated.claims.thunder_root_dir_id,
                    _authenticated.borrow(),
                )
                .await
            } else {
                dir.move_to(
                    ObjectId::from_str(dir_post_data.parent_id.to_owned().unwrap().as_str())
                        .unwrap(),
                    _authenticated.borrow(),
                )
                .await
            };

            if move_result.is_err() {
                return HttpResponse::InternalServerError().json(DefaultStringResponse {
                    result: None,
                    status: false,
                    error: move_result.err().unwrap().to_string(),
                });
            }
        }
        if dir_post_data.name.is_some() {
            dir.name = dir_post_data.name.to_owned().unwrap();
            let update_result = dir.update().await;

            if update_result.modified_count <= 0 {
                event!(
                    Level::DEBUG,
                    "renaming directory failed {:?}",
                    update_result
                );
                return HttpResponse::InternalServerError().json(DefaultStringResponse {
                    result: None,
                    status: false,
                    error: "Renaming directory failed".parse().unwrap(),
                });
            }
        }
    } else {
        event!(Level::DEBUG, "Directory could not be found");
        return HttpResponse::InternalServerError().json(DefaultStringResponse {
            result: None,
            status: false,
            error: "Directory could not be found".parse().unwrap(),
        });
    }

    get_empty_success_response()
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

    let dir = Directory::get_by_oid(id, extract_user_oid(&_authenticated)).await;

    let dir = dir.ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("Could not get requested directory")
    })?;

    Ok(
        HttpResponse::Ok().json(ResultDataType::DirectoryGetResponse(DirectoryGetResponse {
            dirs: Directory::get_all_with_parent_id(dir.id).await,
        })),
    )
}
