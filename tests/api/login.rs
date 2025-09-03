//! src/tests/api/login.rs
use crate::helpers::assert_is_redirect_to;
use crate::helpers::spawn_app;
use serde_json::json;

#[tokio::test]
async fn error_cookie_should_be_set_on_failed_login_attempt() {
    // Arrange
    let app = spawn_app().await;

    let body = json!({
        "username": "randomusername",
        "password": "randompassword"
    });

    // Act 1 - POST with invalid credentials
    let response = app.post_login(&body).await;
    assert_is_redirect_to(&response, "/login");

    // Act 2 - Get the login page and assert the cookie is present
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Act 3 - Get the login page again and assert the cookie is deleted
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_on_login_success() {
    let app = spawn_app().await;

    let body = json!({
        "username": app.user.username,
        "password": app.user.password
    });

    let response = app.post_login(&body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;

    assert!(html_page.contains(&format!("Welcome {}!", app.user.username)));
}
