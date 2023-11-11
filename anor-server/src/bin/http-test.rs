use anor_http::http_client;
use tracing_subscriber::{prelude::*, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "info,anor_storage=debug,anor_api=debug,anor_http=debug,anor_server=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    tracing::info!("http client/server test");

    // try to access the service by http client
    http_client::get_file_info("http://127.0.0.1:8181/LICENSE");
    http_client::get_file("http://127.0.0.1:8181/LICENSE");

    let range = 48..482;
    http_client::get_file_in_range("http://127.0.0.1:8181/LICENSE", Some(range));

    let range = 2000..2100;
    http_client::get_file_in_range("http://127.0.0.1:8181/LICENSE", Some(range));
}
