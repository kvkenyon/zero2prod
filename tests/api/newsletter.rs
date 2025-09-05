//! tests/api/newsletter.rs
use crate::helpers::assert_is_redirect_to;
use crate::helpers::{TestApp, spawn_app};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_subscriber(&app, false).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

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

    // Act
    let response = app
        .post_newsletters(json!({
            "title": "Newsletter 1",
            "content_html": "<p>newsletter content html</p>",
            "content_text": "newsletter content text"
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscriber() {
    let app = spawn_app().await;
    create_subscriber(&app, true).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

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

    let response = app
        .post_newsletters(json!({
            "title": "Newsletter 1",
            "content_html": "<p>newsletter content html</p>",
            "content_text": "newsletter content text"
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
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
    let test_cases = vec![
        (
            json!({
                "content_text": "asdf",
                "content_html": "adsdf"
            }),
            "no title",
        ),
        (
            json!({
                "title": "Newsletter 1"
            }),
            "no content",
        ),
        (
            json!({
                "title": "Newsletter 1",
                "content_text": "asdfasdf"

            }),
            "no html",
        ),
        (
            json!({
                "title": "Newsletter 1",
                "content_html": "asdfasdf"

            }),
            "no text",
        ),
        (json!({}), "no title or content"),
    ];

    for (invalid_body, error) in test_cases {
        let response = app.post_newsletters(invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 bad request when the payload was {}.",
            error
        );
    }
}

#[tokio::test]
async fn admin_dashboard_should_have_link_to_publish_newsletter() {
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

    let page_html = app.get_admin_dashboard_html().await;

    assert!(page_html.contains(r#"<a href="/admin/newsletters">Publish a newsletter issue</a>"#));
}

#[tokio::test]
async fn you_can_visit_newsletter_form_when_logged_in() {
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

    let response = app.get_newsletters_form().await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn you_can_publish_a_newsletter_when_logged_in() {
    let app = spawn_app().await;

    create_subscriber(&app, true).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app
        .post_login(&json!(
            {
                "username": app.user.username,
                "password": app.user.password
            }
        ))
        .await;

    assert_is_redirect_to(&response, "/admin/dashboard");

    let response = app.get_newsletters_form().await;

    assert_eq!(response.status().as_u16(), 200);

    let response = app
        .post_newsletters(json!({
            "title": "Newsletter 1",
            "content_html": "<p>newsletter content html</p>",
            "content_text": "newsletter content text"
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");
}

async fn create_subscriber(app: &TestApp, is_confirmed: bool) {
    let body = "name=Le%20Guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    if is_confirmed {
        let email_request = &app.email_server.received_requests().await.unwrap()[0];

        let confirmation_links = app.get_confirmation_links(email_request);

        reqwest::get(confirmation_links.text).await.unwrap();
    }
}
