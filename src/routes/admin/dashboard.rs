//! src/routes/admin/dashboard.rs
use crate::routes::{admin::helpers::get_username, e500, see_other};
use crate::session_state::TypedSession;
use actix_web::{HttpResponse, http::header::ContentType};
use sqlx::PgPool;

#[tracing::instrument(name = "Get admin dashboard", skip(pool, session))]
pub async fn admin_dashboard(
    pool: actix_web::web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(&pool, &user_id).await.map_err(e500)?
    } else {
        return Ok(see_other("/login"));
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
<p>Available actions:</p>
<ol>
    <li><a href="/admin/password">Change password</a></li>
</ol>
</body>
</html>"#
        )))
}
