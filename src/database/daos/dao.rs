use crate::database::database;
use crate::database::database::MyDBModel;
use async_trait::async_trait;
use mongodb::Collection;

#[async_trait]
pub trait DAO<ENTITY: MyDBModel, IDENTIFIER> {
    async fn get_collection() -> Collection<ENTITY> {
        database::get_collection::<ENTITY>().await.clone_with_type()
    }
    async fn get(_: IDENTIFIER) -> actix_web::Result<Option<ENTITY>>;
    async fn get_with_user(
        _: IDENTIFIER,
        _user_id: IDENTIFIER,
    ) -> actix_web::Result<Option<ENTITY>>;
    async fn insert(_: &mut ENTITY) -> actix_web::Result<IDENTIFIER>;
    async fn update(_: &ENTITY) -> actix_web::Result<u64>;
    async fn delete(_: &ENTITY) -> actix_web::Result<u64>;
}
