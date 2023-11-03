use std::sync::atomic::Ordering;
use std::thread;
use std::{
    sync::{atomic::AtomicBool, mpsc::channel, Arc},
    time::Instant,
};

use anor::storage::Storage;
use anor_api::{
    client::api_client::{SocketClient, StorageApiClient},
    service::api_service::{ApiService, StorageApi},
};

use anor_common::config;

fn main() {
    let launch_start = Instant::now();

    log4rs::init_file("log.yaml", Default::default()).unwrap();

    log::info!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let config = config::load();

    // open the data storage
    let storage = Storage::open_with_config(config.clone());

    // prapare service parameters
    let api_service_config = config.clone();
    let api_service_shutdown = Arc::new(AtomicBool::new(false));
    let api_service_shutdown_clone = api_service_shutdown.clone();
    let (api_service_ready_sender, api_service_ready_receiver) = channel();

    // start the storage api service in a separate thread
    let api_service_handler = thread::spawn(move || {
        let service = StorageApi::with_config(storage, api_service_config);
        if let Err(err) = service.start(api_service_shutdown_clone, api_service_ready_sender) {
            log::error!("{}", err);
            panic!("{}", err);
        }
    });

    // wait for the readiness of api service
    if let Err(err) = api_service_ready_receiver.recv() {
        log::error!("{}", err);
        panic!("{}", err);
    }

    let launch_elapsed = Instant::elapsed(&launch_start);
    log::info!("Anor Storage API service started in {:?}", launch_elapsed);

    let mut api_client1 = StorageApiClient::with_config(config.clone());
    api_client1.connect().expect("client connection error");

    let keys = api_client1.keys();
    log::debug!("{:?}", keys);

    /*
    let msg1 = String::from("Hi there1!");
    client1.set_item(msg1).expect("set item error");
    thread::sleep(Duration::from_millis(20));
    let msg2 = String::from("Hi there2!");
    client1.set_item(msg2).expect("set item error");

    let mut client2 = StorageApiClient::with_config(config.clone());
    client2.connect().expect("client connection error");

    let msg1 = String::from("Hi there1!");
    client2.set_item(msg1).expect("set item error");
    thread::sleep(Duration::from_millis(20));
    let msg2 = String::from("Hi there2!");
    client2.set_item(msg2).expect("set item error");
    */

    // shutdown the api server
    api_service_shutdown.store(true, Ordering::SeqCst);

    // make an empty connection to unblock listener and shutdown the api server
    let mut api_client_terminate = StorageApiClient::with_config(config.clone());
    api_client_terminate
        .connect()
        .expect("client connection error");

    api_service_handler.join().unwrap();

    log::info!("Anor Storage API service is shutdown successfully.");
}
