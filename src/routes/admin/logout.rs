//! src/routes/admin/logout.rs
use crate::{
    authentication::UserId, routes::admin::helpers::see_other, session_state::TypedSession,
};
use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;

pub async fn logout(
    user_id: web::ReqData<UserId>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    user_id.into_inner();
    session.logout();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(see_other("/login"))
}
