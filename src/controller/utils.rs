use std::str::FromStr;

use mongodb::bson::oid::ObjectId;
use serde::Serialize;

#[derive(Serialize)]
pub struct DefaultStringResponse {
    pub result: Option<String>,
    pub status: bool,
    pub error: String,
}

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
