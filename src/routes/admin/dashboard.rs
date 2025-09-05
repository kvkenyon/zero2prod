//! src/routes/admin/dashboard.rs
use crate::authentication::UserId;
use crate::routes::{admin::helpers::get_username, e500};
use actix_web::web;
use actix_web::{HttpResponse, http::header::ContentType};
use sqlx::PgPool;

#[tracing::instrument(name = "Get admin dashboard", skip(pool, user_id))]
pub async fn admin_dashboard(
    pool: actix_web::web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(&pool, &user_id).await.map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Admin dashboard</title>
</head>
<body>
<p>Welcome {username}!</p>
<p>Available actions:</p>
<ol>
    <li><a href="/admin/password">Change password</a></li>
    <li><a href="/admin/logout">Logout</a></li>
</ol>
</body>
</html>"#
        )))
}
