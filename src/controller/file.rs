use std::io::Write;
use std::str::FromStr;

use actix_files::NamedFile;
use actix_jwt_authc::Authenticated;
use actix_multipart::Multipart;
use actix_web::web::Json;
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::TryStreamExt;
use mime::Mime;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{DateTime, Uuid};
use serde::Deserialize;

use crate::database::daos::dao::DAO;
use crate::database::daos::directory_dao::DirectoryDAO;
use crate::jwt_utils::extract_user_oid;
use crate::model::virtfile::VirtualFile;
use crate::storage::storage_provider::StorageProvider;
use crate::Claims;

#[derive(Deserialize)]
pub struct GetSingleQueryParams {
    uuid: String,
    directory: String,
}

#[derive(Deserialize)]
pub struct MultiUploadQueryParams {
    directory: String,
}

#[derive(Deserialize)]
pub struct FilePatch {
    uuid: String,
    directory: String,
    name: Option<String>,
    new_directory: Option<String>,
}

pub async fn get_single(
    _authenticated: Authenticated<Claims>,
    query_params: web::Query<GetSingleQueryParams>,
) -> actix_web::Result<NamedFile> {
    if let Ok(parent_id) = ObjectId::from_str(query_params.directory.as_str()) {
        let dir = DirectoryDAO::get_with_user(parent_id, extract_user_oid(&_authenticated)).await?;
        if let Some(dir) = dir {
            for file in dir.get_files().await {
                if file.uuid.eq(&query_params.uuid) {
                    let mut named_file =
                        NamedFile::open(StorageProvider::get_direct_file_path(file.uuid))?;

                    if let Ok(mime) = Mime::from_str(file.mime.as_str()) {
                        named_file = named_file.set_content_type(mime);
                    }

                    return Ok(named_file);
                }
            }

            return Err(actix_web::error::ErrorBadRequest("File not found"));
        }

        return Err(actix_web::error::ErrorBadRequest("Directory not found"));
    }

    return Err(actix_web::error::ErrorBadRequest(
        "Query field directory is not parseable",
    ));
}

pub async fn multi_upload(
    _authenticated: Authenticated<Claims>,
    request: HttpRequest,
    query_params: web::Query<MultiUploadQueryParams>,
    mut payload: Multipart,
) -> actix_web::Result<HttpResponse> {
    let connection = request.connection_info().clone();
    let _host = connection.peer_addr().unwrap_or("unknown host");
    let mut uploaded_files: Vec<VirtualFile> = Vec::new();

    if let Ok(parent_id) = ObjectId::from_str(query_params.directory.as_str()) {
        let dir = DirectoryDAO::get_with_user(parent_id, extract_user_oid(&_authenticated)).await?;
        if let Some(mut dir) = dir {
            while let Some(mut field) = payload.try_next().await? {
                match field.name() {
                    "file" => {
                        // A multipart/form-data stream has to contain `content_disposition`
                        let content_disposition = field.content_disposition();

                        let filename = content_disposition
                            .get_filename()
                            .map_or_else(|| Uuid::new().to_string(), sanitize_filename::sanitize);

                        let vfile = VirtualFile {
                            parent_id: _authenticated.claims.thunder_root_dir_id,
                            user_id: extract_user_oid(&_authenticated),
                            uuid: Uuid::new().to_string(),
                            hash: "".to_string(),
                            mime: field.content_type().to_string(),
                            name: filename,
                            finished: true,
                            creation_date: DateTime::now(),
                        };

                        // File::create is a blocking operation
                        let mut f = StorageProvider::create_file_handle(vfile.uuid.clone()).await?;

                        // Field in turn is stream of *Bytes* object
                        while let Some(chunk) = field.try_next().await? {
                            // filesystem operations are blocking, may we have to use threadpool
                            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
                        }

                        // Save VirtualFile as DirFile to db
                        dir.files.push(vfile.as_serialized_dir_file());
                        DirectoryDAO::update(&dir)
                            .await
                            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
                        uploaded_files.push(vfile);
                    }
                    _ => {}
                }
            }

            return Ok(HttpResponse::Ok().json(uploaded_files));
        }

        return Err(actix_web::error::ErrorBadRequest("Directory not found"));
    }

    return Err(actix_web::error::ErrorBadRequest(
        "Query field directory is not parseable",
    ));
}

pub async fn update(
    _authenticated: Authenticated<Claims>,
    file_patch_data: Json<FilePatch>,
) -> actix_web::Result<HttpResponse> {
    if let Ok(parent_id) = ObjectId::from_str(&file_patch_data.directory.as_str()) {
        let dir = DirectoryDAO::get_with_user(parent_id, extract_user_oid(&_authenticated)).await?;
        if let Some(mut dir) = dir {
            let dir_file = dir
                .get_dirfile_by_uuid((&file_patch_data.uuid).clone())
                .await;
            let index_in_dir_files = dir
                .get_files_index_by_file_uuid(&file_patch_data.uuid)
                .await;

            if let (Some(mut dir_file), Some(index_in_dir_files)) = (dir_file, index_in_dir_files) {
                // check if file can be renamed
                if let Some(new_name) = &file_patch_data.name {
                    // do not rename if new name = current name
                    if new_name.ne(&dir_file.name) {
                        // check if there is already a file with the given name in the current directory
                        let index_of_same_name_file =
                            dir.get_first_file_index_by_file_name(&new_name).await;

                        if let None = index_of_same_name_file {
                            dir_file.name = (*new_name).clone();
                            dir.files.remove(index_in_dir_files);
                            dir.files.push(serde_json::to_string(&dir_file).unwrap());
                            DirectoryDAO::update(&mut dir).await?;
                        } else {
                            return Err(actix_web::error::ErrorBadRequest("There is already a file with the given name in the current directory"));
                        }
                    }
                }

                // check if file can be moved
                if let Some(new_directory_id) = &file_patch_data.new_directory {
                    // check if the new directory is possible
                    if let Ok(new_directory_oid) = ObjectId::from_str(&new_directory_id.as_str()) {
                        let new_directory = DirectoryDAO::get_with_user(
                            new_directory_oid,
                            extract_user_oid(&_authenticated),
                        )
                        .await?;
                        if let Some(mut new_directory) = new_directory {
                            // check if the new directory already contains a file with the same name
                            let index_of_same_name_file = new_directory
                                .get_first_file_index_by_file_name(&dir_file.name)
                                .await;
                            if let None = index_of_same_name_file {
                                dir.files.remove(index_in_dir_files);
                                new_directory
                                    .files
                                    .push(serde_json::to_string(&dir_file).unwrap());

                                DirectoryDAO::update(&mut dir).await?;
                                DirectoryDAO::update(&mut new_directory).await?;
                            } else {
                                return Err(actix_web::error::ErrorBadRequest("There is already a file with the given name in the new directory"));
                            }
                        } else {
                            return Err(actix_web::error::ErrorBadRequest(
                                "New directory not found",
                            ));
                        }
                    } else {
                        return Err(actix_web::error::ErrorBadRequest(
                            "New directory id not parseable",
                        ));
                    }
                }

                return Ok(HttpResponse::Ok().finish());
            }

            return Err(actix_web::error::ErrorBadRequest("File not found"));
        }

        return Err(actix_web::error::ErrorBadRequest("Directory not found"));
    }

    return Err(actix_web::error::ErrorBadRequest(
        "Query field directory is not parseable",
    ));
}

pub async fn delete(
    _authenticated: Authenticated<Claims>,
    query_params: web::Query<GetSingleQueryParams>,
) -> actix_web::Result<HttpResponse> {
    if let Ok(parent_id) = ObjectId::from_str(query_params.directory.as_str()) {
        let dir = DirectoryDAO::get_with_user(parent_id, extract_user_oid(&_authenticated)).await?;
        if let Some(mut dir) = dir {
            let index_in_dir_files = dir.get_files_index_by_file_uuid(&query_params.uuid).await;

            if let Some(index_in_dir_files) = index_in_dir_files {
                StorageProvider::delete_file(query_params.uuid.clone())?;
                dir.files.remove(index_in_dir_files);
                DirectoryDAO::update(&mut dir).await?;

                return Ok(HttpResponse::Ok().finish());
            }

            return Err(actix_web::error::ErrorBadRequest("File not found"));
        }

        return Err(actix_web::error::ErrorBadRequest("Directory not found"));
    }

    return Err(actix_web::error::ErrorBadRequest(
        "Query field directory is not parseable",
    ));
}
