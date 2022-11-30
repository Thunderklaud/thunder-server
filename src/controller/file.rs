use std::io::Write;
use std::str::FromStr;

use actix_jwt_authc::Authenticated;
use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, web};
use futures_util::TryStreamExt;
use mongodb::bson::{DateTime, Uuid};
use mongodb::bson::oid::ObjectId;
use serde::Deserialize;

use crate::{Claims, Directory};
use crate::jwt_utils::extract_user_oid;
use crate::model::virtfile::VirtualFile;
use crate::storage::storage_provider::StorageProvider;

#[derive(Deserialize)]
pub struct MultiUploadQueryParams {
    directory: String,
}

pub async fn multi_upload(
    _authenticated: Authenticated<Claims>,
    request: HttpRequest,
    query_params: web::Query<MultiUploadQueryParams>,
    mut payload: Multipart,
) -> actix_web::Result<HttpResponse> {
    let connection = request.connection_info().clone();
    let _host = connection.peer_addr().unwrap_or("unknown host");

    if let Ok(parent_id) = ObjectId::from_str(query_params.directory.as_str()) {
        let dir = Directory::get_by_oid(parent_id, extract_user_oid(&_authenticated)).await?;
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
                        dir.update().await.map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
                    }
                    _ => {}
                }
            }

            return Ok(HttpResponse::Ok().finish());
        }

        return Err(actix_web::error::ErrorBadRequest(
            "Directory not found",
        ));
    }

    return Err(actix_web::error::ErrorBadRequest(
        "Query field directory is not parseable",
    ));
}
