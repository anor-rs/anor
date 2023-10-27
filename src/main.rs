use anor_common::utils::config;
use anor_http::client::http_client;
use core::time;
use log::{log_enabled, Level};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();

    log::info!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let config = config::load();

    // open the data storage
    // let storage = Storage::open_with_config(config.clone());

    // start file server
    let http_service_config = config.clone();

    // start the storage api service

    let http_service_ready = Arc::new(AtomicBool::new(false));
    let http_service_shutdown = Arc::new(AtomicBool::new(false));
    let handle_http_service = anor_http::service::http_service::start_http_service(
        http_service_config,
        http_service_ready.clone(),
        http_service_shutdown.clone(),
    );

    // wait until service is ready
    while !http_service_ready.load(Ordering::SeqCst) {
        if log_enabled!(Level::Trace) {
            log::trace!("HTTP service not ready yet, wait...");
        }
        thread::sleep(time::Duration::from_millis(20));
    }

    // try to access the service by http client
    http_client::get_file_info("http://127.0.0.1:8181/LICENSE");
    http_client::get_file("http://127.0.0.1:8181/LICENSE");

    let range = 48..482;
    http_client::get_file_in_range("http://127.0.0.1:8181/LICENSE", Some(range));

    let range = 2000..2100;
    http_client::get_file_in_range("http://127.0.0.1:8181/LICENSE", Some(range));

    // shutdown the HTTP service
    // http_service_shutdown.store(true, Ordering::SeqCst);

    handle_http_service.join().unwrap();
    log::info!("Anor HTTP service is shutdown successfully.");
}
