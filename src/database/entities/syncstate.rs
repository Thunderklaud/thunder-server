use crate::database::database::MyDBModel;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub corresponding_id: ObjectId,
    pub corresponding_parent_id: Option<ObjectId>,
    r#type: String,
    action: String,
    pub creation_date: DateTime,
}

impl MyDBModel for SyncState {
    fn type_name() -> &'static str {
        "SyncState"
    }
}

pub enum SyncStateType {
    Directory,
    File,
    User,
}

pub enum SyncStateAction {
    Create, // dir, file, user
    Rename, // dir, file
    Move,   // dir, file (list file info by id is required to get info about current folder)
    Delete, // dir, file, user
}

impl SyncState {
    fn get_type_match(state_type: SyncStateType) -> String {
        match state_type {
            SyncStateType::Directory => "Directory".to_string(),
            SyncStateType::File => "File".to_string(),
            SyncStateType::User => "User".to_string(),
        }
    }
    fn get_action_match(state_action: SyncStateAction) -> String {
        match state_action {
            SyncStateAction::Create => "create".to_string(),
            SyncStateAction::Rename => "rename".to_string(),
            SyncStateAction::Move => "move".to_string(),
            SyncStateAction::Delete => "delete".to_string(),
        }
    }
    pub fn add(
        state_type: SyncStateType,
        state_action: SyncStateAction,
        corresponding_id: ObjectId,
        corresponding_parent_id: Option<ObjectId>,
        user_id: ObjectId,
    ) -> SyncState {
        SyncState {
            id: None,
            user_id,
            corresponding_id,
            corresponding_parent_id,
            r#type: SyncState::get_type_match(state_type),
            action: SyncState::get_action_match(state_action),
            creation_date: DateTime::now(),
        }
    }
}
