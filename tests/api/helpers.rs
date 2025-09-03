//! tests/api/helper.rs
use argon2::password_hash::SaltString;
use argon2::{Argon2, Params, PasswordHasher};
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

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        TestUser {
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            Params::new(15000, 2, 1, None).expect("Failed to build params."),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash)
VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to create test users.");
    }
}

pub struct TestApp {
    pub user: TestUser,
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16,
    api_client: reqwest::Client,
}

impl TestApp {
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(format!("{}/newsletters", self.address))
            .basic_auth(&self.user.username, Some(&self.user.password))
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

    // Create test user and store them in the database
    let user = TestUser::generate();
    user.store(&connection_pool).await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    TestApp {
        address,
        db_pool: connection_pool,
        email_server,
        port: application_port,
        user,
        api_client: client,
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

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
