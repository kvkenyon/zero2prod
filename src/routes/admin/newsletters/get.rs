//! src/routes/admin/newsletters/get.rs
use crate::authentication::UserId;
use actix_web::{HttpResponse, http::header::ContentType, web};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

#[tracing::instrument(name = "Get publish newsletter form", skip(user_id, flash_messages))]
pub async fn publish_newsletter_form(
    flash_messages: IncomingFlashMessages,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    user_id.into_inner();

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
  <title>Publish Newsletter</title>
</head>
<body>
    <h1>Publish Newsletter</h1>
    {msg_html}
    <form action="/admin/newsletters" method="post">
      <label for="title">Title
      <input type="text" name="title" placeholder="Enter a newsletter title">
      </label>
      <br />
      <label for="content">Text content
      <textarea  name="text_content" placeholder="Write your newsletter (plain text)">
      </label>
      <br />
      <label for="html_content">HTML content
      <textarea  name="html_content" placeholder="Write your newsletter (html)">
      </label>
      <br />
      <button type="submit">Submit</button>
     <form>
</body>
</html>
    "#,
        )))
}
