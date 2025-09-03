//! src/tests/api/login.rs
use crate::helpers::assert_is_redirect_to;
use crate::helpers::spawn_app;
use serde_json::json;

#[tokio::test]
async fn error_cookie_should_be_set_on_failed_login_attempt() {
    let app = spawn_app().await;

    let body = json!({
        "username": "randomusername",
        "password": "randompassword"
    });

    let response = app.post_login(&body).await;

    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();

    assert_eq!(flash_cookie.value(), "Authentication failed");
    assert_is_redirect_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}
