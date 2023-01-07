use actix_jwt_authc::Authenticated;
use actix_web::web::Json;
use actix_web::{web, HttpResponse};

use crate::database::daos::dao::DAO;
use crate::database::daos::file_dao::FileDAO;
use crate::database::daos::share_dao::ShareDAO;
use crate::database::entities::share::{FileShareCreate, Share, ShareGet, ShareType};
use crate::jwt_utils::extract_user_oid;
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
            file.parent_id,
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
