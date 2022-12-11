use actix_jwt_authc::Authenticated;
use actix_web::{web, HttpResponse};
use mongodb::bson::DateTime;

use crate::database::daos::syncstate_dao::SyncStateDAO;
use crate::database::entities::syncstate::SyncStateGet;
use crate::jwt_utils::extract_user_oid;
use crate::Claims;

pub async fn get(
    _authenticated: Authenticated<Claims>,
    syncstate_get_data: web::Query<SyncStateGet>,
) -> actix_web::Result<HttpResponse> {
    let states = SyncStateDAO::get_since_for_user(
        DateTime::from_millis(syncstate_get_data.since),
        extract_user_oid(&_authenticated),
    )
    .await?;
    Ok(HttpResponse::Ok().json(states))
}
