use async_trait::async_trait;

#[async_trait]
pub trait DAO<T, IDENTIFIER> {
    async fn get(_: IDENTIFIER) -> actix_web::Result<Option<T>>;
    async fn get_with_user(_: IDENTIFIER, _user_id: IDENTIFIER) -> actix_web::Result<Option<T>>;
    async fn insert(_: &mut T) -> actix_web::Result<IDENTIFIER>;
    async fn update(_: &T) -> actix_web::Result<u64>;
    async fn delete(_: &mut T) -> actix_web::Result<Option<IDENTIFIER>>;
}
