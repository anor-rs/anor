use anor::storage::Storage;
use anor_api::client::api_client::{ApiClient, TcpClient};
use anor_api::service::{api_service::ApiService, storage_api::AnorStorageApi};
use anor_common::utils::config;
use core::time;
use log::{log_enabled, Level};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();

    log::info!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let config = config::load();

    // open the data storage
    let storage = Storage::open_with_config(config.clone());

    // start the storage api service
    let api_service_ready = Arc::new(AtomicBool::new(false));
    let api_service_shutdown = Arc::new(AtomicBool::new(false));

    let api_service_ready_clone = api_service_ready.clone();
    let api_service_shutdown_clone = api_service_shutdown.clone();
    let config_clone = config.clone();
    let handle_socket_service = thread::spawn(move || {
        let service = ApiService::with_config(storage, config_clone);
        service.start(api_service_shutdown_clone, api_service_ready_clone);
    });

    while !api_service_ready.load(Ordering::SeqCst) {
        if log_enabled!(Level::Trace) {
            log::trace!("Anor Storage API service not ready yet, wait...");
        }
        thread::sleep(time::Duration::from_millis(20));
    }

    let mut client1 = ApiClient::with_config(config.clone());
    client1.connect().expect("client connection error");

    let msg1 = String::from("Hi there1!");
    client1.set_item(msg1).expect("set item error");
    thread::sleep(Duration::from_millis(20));
    let msg2 = String::from("Hi there2!");
    client1.set_item(msg2).expect("set item error");

    let mut client2 = ApiClient::with_config(config.clone());
    client2.connect().expect("client connection error");

    let msg1 = String::from("Hi there1!");
    client2.set_item(msg1).expect("set item error");
    thread::sleep(Duration::from_millis(20));
    let msg2 = String::from("Hi there2!");
    client2.set_item(msg2).expect("set item error");

    // shutdown the server
    // server_shutdown.store(false, Ordering::SeqCst);

    handle_socket_service.join().unwrap();
    log::info!("Anor Storage API service is shutdown successfully.");
}
