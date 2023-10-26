use anor::storage::Storage;
use anor_api::storage_api::{storage_api_server::StorageApiServer, StorageApi};
use anor_common::utils::config;
use core::time;
use log::{log_enabled, Level};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();

    log::info!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let config = config::get_config();

    // open the data storage
    let storage = Storage::open_with_config(config.clone());

    // start the storage api
    let storage_api_ready = Arc::new(AtomicBool::new(false));
    let storage_api_shutdown = Arc::new(AtomicBool::new(false));

    let storage_api_ready_clone = storage_api_ready.clone();
    let storage_api_shutdown_clone = storage_api_shutdown.clone();
    let handle_tcp_server = thread::spawn(move || {
        let server = StorageApiServer::with_config(storage, config);
        server.start(storage_api_shutdown_clone, storage_api_ready_clone);
    });

    while !storage_api_ready.load(Ordering::SeqCst) {
        if log_enabled!(Level::Trace) {
            log::trace!("Anor Storage API server not ready yet, wait...");
        }
        thread::sleep(time::Duration::from_millis(20));
    }

    // shutdown the server
    // server_shutdown.store(false, Ordering::SeqCst);

    handle_tcp_server.join().unwrap();
    log::info!("Anor Storage API server is shutdown successfully.");
}
