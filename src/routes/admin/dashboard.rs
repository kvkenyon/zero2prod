//! src/routes/admin/dashboard.rs
use crate::session_state::TypedSession;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, http::header::ContentType, http::header::LOCATION};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

#[tracing::instrument(name = "Get admin dashboard", skip(pool, session))]
pub async fn admin_dashboard(
    pool: actix_web::web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(&pool, &user_id).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::build(StatusCode::SEE_OTHER)
            .insert_header((LOCATION, "/login"))
            .finish());
    };
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
</body>
</html>"#
        )))
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(pool: &PgPool, user_id: &Uuid) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform query to retrieve a username.")?;
    Ok(row.username)
}
