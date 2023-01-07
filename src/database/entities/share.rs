use crate::database::database::MyDBModel;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub corresponding_id: ObjectId,
    pub corresponding_parent_id: ObjectId,
    r#type: String,
    label: String,
    pub max_dl_count: Option<u32>, // max. 4294967295 with u32
    pub current_dl_count: u32,
    pub valid_until: Option<DateTime>,
    pub creation_date: DateTime,
}

impl MyDBModel for Share {
    fn type_name() -> &'static str {
        "Share"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareGet {
    pub id: ObjectId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareDelete {
    pub id: ObjectId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileShareCreate {
    pub uuid: String,
    pub label: String,
    pub max_dl_count: Option<u32>, // max. 4294967295 with u32
    pub valid_until: Option<DateTime>,
}

pub enum ShareType {
    Directory,
    File,
}

impl Share {
    fn get_type_match(state_type: ShareType) -> String {
        match state_type {
            ShareType::Directory => "Directory".to_string(),
            ShareType::File => "File".to_string(),
        }
    }
    pub fn new(
        state_type: ShareType,
        corresponding_id: ObjectId,
        corresponding_parent_id: ObjectId,
        user_id: ObjectId,
        label: String,
        max_dl_count: Option<u32>,
        valid_until: Option<DateTime>,
    ) -> Share {
        Share {
            id: None,
            user_id,
            corresponding_id,
            corresponding_parent_id,
            r#type: Share::get_type_match(state_type),
            label,
            max_dl_count,
            current_dl_count: 0_u32,
            valid_until,
            creation_date: DateTime::now(),
        }
    }
}
