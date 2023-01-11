use crate::archive::ArchiveMethod;
use actix_jwt_authc::Authenticated;
use actix_web::web::Json;
use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::bson::DateTime;

use crate::database::daos::dao::DAO;
use crate::database::daos::file_dao::FileDAO;
use crate::database::daos::share_dao::ShareDAO;
use crate::database::entities::file::GetSingleQueryParams;
use crate::database::entities::share::{FileShareCreate, Share, ShareDelete, ShareGet, ShareType};
use crate::jwt_utils::extract_user_oid;
use crate::storage::storage_provider::StorageProvider;
use crate::Claims;

pub async fn get_share_info(
    share_get_data: web::Query<ShareGet>,
) -> actix_web::Result<HttpResponse> {
    if let Some(share) = ShareDAO::get(share_get_data.id).await? {
        return Ok(HttpResponse::Ok().json(share));
    }

    Err(actix_web::error::ErrorBadRequest(
        "Requested share could not be found",
    ))
}

pub async fn download(
    share_get_data: web::Query<ShareGet>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    if let Some(mut share) = ShareDAO::get(share_get_data.id).await? {
        if let Some(valid_until) = share.valid_until {
            if valid_until < DateTime::now() {
                return Err(actix_web::error::ErrorForbidden("Share expired"));
            }
        }

        if let Some(max_dl_count) = share.max_dl_count {
            if share.current_dl_count >= max_dl_count {
                return Err(actix_web::error::ErrorForbidden("Max shares reached"));
            }
        }

        return match share.get_type() {
            ShareType::File => {
                if let Some(file) = FileDAO::get(share.corresponding_id).await? {
                    let mut archive_method: Option<ArchiveMethod> = None;
                    if let Some(archive) = &share_get_data.archive {
                        archive_method = Some(match archive.as_str() {
                            "zip" => ArchiveMethod::Zip,
                            "tar.gz" => ArchiveMethod::TarGz,
                            _ => ArchiveMethod::Tar,
                        })
                    }

                    ShareDAO::register_share_download(&mut share).await?;

                    if let Some(archive_method) = archive_method {
                        let rx =
                            StorageProvider::get_compressed_file_stream(&file, archive_method)?;
                        let file_name = format!("{}.{}", &file.name, archive_method.extension());

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

                    return Ok(StorageProvider::get_named_file(&file)?.into_response(&req));
                }
                Err(actix_web::error::ErrorInternalServerError(
                    "Requested file could not be found",
                ))
            }
            _ => Err(actix_web::error::ErrorInternalServerError(
                "Share type download not supported yet",
            )),
        };
    }

    Err(actix_web::error::ErrorBadRequest(
        "Requested share could not be found",
    ))
}

pub async fn get_share_infos_for_file(
    _authenticated: Authenticated<Claims>,
    share_get_data: web::Query<GetSingleQueryParams>,
) -> actix_web::Result<HttpResponse> {
    if let Some(file) =
        FileDAO::get_file_by_uuid_for_user(&share_get_data.uuid, extract_user_oid(&_authenticated))
            .await?
    {
        if let Ok(shares) = ShareDAO::get_all_for_corresponding_id(file.id.unwrap()).await {
            return Ok(HttpResponse::Ok().json(shares));
        }
    }

    Err(actix_web::error::ErrorBadRequest(
        "Requested file or shares could not be found",
    ))
}

pub async fn get_share_infos_for_user(
    _authenticated: Authenticated<Claims>,
) -> actix_web::Result<HttpResponse> {
    if let Ok(shares) = ShareDAO::get_all_for_user(extract_user_oid(&_authenticated)).await {
        return Ok(HttpResponse::Ok().json(shares));
    }

    Err(actix_web::error::ErrorBadRequest(
        "Requested shares could not be found",
    ))
}

pub async fn create_file_share(
    _authenticated: Authenticated<Claims>,
    create_share_data: Json<FileShareCreate>,
) -> actix_web::Result<HttpResponse> {
    let user_id = extract_user_oid(&_authenticated);

    if let Some(file) = FileDAO::get_file_by_uuid_for_user(&create_share_data.uuid, user_id).await?
    {
        let mut share = Share::new(
            ShareType::File,
            file.id.unwrap(),
            user_id,
            create_share_data.label.to_string(),
            create_share_data.max_dl_count,
            create_share_data.valid_until,
        );
        let share_id = ShareDAO::insert(&mut share).await?;

        return Ok(HttpResponse::Ok().json(share_id));
    }

    Err(actix_web::error::ErrorBadRequest(
        "Requested file could not be found",
    ))
}

pub async fn delete_share(
    _authenticated: Authenticated<Claims>,
    delete_share_data: web::Query<ShareDelete>,
) -> actix_web::Result<HttpResponse> {
    if let Some(share) =
        ShareDAO::get_with_user(delete_share_data.id, extract_user_oid(&_authenticated)).await?
    {
        ShareDAO::delete(&share).await?;
        return Ok(HttpResponse::Ok().finish());
    }

    Err(actix_web::error::ErrorBadRequest(
        "Requested share could not be found",
    ))
}
