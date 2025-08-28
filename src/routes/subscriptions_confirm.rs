use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize, Debug)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Handle subscription confirmation", skip(pool, parameters))]
pub async fn confirm(pool: web::Data<PgPool>, parameters: web::Query<Parameters>) -> HttpResponse {
    let subscriber_id =
        match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
            Ok(id) => id,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };
    match subscriber_id {
        None => HttpResponse::Unauthorized().finish(),
        Some(subscriber_id) => match confirm_subscriber(&pool, subscriber_id).await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(_) => HttpResponse::InternalServerError().finish(),
        },
    }
}

#[tracing::instrument(name = "Get subscriber_id from subscription token", skip(pool, token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
    SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1
    "#,
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
