use actix_web::{HttpResponse, http::StatusCode, web};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Handle subscription confirmation", skip(pool, parameters))]
pub async fn confirm(
    pool: web::Data<PgPool>,
    parameters: web::Query<Parameters>,
) -> Result<HttpResponse, ConfirmError> {
    let subscriber_id = get_subscriber_id_from_token(&pool, &parameters.subscription_token)
        .await
        .context("Failed to get subscriber id from token.")?;
    match subscriber_id {
        None => Err(ConfirmError::UnauthorizedError(
            "Invalid confirmation token".into(),
        )),
        Some(subscriber_id) => {
            confirm_subscriber(&pool, subscriber_id)
                .await
                .context("Failed to match confirm subscriber.")?;
            Ok(HttpResponse::Ok().finish())
        }
    }
}

pub fn error_chain_fmt(
    f: &mut std::fmt::Formatter<'_>,
    e: &impl std::error::Error,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum ConfirmError {
    #[error("{0}")]
    UnauthorizedError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl actix_web::ResponseError for ConfirmError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ConfirmError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConfirmError::UnauthorizedError(_) => StatusCode::UNAUTHORIZED,
        }
    }
}

impl std::fmt::Debug for ConfirmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(f, self)
    }
}

#[tracing::instrument(name = "Get subscriber_id from subscription token", skip(pool, token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, GetSubscriberIdError> {
    let result = sqlx::query!(
        r#"
    SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1
    "#,
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(GetSubscriberIdError)?;
    Ok(result.map(|r| r.subscriber_id))
}

#[derive(thiserror::Error)]
#[error("A database error occurred while fetching the subscriber from the token.")]
pub struct GetSubscriberIdError(#[source] sqlx::Error);

impl std::fmt::Debug for GetSubscriberIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(f, self)
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(
    pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<(), ConfirmSubscriberError> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(ConfirmSubscriberError)?;
    Ok(())
}

#[derive(thiserror::Error)]
#[error("A database error occurred while marking subscriber as confirmed.")]
pub struct ConfirmSubscriberError(#[source] sqlx::Error);

impl std::fmt::Debug for ConfirmSubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(f, self)
    }
}
