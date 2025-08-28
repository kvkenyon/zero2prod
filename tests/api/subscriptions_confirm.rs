//! tests/api/subscriptions_confirma.rs
use crate::helpers::spawn_app;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::test]
async fn confirmation_without_token_are_rejected_with_400() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let response = reqwest::get(format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
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

    let response = reqwest::get(confirmation_links.text).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_confirmation_link_confirms_the_subscriber() {
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

    reqwest::get(confirmation_links.text).await.unwrap();

    let saved = sqlx::query!("SELECT name, email, status FROM subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to query for subscriber.");

    assert_eq!(saved.status, "confirmed");
    assert_eq!(saved.name, "Kevin Kenyon");
    assert_eq!(saved.email, "kvkenyon@gmail.com");
}
