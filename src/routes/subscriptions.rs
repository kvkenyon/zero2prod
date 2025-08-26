use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use std::error::Error;
use tracing_error::SpanTrace;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(Debug)]
pub struct SubscriptionError {
    message: String,
    span_trace: SpanTrace,
}

impl SubscriptionError {
    pub fn new(message: impl Into<String>) -> Self {
        SubscriptionError {
            span_trace: SpanTrace::capture(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error: {}", self.message)?;
        self.span_trace.fmt(f)?;
        Ok(())
    }
}

impl Error for SubscriptionError {}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[derive(serde::Deserialize)]
#[allow(unused)]
pub struct FormData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    // A fallible conversion that consumes (moves) to input value.
    let new_subscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_subscription(&pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(pool, new_subscriber)
)]
async fn insert_subscription(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), SubscriptionError> {
    sqlx::query!(
        r#"
INSERT INTO subscriptions (id, email, name, subscribed_at)
VALUES ($1, $2, $3, $4)
"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|err| {
        let err = SubscriptionError::new(err.to_string());
        tracing::error!("{}", err);
        err
    })?;

    Ok(())
}
