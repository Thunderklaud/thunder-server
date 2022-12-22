use crate::controller::utils::extract_object_id_or_die;
use crate::database::daos::dao::DAO;
use crate::database::daos::user_dao::UserDAO;
use crate::database::entities::user::Role;

pub async fn update_user_role(uid: String, role: Role) -> actix_web::Result<()> {
    let uid = extract_object_id_or_die(Some(&uid))?;
    let user = UserDAO::get(uid).await?;

    if let Some(mut user) = user {
        user.role = role;
        UserDAO::update(&user).await?;
    }

    Ok(())
}
