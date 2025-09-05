//! src/routes/admin/password/post.rs
use crate::authentication::{AuthError, Credentials, UserId, validate_credentials};
use crate::domain::Password;
use crate::routes::admin::helpers::{e500, see_other};
use crate::routes::get_username;
use actix_web::HttpResponse;
use actix_web::web;
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, Params, PasswordHasher};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    verify_new_password: Secret<String>,
}

#[tracing::instrument(
    name = "Handle password change",
    skip(form, user_id, pool), fields(
    user_id=tracing::field::Empty
))]
pub async fn change_password(
    pool: web::Data<PgPool>,
    web::Form(form): web::Form<FormData>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    // Try to get the user_id from the session (redirect to login on invalid session)
    let user_id = user_id.into_inner();

    // Handle case where new passwords don't match
    if form.new_password.expose_secret() != form.verify_new_password.expose_secret() {
        tracing::info!("new passwords are not equal");
        FlashMessage::error("The new passwords need to match").send();
        return Ok(see_other("/admin/password"));
    }

    let new_password = form.new_password.clone();
    if let Err(e) = Password::parse(new_password) {
        FlashMessage::error(e).send();
        return Ok(see_other("/admin/password"));
    }

    // Fetch the username so we can validate the users credentials
    let username = get_username(&pool, &user_id).await.map_err(e500)?;

    // Attempt to validate the credentials
    let credentials = Credentials::new(username, form.current_password);
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password you entered is invalid").send();
                return Ok(see_other("/admin/password"));
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    // Set the new password
    match set_new_password(&pool, *user_id, &form.new_password).await {
        Ok(_) => {
            FlashMessage::info("Password changed successfully!").send();
        }
        Err(e) => {
            tracing::error!("{e:?}");
            FlashMessage::error("Password change failed, please try again.").send();
        }
    }
    Ok(see_other("/admin/password"))
}

#[tracing::instrument(name = "Set new password", skip(pool, new_password))]
pub async fn set_new_password(
    pool: &PgPool,
    user_id: Uuid,
    new_password: &Secret<String>,
) -> Result<(), anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(15000, 2, 1, None).expect("Failed to build params."),
    )
    .hash_password(new_password.expose_secret().as_bytes(), &salt)
    .unwrap()
    .to_string();
    sqlx::query!(
        r#"UPDATE users SET password_hash = $1 WHERE user_id = $2;"#,
        password_hash,
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to set password hash")?;
    Ok(())
}
