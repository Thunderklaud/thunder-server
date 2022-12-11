use std::str::FromStr;

use mongodb::bson::oid::ObjectId;

pub fn extract_object_id(
    id: Option<&String>,
    default_if_none: ObjectId,
) -> actix_web::Result<ObjectId> {
    Ok(match id {
        Some(parent_id) if !parent_id.is_empty() => {
            ObjectId::from_str(parent_id).map_err(|e| actix_web::error::ErrorBadRequest(e))?
        }
        _ => default_if_none,
    })
}

pub fn extract_object_id_or_die(id: Option<&String>) -> actix_web::Result<ObjectId> {
    if let Some(id) = id {
        return Ok(ObjectId::from_str(id).map_err(|e| actix_web::error::ErrorBadRequest(e))?);
    }

    Err(actix_web::error::ErrorBadRequest("no ObjectId to extract"))
}
