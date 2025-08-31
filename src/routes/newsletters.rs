//! src/routes/newsletters.rs
use crate::{domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt};
use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use anyhow::Context;
use sqlx::PgPool;

#[derive(serde::Deserialize, Debug)]
pub struct Newsletter {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize, Debug)]
pub struct Content {
    text: String,
    html: String,
}

#[tracing::instrument(name = "Publish a newsletter issue", skip(pool, email_client))]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    newsletter: web::Json<Newsletter>,
) -> Result<HttpResponse, PublishError> {
    let confirmed_subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to get confirmed subscribers")?;

    for confirmed_subscriber in confirmed_subscribers {
        match confirmed_subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &newsletter.title,
                    &newsletter.content.html,
                    &newsletter.content.text,
                )
                .await
                .with_context(|| {
                    format!(
                        "Failed to send email to subscriber with email: {}",
                        subscriber.email
                    )
                })?,
            Err(e) => {
                tracing::warn!(e.cause_chain = ?e, "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid.
                ");
            }
        };
    }
    Ok(HttpResponse::Ok().finish())
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(f, self)
    }
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed';"#)
        .fetch_all(pool)
        .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(subscriber_email) => Ok(ConfirmedSubscriber {
                email: subscriber_email,
            }),
            Err(e) => Err(anyhow::anyhow!(e)),
        })
        .collect();
    Ok(confirmed_subscribers)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}
