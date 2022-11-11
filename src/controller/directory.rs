use std::str::FromStr;

use actix_jwt_authc::Authenticated;
use actix_web::{HttpResponse, web::Json};
use mongodb::bson::DateTime;
use mongodb::bson::datetime::DateTimeBuilder;
use mongodb::bson::extjson::de::Error;
use mongodb::bson::oid::ObjectId;
use mongodb::results::InsertOneResult;
use serde::Serialize;
use tracing::{event, Level};

use crate::Claims;
use crate::controller::utils::{DefaultStringResponse, get_default_insert_response, get_empty_success_response};
use crate::model::directory::{Directory, DirectoryGet, DirectoryPatch, DirectoryPost, MinimalDirectoryObject, ROOT_DIR_OID};

#[derive(Serialize)]
pub struct DefaultResponse {
    #[serde(flatten)]
    result: Option<ResultDataType>,
    status: bool,
    error: String,
}

#[derive(Serialize)]
enum ResultDataType {
    #[serde(rename(serialize = "result"))]
    DirectoryGetResponse(DirectoryGetResponse),
}

#[derive(Serialize)]
struct DirectoryGetResponse {
    dirs: Vec<MinimalDirectoryObject>,
}

pub async fn create(_authenticated: Authenticated<Claims>, dir_post_data: Json<DirectoryPost>) -> HttpResponse {
    let mut parent_id = Some(ROOT_DIR_OID.get().unwrap().to_owned());
    if dir_post_data.parent_id.is_some() && !dir_post_data.parent_id.to_owned().unwrap().to_string().eq("") {
        parent_id = Some(ObjectId::from_str(dir_post_data.parent_id.to_owned().unwrap().as_str()).unwrap())
    }

    let mut dir = Directory {
        id: None,
        parent_id,
        name: dir_post_data.name.to_owned().to_string(),
        creation_date: DateTime::now(),
        child_ids: vec![],
        files: vec![],
    };

    let dir_detail = dir.create().await;
    get_default_insert_response(dir_detail)
}

pub async fn update(_authenticated: Authenticated<Claims>, dir_post_data: Json<DirectoryPatch>) -> HttpResponse {
    let dir = Directory::get_by_oid(dir_post_data.id).await;

    if dir.is_some() {
        let mut dir = dir.unwrap();
        if dir_post_data.parent_id.is_some() {      // move dir if parent_id changes
            if !dir_post_data.parent_id.to_owned().unwrap().to_string().eq("") {
                dir.move_to(ObjectId::from_str(dir_post_data.parent_id.to_owned().unwrap().as_str()).unwrap()).await;
            } else {
                dir.move_to(ROOT_DIR_OID.get().unwrap().to_owned()).await;
            }
        }
        if dir_post_data.name.is_some() {
            dir.name = dir_post_data.name.to_owned().unwrap();
            let update_result = dir.update().await;

            if update_result.modified_count <= 0 {
                event!(Level::DEBUG, "renaming directory failed {:?}", update_result);
                return HttpResponse::InternalServerError().json(DefaultStringResponse {
                    result: None,
                    status: false,
                    error: "Renaming directory failed".parse().unwrap(),
                });
            }
        }
    } else {
        event!(Level::DEBUG, "Directory could not be found");
        return HttpResponse::InternalServerError().json(DefaultStringResponse {
            result: None,
            status: false,
            error: "Directory could not be found".parse().unwrap(),
        });
    }

    get_empty_success_response()
}

pub async fn get(_authenticated: Authenticated<Claims>, dir_get_data: Json<DirectoryGet>) -> HttpResponse {
    let mut id = Some(ROOT_DIR_OID.get().unwrap().to_owned());
    if dir_get_data.id.is_some() && !dir_get_data.id.to_owned().unwrap().to_string().eq("") {
        id = Some(ObjectId::from_str(dir_get_data.id.to_owned().unwrap().as_str()).unwrap());
    }

    let dir = Directory::get_by_oid(id.unwrap()).await;
    if dir.is_some() {
        let dir_names = Directory::get_all_with_parent_id(dir.unwrap().id).await;

        return HttpResponse::Ok().json(DefaultResponse {
            result: Some(ResultDataType::DirectoryGetResponse(DirectoryGetResponse {
                dirs: dir_names
            })),
            status: true,
            error: "".parse().unwrap(),
        });
    }

    HttpResponse::InternalServerError().json(DefaultResponse {
        result: None,
        status: true,
        error: "Could not get requested directory".parse().unwrap(),
    })
}
