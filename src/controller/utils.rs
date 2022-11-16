use std::str::FromStr;

use actix_web::HttpResponse;
use mongodb::bson::extjson::de::Error;
use mongodb::bson::oid::ObjectId;
use mongodb::results::InsertOneResult;
use serde::Serialize;
use tracing::{event, Level};

#[derive(Serialize)]
pub struct DefaultStringResponse {
    pub result: Option<String>,
    pub status: bool,
    pub error: String,
}

#[derive(Serialize)]
pub struct UtilizedDefaultResponse {
    #[serde(flatten)]
    result: Option<UtilizedResultDataType>,
    status: bool,
    error: String,
}

#[derive(Serialize)]
enum UtilizedResultDataType {
    #[serde(rename(serialize = "result"))]
    InsertOneResult(InsertOneResult),
}

pub fn get_default_insert_response(data_detail: Result<InsertOneResult, Error>) -> HttpResponse {
    event!(Level::DEBUG, "{:?}", data_detail);
    match data_detail {
        Ok(data) => HttpResponse::Ok().json(UtilizedDefaultResponse {
            result: Some(UtilizedResultDataType::InsertOneResult(data)),
            status: true,
            error: "".to_string(),
        }),
        Err(err) => HttpResponse::InternalServerError().json(UtilizedDefaultResponse {
            result: None,
            status: false,
            error: err.to_string(),
        }),
    }
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
