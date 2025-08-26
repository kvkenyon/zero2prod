//! main.rs
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = telemetry::get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration file.");
    let connection_pool =
        PgPoolOptions::new().connect_lazy_with(configuration.database.connection_options());
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind port 8000.");
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Failed to parse email.");
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        std::time::Duration::from_millis(configuration.email_client.timeout_milliseconds),
    );
    run(listener, connection_pool, email_client)?.await
}
