use std::borrow::Borrow;
use std::str::FromStr;

use crate::archive::ArchiveMethod;
use actix_jwt_authc::Authenticated;
use actix_web::{web, web::Json, HttpResponse};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use tracing::{event, Level};

use crate::controller::utils::{extract_object_id, extract_object_id_or_die};
use crate::database::daos::dao::DAO;
use crate::database::daos::directory_dao::DirectoryDAO;
use crate::database::entities::directory::{
    Directory, DirectoryDelete, DirectoryGet, DirectoryGetResponse, DirectoryPatch, DirectoryPost,
    GetDirectoryArchiveQueryParams,
};
use crate::jwt_utils::extract_user_oid;
use crate::storage::storage_provider::StorageProvider;
use crate::Claims;

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
    };

    let dir_detail = DirectoryDAO::insert(&mut dir).await?;
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

        let new_parent_oid =
            extract_object_id(Some(parent_id), _authenticated.claims.thunder_root_dir_id)?;

        // do not move, if a rename after moving is requested, but not possible
        if let Some(name) = &dir_post_data.name {
            if DirectoryDAO::dir_by_name_exists_in(name, new_parent_oid).await? {
                return Err(actix_web::error::ErrorForbidden(
                    "A directory with that name already exists in the destination",
                ));
            }
        }

        DirectoryDAO::move_to(&mut dir, new_parent_oid, _authenticated.borrow()).await?;
    }

    if let Some(name) = &dir_post_data.name {
        if !dir.name.eq(name) {
            if name.is_empty() {
                return Err(actix_web::error::ErrorBadRequest(
                    "Directory name cannot be empty",
                ));
            }

            DirectoryDAO::rename(&mut dir, name).await?;
        }
    }

    Ok(HttpResponse::Ok().finish())
}

pub async fn delete(
    _authenticated: Authenticated<Claims>,
    dir_delete_data: web::Query<DirectoryDelete>,
) -> actix_web::Result<HttpResponse> {
    let dir = DirectoryDAO::get_with_user(
        extract_object_id_or_die(Some(&dir_delete_data.id))?,
        extract_user_oid(&_authenticated),
    )
    .await?;

    let dir = dir.ok_or_else(|| {
        actix_web::error::ErrorInternalServerError("Directory could not be found")
    })?;

    DirectoryDAO::delete(&dir).await?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn get(
    _authenticated: Authenticated<Claims>,
    dir_get_data: web::Query<DirectoryGet>,
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
            dirs: DirectoryDAO::get_all_with_parent_id_for_response(dir.id).await?,
            files: dir.get_files().await,
        })),
        _ => Err(actix_web::error::ErrorInternalServerError(
            "Could not get requested directory",
        )),
    }
}

pub async fn get_directory_archive_stream(
    _authenticated: Authenticated<Claims>,
    query_params: web::Query<GetDirectoryArchiveQueryParams>,
) -> actix_web::Result<HttpResponse> {
    let id = match &query_params.id {
        Some(id) if !id.is_empty() => {
            ObjectId::from_str(id).map_err(|e| actix_web::error::ErrorBadRequest(e))?
        }
        _ => _authenticated.claims.thunder_root_dir_id,
    };

    let dir = DirectoryDAO::get_with_user(id, extract_user_oid(&_authenticated)).await?;
    match dir {
        Some(mut dir) => {
            let mut archive_method = ArchiveMethod::Tar;
            if let Some(archive) = &query_params.archive {
                match archive.as_str() {
                    "zip" => archive_method = ArchiveMethod::Zip,
                    "tar.gz" => archive_method = ArchiveMethod::TarGz,
                    _ => archive_method = ArchiveMethod::Tar,
                }
            }

            let file_name = format!(
                "{}.{}",
                match (&dir.name).as_str() {
                    "/" => {
                        dir.name = "root".to_string();
                        "root"
                    }
                    _ => &dir.name,
                },
                archive_method.extension()
            );
            let rx = StorageProvider::get_compressed_directory_stream(&dir, archive_method).await?;

            return Ok(HttpResponse::Ok()
                .content_type(archive_method.content_type())
                .append_header(archive_method.content_encoding())
                .append_header(("Content-Transfer-Encoding", "binary"))
                .append_header((
                    "Content-Disposition",
                    format!("attachment; filename={:?}", file_name),
                ))
                .body(actix_web::body::BodyStream::new(rx)));
        }
        _ => Err(actix_web::error::ErrorInternalServerError(
            "Could not get requested directory",
        )),
    }
}
