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
use crate::controller::utils::{DefaultResponse, get_default_insert_response, get_empty_success_response};
use crate::model::directory::{Directory, DirectoryPatch, DirectoryPost};


pub async fn create(authenticated: Authenticated<Claims>, dir_post_data: Json<DirectoryPost>) -> HttpResponse {
    let mut dir = Directory {
        id: None,
        parent_id: if dir_post_data.parent_id.is_some() { Some(ObjectId::from_str(dir_post_data.parent_id.to_owned().unwrap().as_str()).unwrap()) } else { None },
        name: dir_post_data.name.to_owned().to_string(),
        creation_date: DateTime::now(),
        child_ids: vec![],
        files: vec![],
    };

    let dir_detail = dir.create().await;
    get_default_insert_response(dir_detail)
}

pub async fn update(authenticated: Authenticated<Claims>, dir_post_data: Json<DirectoryPatch>) -> HttpResponse {
    let dir = Directory::get_by_oid(dir_post_data.id).await;

    if dir.is_some() {
        let mut dir = dir.unwrap();
        if dir_post_data.parent_id.is_some() {      // try to move dir if parent_id changes
            dir.move_to(dir_post_data.parent_id).await;
        }
        if dir_post_data.name.is_some() {
            dir.name = dir_post_data.name.to_owned().unwrap();
            let update_result = dir.update().await;

            if update_result.modified_count <= 0 {
                event!(Level::DEBUG, "renaming directory failed {:?}", update_result);
                return HttpResponse::InternalServerError().json(DefaultResponse {
                    result: None,
                    status: false,
                    error: "Renaming directory failed".parse().unwrap(),
                });
            }
        }
    } else {
        event!(Level::DEBUG, "Directory could not be found");
        return HttpResponse::InternalServerError().json(DefaultResponse {
            result: None,
            status: false,
            error: "Directory could not be found".parse().unwrap(),
        });
    }

    get_empty_success_response()
}
