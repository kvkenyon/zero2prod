use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use std::error::Error;
use tracing_error::SpanTrace;
use uuid::Uuid;

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
    match insert_subscription(&pool, &form).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Saving new subscriber details in database", skip(pool, form))]
async fn insert_subscription(pool: &PgPool, form: &FormData) -> Result<(), SubscriptionError> {
    sqlx::query!(
        r#"
INSERT INTO subscriptions (id, email, name, subscribed_at)
VALUES ($1, $2, $3, $4)
"#,
        Uuid::new_v4(),
        form.email,
        form.name,
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
