use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = test_app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_a_400_for_invalid_form_data() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let test_cases = vec![
        ("name=Kevin%20Kenyon", "missing the email"),
        ("email=kvkenyon%40gmail.com", "missing the name"),
        ("", "missing both"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(invalid_body.into()).await;
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 bad request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let test_cases = vec![
        ("name=&email=kevin%40turing.club", "empty name"),
        ("name=Kevin&email=not-an-email", "invalid email"),
        ("name=kevin&email=", "missing name"),
    ];

    for (body, desc) in test_cases {
        let response = test_app.post_subscriptions(body.into()).await;
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            desc
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let body = "name=Kevin&20Kenyon&email=kvkenyon%40gmail.com";
    test_app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let body = "name=Kevin%20Kenyon&email=kvkenyon%40gmail.com";
    test_app.post_subscriptions(body.into()).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];

    let confirmation_links = test_app.get_confirmation_links(email_request);
    assert_eq!(confirmation_links.text, confirmation_links.html);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = spawn_app().await;

    let body = "name=Kevin%20Kenyon&email=kvkenyon%40gmail.com";
    test_app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT name, email, status FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.status, "pending_confirmation");
    assert_eq!(saved.email, "kvkenyon@gmail.com");
    assert_eq!(saved.name, "Kevin Kenyon");
}

#[tokio::test]
async fn subscribe_persists_a_subscription_token_for_new_subscriber() {
    let test_app = spawn_app().await;

    let body = "name=Kevin%20Kenyon&email=kvkenyon%40gmail.com";
    test_app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT id FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    let id = saved.id;

    let saved = sqlx::query!(
        "SELECT subscription_token, subscriber_id FROM subscription_tokens WHERE subscriber_id = $1",
        id
    )
    .fetch_one(&test_app.db_pool)
    .await
    .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.subscriber_id, id);
}
