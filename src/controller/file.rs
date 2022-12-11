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

use crate::database::daos::dao::DAO;
use crate::database::daos::directory_dao::DirectoryDAO;
use crate::database::daos::file_dao::FileDAO;
use crate::database::daos::syncstate_dao::SyncStateDAO;
use crate::database::entities::file::{
    File, FilePatch, GetSingleQueryParams, MultiUploadQueryParams,
};
use crate::database::entities::syncstate::{SyncState, SyncStateAction, SyncStateType};
use crate::jwt_utils::extract_user_oid;
use crate::storage::storage_provider::StorageProvider;
use crate::Claims;

pub async fn get_single(
    _authenticated: Authenticated<Claims>,
    query_params: web::Query<GetSingleQueryParams>,
) -> actix_web::Result<NamedFile> {
    if let Some(file) =
        FileDAO::get_file_by_uuid_for_user(&query_params.uuid, extract_user_oid(&_authenticated))
            .await?
    {
        let mut named_file = NamedFile::open(StorageProvider::get_direct_file_path(file.uuid))?;

        if let Ok(mime) = Mime::from_str(file.mime.as_str()) {
            named_file = named_file.set_content_type(mime);
        }

        return Ok(named_file);
    }

    return Err(actix_web::error::ErrorBadRequest("File not found"));
}

pub async fn multi_upload(
    _authenticated: Authenticated<Claims>,
    request: HttpRequest,
    query_params: web::Query<MultiUploadQueryParams>,
    mut payload: Multipart,
) -> actix_web::Result<HttpResponse> {
    let connection = request.connection_info().clone();
    let _host = connection.peer_addr().unwrap_or("unknown host");
    let mut uploaded_files: Vec<File> = Vec::new();
    let user_id = extract_user_oid(&_authenticated);

    if let Ok(parent_id) = ObjectId::from_str(query_params.directory.as_str()) {
        let dir = DirectoryDAO::get_with_user(parent_id, user_id).await?;
        if let Some(dir) = dir {
            while let Some(mut field) = payload.try_next().await? {
                match field.name() {
                    "file" => {
                        // A multipart/form-data stream has to contain `content_disposition`
                        let content_disposition = field.content_disposition();

                        let filename = content_disposition
                            .get_filename()
                            .map_or_else(|| Uuid::new().to_string(), sanitize_filename::sanitize);

                        if dir.has_file_with_name(&filename).await {
                            continue;
                        }

                        let mut file = File {
                            id: None,
                            parent_id: dir.id.unwrap(),
                            user_id,
                            uuid: Uuid::new().to_string(),
                            hash: "".to_string(),
                            mime: field.content_type().to_string(),
                            name: filename,
                            finished: true,
                            creation_date: DateTime::now(),
                        };

                        // File::create is a blocking operation
                        let mut storage_file =
                            StorageProvider::create_file_handle(file.uuid.clone()).await?;

                        // Field in turn is stream of *Bytes* object
                        while let Some(chunk) = field.try_next().await? {
                            // filesystem operations are blocking, may we have to use threadpool
                            storage_file = web::block(move || {
                                storage_file.write_all(&chunk).map(|_| storage_file)
                            })
                            .await??;
                        }

                        // Save VirtualFile as DirFile to db
                        FileDAO::insert(&mut file).await?;
                        uploaded_files.push(file);
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
    let user_id = extract_user_oid(&_authenticated);
    if let Some(mut file) =
        FileDAO::get_file_by_uuid_for_user(&file_patch_data.uuid, user_id).await?
    {
        let dir = DirectoryDAO::get_with_user(file.parent_id, user_id).await?;
        if let Some(dir) = dir {
            let mut changed = false;

            // check if file can be renamed
            if let Some(new_name) = &file_patch_data.new_name {
                // do not rename if new name = current name
                if new_name.ne(&file.name) {
                    // check if there is already a file with the given name in the current directory
                    if !dir.has_file_with_name(&new_name).await {
                        file.name = (*new_name).clone();
                        changed = true;

                        let _ = SyncStateDAO::insert(&mut SyncState::new(
                            SyncStateType::File,
                            SyncStateAction::Create,
                            file.id.unwrap(),
                            Some(file.parent_id),
                            file.user_id,
                        ))
                        .await?;
                    } else {
                        return Err(actix_web::error::ErrorBadRequest(
                            "There is already a file with the given name in the current directory",
                        ));
                    }
                }
            }

            // check if file can be moved
            if let Some(new_directory_id) = &file_patch_data.new_directory {
                // check if the new directory is possible
                if let Ok(new_directory_oid) = ObjectId::from_str(&new_directory_id.as_str()) {
                    let new_directory =
                        DirectoryDAO::get_with_user(new_directory_oid, user_id).await?;

                    if let Some(new_directory) = new_directory {
                        // check if the new directory already contains a file with the same name
                        if !new_directory.has_file_with_name(&file.name).await {
                            file.parent_id = new_directory_oid;
                            changed = true;

                            let _ = SyncStateDAO::insert(&mut SyncState::new(
                                SyncStateType::File,
                                SyncStateAction::Move,
                                file.id.unwrap(),
                                Some(file.parent_id),
                                file.user_id,
                            ))
                            .await?;
                        } else {
                            return Err(actix_web::error::ErrorBadRequest(
                                "There is already a file with the given name in the new directory",
                            ));
                        }
                    } else {
                        return Err(actix_web::error::ErrorBadRequest("New directory not found"));
                    }
                } else {
                    return Err(actix_web::error::ErrorBadRequest(
                        "New directory id not parseable",
                    ));
                }
            }

            if changed {
                FileDAO::update(&file).await?;
            }
            return Ok(HttpResponse::Ok().finish());
        }

        return Err(actix_web::error::ErrorBadRequest("Directory not found"));
    }

    return Err(actix_web::error::ErrorBadRequest("File not found"));
}

pub async fn delete(
    _authenticated: Authenticated<Claims>,
    query_params: web::Query<GetSingleQueryParams>,
) -> actix_web::Result<HttpResponse> {
    if let Some(file) =
        FileDAO::get_file_by_uuid_for_user(&query_params.uuid, extract_user_oid(&_authenticated))
            .await?
    {
        StorageProvider::delete_file(file.uuid.clone())?;
        FileDAO::delete(&file).await?;

        return Ok(HttpResponse::Ok().finish());
    }

    return Err(actix_web::error::ErrorBadRequest("File not found"));
}
