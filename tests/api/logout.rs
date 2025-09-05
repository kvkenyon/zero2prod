//! src/tests/api/logout.rs
use crate::helpers::{assert_is_redirect_to, spawn_app};
use serde_json::json;

#[tokio::test]
async fn you_should_have_a_logout_action_in_admin_dashboard() {
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

    let response = app.get_admin_dashboard_html().await;

    assert!(response.contains(r#"<a href="/admin/logout">Logout</a>"#));
}

#[tokio::test]
async fn you_should_be_redirected_to_login_on_logout_with_success_message() {
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

    let response = app.get_logout().await;

    assert_is_redirect_to(&response, "/login");

    let page_html = app.get_login_html().await;

    assert!(page_html.contains("You have successfully logged out."));
}

#[tokio::test]
async fn you_should_lose_access_to_admin_dashboard_after_logout() {
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

    let response = app.get_logout().await;

    assert_is_redirect_to(&response, "/login");

    let response = app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login");
}
