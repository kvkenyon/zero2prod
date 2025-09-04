//! src/routes/admin/helpers.rs
use actix_web::{HttpResponse, error::InternalError, http::StatusCode, http::header::LOCATION};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(pool: &PgPool, user_id: &Uuid) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform query to retrieve a username.")?;
    Ok(row.username)
}

pub fn login_redirect<E>(e: E) -> InternalError<E>
where
    E: std::string::ToString,
{
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::build(StatusCode::SEE_OTHER)
        .insert_header((LOCATION, "/login"))
        .finish();
    InternalError::from_response(e, response)
}

pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::build(StatusCode::SEE_OTHER)
        .insert_header((LOCATION, location))
        .finish()
}
