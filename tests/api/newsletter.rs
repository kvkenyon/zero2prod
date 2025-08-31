//! tests/api/newsletter.rs
use crate::helpers::{TestApp, spawn_app};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

// Hit a new endpoint
// /newsletter/deliver
// POST
// {
//   "plain_text": "plain text content",
//   "html": "<a></a>"
// }
// Iterate over all the subscribers that are confirmed
// let subscribers =  SELECT email, name FROM subscriptions WHERE status = 'confirmed';
// for s in subscribers { do_send(); }

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

    // Act
    let response = app
        .post_newsletters(json!({
            "title": "Newsletter 1",
            "content": {
                "html": "<p>Newsletter</p>",
                "text": "Newsletter"
            }
        }))
        .await;
    // Assert
    assert_eq!(response.status().as_u16(), 200);
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

    let response = app
        .post_newsletters(json!({
            "title": "Newsletter 1",
            "content": {
                "html": "<p>Newsletter</p>",
                "text": "Newsletter"
            }
        }))
        .await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    let test_cases = vec![
        (
            json!({
                "content": {
                    "html": "<p>hello</p>",
                    "text": "hello"
                }
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
                "content": {
                    "text": "This is a text only newsletter"
                }

            }),
            "no html",
        ),
        (
            json!({
                "title": "Newsletter 1",
                "content": {
                    "html": "<p>This is a html only newsletter</p>"
                }

            }),
            "no text",
        ),
        (json!({}), "no title or content"),
    ];

    for invalid_body in test_cases {
        let response = reqwest::Client::new()
            .post(format!("{}/newsletters", app.address))
            .json(&invalid_body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 400);
    }
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
