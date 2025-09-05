use actix_web::{HttpResponse, http::header::ContentType};
use std::fmt::Write;

use actix_web_flash_messages::IncomingFlashMessages;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let html_template = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="content-type" content="text/html; charset=utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Login</title>
</head>
<body>
    <h1>Login</h1>
    {msg_html}
    <form action="/login" method="post">
      <label for="username">Username
        <input type="text" name="username" placeholder="Enter username">
      </label>
      <br />
      <label for="Password">Password
      <input type="password" name="password" placeholder="Enter password">
      </label>
      <button type="submit">Login</button>
     <form>
</body>
</html>
    "#
    );
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_template)
}
