//use futures::AsyncWrite;
//use rocket::http::Status;
use rocket::tokio::runtime::Runtime;
use rocket::{route, Data, Request};
use tracing::{event, Level};

use crate::model;
/*
pub fn login<'r>(req: &'r Request, data: Data<'r>) -> route::BoxFuture<'r> {
    let param = req
        .param::<&'r str>(1)
        .and_then(Result::ok)
        .unwrap_or("unnamed");

    route::Outcome::from(req, param).pin()
}
pub fn logout<'r>(req: &'r Request, data: Data<'r>) -> route::BoxFuture<'r> {
    let param = req
        .param::<&'r str>(0)
        .and_then(Result::ok)
        .unwrap_or("unnamed");

    route::Outcome::from(req, param).pin()
}*/

pub fn register<'r>(req: &'r Request, _data: Data<'r>) -> route::BoxFuture<'r> {
    let tokio_runtime = Runtime::new().unwrap();

    let param = req
        .param::<&'r str>(0)
        .and_then(Result::ok)
        .unwrap_or("unnamed");

    let mut new_user = model::user::User {
        id: None,
        email: "mail@example.com".to_string(),
        pw_hash: "".to_string(),
        role: "testrole".to_string(),
    };
    let user_save_result = tokio_runtime.handle().block_on(new_user.create());
    if user_save_result.is_ok() {
        event!(Level::INFO, "user creation successful");
    } else {
        event!(Level::WARN, "user creation failed");
    }

    route::Outcome::from(req, param).pin()
}
