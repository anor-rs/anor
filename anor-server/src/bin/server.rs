use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::{
    sync::{atomic::AtomicBool, mpsc::channel, Arc},
    time::Instant,
};

use tokio::signal::unix::{signal, SignalKind};

use tracing_subscriber::{prelude::*, util::SubscriberInitExt};

use anor_api::{client::api_client, ApiService, SocketClient};
use anor_http::{http_client, http_service};
use anor_storage::Storage;
use anor_utils::config::{self, Config};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "info,anor_storage=debug,anor_api=debug,anor_http=debug,anor_server=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    // load the configuration
    let config = config::load();

    // open the data storage
    let storage = Storage::open_with_config(config.clone());
    let arc_storage = Arc::new(storage);

    let server_shutdown = Arc::new(AtomicBool::new(false));

    // starting API service
    let api_service = if config.api.is_some() && config.api.as_ref().unwrap().enabled {
        Some(start_api_service(
            config.clone(),
            arc_storage.clone(),
            server_shutdown.clone(),
        ))
    } else {
        None
    };

    // starting HTTP service
    let http_service = if config.http.is_some() && config.http.as_ref().unwrap().enabled {
        Some(start_http_service(
            config.clone(),
            arc_storage.clone(),
            server_shutdown.clone(),
        ))
    } else {
        None
    };

    // hook for graceful shutdown
    let config_cloned = config.clone();
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = sigint.recv() => {
                tracing::debug!("Recieved SIGINT");
            }
            _ = sigterm.recv() => {
                tracing::debug!("Recieved SIGTERM");
            },
        };
        graceful_shutdown(server_shutdown, config_cloned).await;
    });

    if let Some(service) = api_service {
        service.join().unwrap();
        tracing::info!("API service closed.");
    }

    if let Some(service) = http_service {
        service.join().unwrap();
        tracing::info!("HTTP service closed.");
    }

    tracing::info!("Anor Server shutdown successfully.");
}

fn start_api_service(
    config: Arc<Config>,
    storage: Arc<Storage>,
    server_shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    let launch_start = Instant::now();
    tracing::info!("Starting API service...");

    // prapare service parameters
    let (api_service_ready_sender, api_service_ready_receiver) = channel();

    // start the storage api service in a separate thread
    let api_service_handler = thread::spawn(move || {
        let api_service = anor_api::Service::with_config(storage, config);
        if let Err(err) = api_service.start(server_shutdown, api_service_ready_sender) {
            tracing::error!("{}", err);
            panic!("{}", err);
        }
    });

    // wait for the readiness of api service
    if let Err(err) = api_service_ready_receiver.recv() {
        tracing::error!("{}", err);
        panic!("{}", err);
    }

    let launch_elapsed = Instant::elapsed(&launch_start);
    tracing::info!("API service started in {:?}", launch_elapsed);

    api_service_handler
}

fn start_http_service(
    config: Arc<Config>,
    storage: Arc<Storage>,
    server_shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    tracing::info!("Starting HTTP service...");
    let launch_start = Instant::now();

    // prepare http service
    let (http_service_ready_sender, http_service_ready_receiver) = channel();

    let http_service = http_service::Service::with_config(storage, config);

    // start the http service
    let handle_http_service = http_service.start(http_service_ready_sender, server_shutdown);

    // wait for the readiness of http service
    if let Err(err) = http_service_ready_receiver.recv() {
        tracing::error!("{}", err);
        panic!("{}", err);
    }

    let launch_elapsed = Instant::elapsed(&launch_start);
    tracing::info!("HTTP service started in {:?}", launch_elapsed);

    handle_http_service
}

async fn graceful_shutdown(server_shutdown: Arc<AtomicBool>, config: Arc<Config>) {
    tracing::info!("Initializing the graceful shutdown process...");
    server_shutdown.store(true, Ordering::SeqCst);

    // a temporary solution to unblock socket listener
    // make an empty connection to unblock listener and shutdown the api server
    if config.api.is_some() && config.api.as_ref().unwrap().enabled {
        let mut api_client_terminate = api_client::Client::with_config(config.clone());
        api_client_terminate
            .connect()
            .expect("client connection error");
        _ = api_client_terminate.disconnect();
    }

    if config.http.is_some() && config.http.as_ref().unwrap().enabled {
        let url = http_client::parse_url_to_uri("http://127.0.0.1:8181/LICENSE");
        _ = http_client::request_url("HEAD", url, None).await;
    }
}
