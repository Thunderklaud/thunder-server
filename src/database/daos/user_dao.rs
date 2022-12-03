use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use crate::database::daos::dao::DAO;
use crate::database::entities::user::User;

pub trait UserDAO: DAO<User, ObjectId> {}

/*
#[async_trait]
impl DAO<User, ObjectId> for User {
    async fn get(_: ObjectId) -> actix_web::Result<Option<User>> {
        todo!()
    }

    async fn insert(&mut _: User) -> actix_web::Result<ObjectId> {
        todo!()
    }

    async fn update(&mut _: User) -> actix_web::Result<u64> {
        todo!()
    }

    async fn delete(&mut _: User) -> actix_web::Result<Option<ObjectId>> {
        todo!()
    }
}

// custom methods
impl UserDAO for User {

}
*/