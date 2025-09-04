//! src/routes/admin/password/get.rs
use crate::{
    routes::admin::helpers::{e500, see_other},
    session_state::TypedSession,
};
use actix_web::{HttpResponse, http::header::ContentType};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

#[tracing::instrument(name = "Get change password form", skip(session, flash_messages))]
pub async fn change_password_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(see_other("/login"));
    }

    let mut msg_html = String::new();
    for m in flash_messages
        .iter()
        .filter(|m| m.level() == Level::Error || m.level() == Level::Info)
    {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="content-type" content="text/html; charset=utf-8">
  <title>Admin Dashboard - Change Password</title>
</head>
<body>
    <h1>Change Password</h1>
    {msg_html}
    <form action="/admin/dashboard/password" method="post">
      <label for="Password">Current password
      <input type="password" name="current_password" placeholder="Enter your current password">
      </label>
      <br />
      <label for="Password">New password
      <input type="password" name="new_password" placeholder="Enter a new password">
      </label>
      <br />
      <label for="Password">Verify new password
      <input type="password" name="verify_new_password" placeholder="Enter the new password again">
      </label>
      <br />
      <button type="submit">Submit</button>
     <form>
</body>
</html>
    "#,
        )))
}
