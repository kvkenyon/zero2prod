//! tests/api/helper.rs

use reqwest::Url;
use secrecy::Secret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::startup::get_connection_pool;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use zero2prod::{
    configuration::{DatabaseSettings, get_configuration},
    startup::Application,
};

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("test".into(), "debug".into(), std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct ConfirmationLinks {
    pub html: Url,
    pub text: Url,
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/newsletters", self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut link = Url::parse(&raw_link).unwrap();
            assert_eq!(link.host_str().unwrap(), "127.0.0.1");
            link.set_port(Some(self.port)).unwrap();
            link
        };

        ConfirmationLinks {
            html: get_link(body["HtmlBody"].as_str().unwrap()),
            text: get_link(body["TextBody"].as_str().unwrap()),
        }
    }
}

#[allow(clippy::let_underscore_future)]
pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);
    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    let app = Application::build(&configuration)
        .await
        .expect("Failed to build application server.");

    let application_port = app.port();

    let connection_pool = get_connection_pool(&configuration.database).await;
    configure_database(&configuration.database, &connection_pool).await;

    let address = format!("http://127.0.0.1:{}", app.port());
    let _ = tokio::spawn(app.run_until_stopped());

    TestApp {
        address,
        db_pool: connection_pool,
        email_server,
        port: application_port,
    }
}

pub async fn configure_database(config: &DatabaseSettings, connection_pool: &PgPool) {
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: Secret::new("password".to_string()),
        ..config.clone()
    };
    let mut connection = PgConnection::connect_with(&maintenance_settings.connection_options())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    sqlx::migrate!("./migrations")
        .run(connection_pool)
        .await
        .expect("Failed to connect to Postgres");
}
