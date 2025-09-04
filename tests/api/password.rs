//! src/tests/api/password.rs
use crate::helpers::assert_is_redirect_to;
use crate::helpers::spawn_app;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn get_change_password_redirects_to_login_when_not_logged_in() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn get_change_password_displays_change_password_form_when_logged_in() {
    let app = spawn_app().await;

    let response = app
        .post_login(&json!(
            {
                "username": app.user.username,
                "password": app.user.password
            }
        ))
        .await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let response = app.get_change_password().await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn change_password_displays_error_when_new_passwords_are_not_equal() {
    // Arrange
    let app = spawn_app().await;

    let response = app
        .post_login(&json!(
            {
                "username": app.user.username,
                "password": app.user.password
            }
        ))
        .await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();
    let verify_new_password = Uuid::new_v4().to_string();

    assert_ne!(new_password, verify_new_password);

    let body = json!({
        "current_password": app.user.password,
        "new_password": new_password,
        "verify_new_password":verify_new_password
    });

    // Act
    let response = app.post_change_password(&body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/dashboard/password");
    let page_html = app.get_change_password_html().await;

    assert!(page_html.contains("The new passwords need to match"));
}

#[tokio::test]
async fn change_password_displays_error_when_using_invalid_current_password() {
    // Arrange
    let app = spawn_app().await;

    // Login the test user and assert we redirect to the dashboard
    let response = app
        .post_login(&json!(
            {
                "username": app.user.username,
                "password": app.user.password
            }
        ))
        .await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();

    let body = json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password":  new_password,
        "verify_new_password": new_password
    });

    let response = app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/admin/dashboard/password");
    let page_html = app.get_change_password_html().await;

    assert!(page_html.contains("The current password you entered is invalid"));
}

#[tokio::test]
async fn change_password_displays_success_message_on_success() {
    let app = spawn_app().await;
    // Login the test user and assert we redirect to the dashboard
    let response = app
        .post_login(&json!(
            {
                "username": app.user.username,
                "password": app.user.password
            }
        ))
        .await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();

    let body = json!({
        "current_password": app.user.password,
        "new_password":  new_password,
        "verify_new_password": new_password
    });

    let response = app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/admin/dashboard/password");
    let page_html = app.get_change_password_html().await;

    assert!(page_html.contains("Password changed successfully!"));
}
