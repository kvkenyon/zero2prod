//! src/test/admin_dashboard.rs
use crate::helpers::{assert_is_redirect_to, spawn_app};
use serde_json::json;

#[tokio::test]
pub async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
pub async fn admin_page_has_link_to_change_password_page() {
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

    let page_html = app.get_admin_dashboard_html().await;

    assert!(page_html.contains(r#"<a href="/admin/password">Change password</a>"#));
}
