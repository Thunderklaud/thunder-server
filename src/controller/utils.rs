use actix_web::HttpResponse;
use mongodb::results::InsertOneResult;
use mongodb::bson::extjson::de::Error;
use serde::Serialize;
use tracing::{event, Level};

#[derive(Serialize)]
pub struct DefaultResponse {
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

pub fn get_empty_success_response() -> HttpResponse {
    HttpResponse::Ok().json(DefaultResponse {
        result: None,
        status: true,
        error: "".parse().unwrap(),
    })
}