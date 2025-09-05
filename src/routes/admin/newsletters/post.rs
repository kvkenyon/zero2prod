//! src/routes/admin/newsletters/post.rs
use crate::{
    authentication::UserId,
    domain::SubscriberEmail,
    email_client::EmailClient,
    routes::{admin::helpers::e500, see_other},
};
use actix_web::{HttpResponse, web};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

#[derive(serde::Deserialize, Debug)]
pub struct FormData {
    title: String,
    content_text: String,
    content_html: String,
}

#[tracing::instrument(name = "Publish a newsletter issue",
    skip(pool, email_client, user_id ),
    fields(user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
    web::Form(form): web::Form<FormData>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let confirmed_subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to get confirmed subscribers")
        .map_err(e500)?;

    for confirmed_subscriber in confirmed_subscribers {
        match confirmed_subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &form.title,
                    &form.content_html,
                    &form.content_text,
                )
                .await
                .with_context(|| {
                    format!(
                        "Failed to send email to subscriber with email: {}",
                        subscriber.email
                    )
                })
                .map_err(e500)?,
            Err(e) => {
                tracing::warn!(e.cause_chain = ?e, "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid.
                ");
            }
        };
    }
    FlashMessage::info("Newsletter published successfully!").send();
    Ok(see_other("/admin/newsletters"))
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
