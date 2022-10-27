use serde::{Deserialize, Serialize};
use wither::bson::{doc, oid::ObjectId};
use wither::{prelude::*, Result};

use crate::database;

// Define a model. Simple as deriving a few traits.
#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"email": 1}"#, options = r#"doc!{"unique": true}"#))]
pub struct User {
    /// The ID of the model.
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub pw_hash: String,
    pub role: String,
}

impl User {
    pub async fn create(&mut self) -> Result<()> {
        let db = database::establish_connection().await.unwrap();
        User::sync(&db).await?;

        self.save(&db, None).await
    }
}
