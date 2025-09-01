//! src/routes/newsletters.rs
use crate::{
    domain::SubscriberEmail, email_client::EmailClient, routes::error_chain_fmt, telemetry,
};
use actix_web::{
    HttpRequest, HttpResponse, ResponseError,
    http::StatusCode,
    http::header::{HeaderMap, HeaderValue, WWW_AUTHENTICATE},
    web,
};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::ExposeSecret;
use secrecy::Secret;
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

#[tracing::instrument(name = "Get stored credentials", skip(pool, username))]
async fn get_stored_credentials(
    pool: &PgPool,
    username: &str,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT user_id, password_hash FROM users WHERE username = $1;"#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perfrom a query to retrieve stored credentials.")?
    .map(|r| (r.user_id, Secret::new(r.password_hash)));
    Ok(row)
}

#[tracing::instrument(name = "Verify password hash", skip(password, expected_password_hash))]
fn verify_password(
    password: Secret<String>,
    expected_password_hash: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC format.")
        .map_err(PublishError::UnexpectedError)?;
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_password_hash)
        .context("Failed to verify password")
        .map_err(PublishError::AuthError)
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
gZiV/M1gPc22ElAH/Jh1Hw$\
CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );
    if let Some((stored_user_id, stored_expected_password_hash)) =
        get_stored_credentials(pool, &credentials.username)
            .await
            .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_expected_password_hash;
    };
    telemetry::spawn_blocking_with_tracing(move || {
        verify_password(credentials.password, expected_password_hash)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(PublishError::UnexpectedError)??;

    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(name = "Publish a newsletter issue", skip(pool, email_client, request), fields(
    username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    newsletter: web::Json<Newsletter>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));
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

struct Credentials {
    username: String,
    password: Secret<String>,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not valid UTF-8");

    let base64encoded_segment = header_value?
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64 decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;
    // Split into two segments, using ':' as delimiter
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish"#).unwrap();
                response
                    .headers_mut()
                    .insert(WWW_AUTHENTICATE, header_value);
                response
            }
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
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
