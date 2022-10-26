use std;
use std::io;
use bson;
use bson::doc;
use bson::oid::ObjectId;
use mongodb::ThreadedClient;

use crate::{database, SETTINGS};

#[derive(Debug)]
pub struct Model {
    pub email: String,
    pub pw_hash: String,
    pub role: String,
}

impl Model {
    pub fn to_bson(&self) -> bson::ordered::OrderedDocument {
        doc! {
          "email": self.email.to_owned(),
          "pw_hash": self.pw_hash.to_owned(),
          "role": self.role.to_owned(),
        }
    }

    pub async fn create(&self) -> Result<std::option::Option<bson::ordered::OrderedDocument>, io::Error> {
        let client = database::establish_connection();
        let settings = SETTINGS.get().unwrap();
        let collection = client.db(&settings.database.name).collection("users");
        collection.insert_one(self.to_bson().clone(), None)
            .ok().expect("Failed to insert user.");

        let response_document = collection.find_one(Some(self.to_bson().clone()), None)
            .ok().expect("Failed to execute find user.");

        Ok(response_document)
    }
}
