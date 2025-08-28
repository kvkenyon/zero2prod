use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{HttpResponse, web};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use std::error::Error;
use tracing_error::SpanTrace;
use uuid::Uuid;

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

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
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> HttpResponse {
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    // A fallible conversion that consumes (moves) the input value.
    let new_subscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let subscriber_id = match insert_subscription(&mut transaction, &new_subscriber).await {
        Ok(sid) => sid,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let subscription_token = generate_subscription_token();

    if insert_subscription_token(&mut transaction, &subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    };

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    };

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscription(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, SubscriptionError> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
INSERT INTO subscriptions (id, email, name, subscribed_at, status)
VALUES ($1, $2, $3, $4, 'pending_confirmation')
"#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    transaction.execute(query).await.map_err(|err| {
        let err = SubscriptionError::new(err.to_string());
        tracing::error!("{}", err);
        err
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Send confirmation email to new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{base_url}/subscriptions/confirm?subscription_token={}",
        subscription_token
    );
    let text_content = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_content = format!(
        "Welcome to our newsletter!<br/>\
            <a href=\"{}\">Click here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &html_content,
            &text_content,
        )
        .await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(transaction, subscriber_id, subscription_token)
)]

async fn insert_subscription_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: &Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
    INSERT INTO subscription_tokens (subscription_token, subscriber_id) VALUES ($1, $2)
    
    "#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(|err| {
        tracing::error!("{:?}", err);
        err
    })?;
    Ok(())
}
