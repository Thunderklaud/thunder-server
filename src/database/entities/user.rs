use mongodb::bson::oid::ObjectId;
use ring::test::from_hex;
use serde::{Deserialize, Serialize};

use crate::database::database::MyDBModel;
use strum_macros::AsRefStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub pw_hash: String,
    pub role: Role,
    pub root_dir_id: Option<ObjectId>,
}

impl MyDBModel for User {
    fn type_name() -> &'static str {
        "User"
    }
}

#[derive(Debug, Serialize, Deserialize, AsRefStr)]
pub enum Role {
    Admin,
    BaseUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLogin {
    pub email: String,
    pub pw_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRegister {
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub pw_hash: String,
}

impl User {
    pub fn is_valid_hash_design(hash: &str) -> bool {
        let pw_bytes_res = from_hex(hash);

        // sha256 requires 32 bytes = 256 bit
        // sha512 requires 64 bytes = 512 bit
        pw_bytes_res.is_ok() && pw_bytes_res.to_owned().unwrap().len() >= 32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hash_design() {
        let trash = "asdf1234567xx";
        let sha1 = "4fcdced7b0bdb6d4861c458c74bf0b8ace258c5d";
        let sha256 = "1bc464c87c470882de2453b9978c4fa61dd680c30617b68c5ac1d4052ed39aef";
        let sha512 = "12320d5e2a6c4f869f9dcca6fce9f36a9e51d8e324538adfbd0631f18011a2bbbcb5824150de3b1704d7b38a164eab368dcb1c396e0fe3febc5dc1e792e46660";

        assert_eq!(User::is_valid_hash_design(trash), false);
        assert_eq!(User::is_valid_hash_design(sha1), false);
        assert_eq!(User::is_valid_hash_design(sha256), true);
        assert_eq!(User::is_valid_hash_design(sha512), true);
    }
}
