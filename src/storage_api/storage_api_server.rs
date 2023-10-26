use anor::storage::Storage;
use anor_common::utils::config::Config;
use log;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use anor::storage::storage_item::StorageItem;

use super::StorageApi;

pub struct StorageApiServer {
    storage: Storage,
    config: Arc<Config>,
}

pub type AnorApiMutex<'a> = Arc<Mutex<StorageApiServer>>;

impl StorageApi for StorageApiServer {
    fn with_config(storage: Storage, config: Arc<Config>) -> Self {
        StorageApiServer { storage, config }
    }

    fn start(&self, flag_shutdown: Arc<AtomicBool>, flag_ready: Arc<AtomicBool>) {
        assert!(self.config.server.is_some());
        let config_server = self.config.server.as_ref().unwrap();
        assert!(!config_server.listen_on.is_empty());
        let listen_on = config_server.listen_on[0];

        let listener = TcpListener::bind(listen_on).unwrap();
        flag_ready.store(true, Ordering::SeqCst);

        log::info!(
            "Anor Storage API server started listening on {} ...",
            listen_on
        );
        // listener.set_nonblocking(true).unwrap();

        // todo: use thread pooling
        let mut handles = Vec::<JoinHandle<()>>::new();
        while !flag_shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let connection_shutdown = flag_shutdown.clone();
                    let handle =
                        thread::spawn(move || handle_connection(stream, addr, connection_shutdown));
                    handles.push(handle);
                }
                /*
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // wait until network socket is ready, typically implemented
                    // via platform-specific APIs such as epoll or IOCP
                    thread::sleep(time::Duration::from_millis(1));
                    continue;
                }
                */
                Err(e) => log::error!("couldn't get client: {e:?}"),
            }
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    fn stop(&self) {}

    fn keys(&self) -> Vec<String> {
        self.storage.keys()
    }

    fn set_item(&self, _key: &str, _item: StorageItem) -> bool {
        false
    }

    fn get_item(&self, _key: &str) -> Option<StorageItem> {
        None
    }

    fn remove_item(&self, key: &str) -> bool {
        self.storage.remove(key);
        true
    }
}

fn handle_connection(mut stream: TcpStream, addr: SocketAddr, flag_shutdown: Arc<AtomicBool>) {
    log::debug!("Client connected: {}", addr);
    let mut buf = [0; 1024];
    let addr = stream.peer_addr().unwrap();
    while !flag_shutdown.load(Ordering::SeqCst) {
        let count = stream.read(&mut buf).unwrap();
        log::debug!("Received bytes count from {} : {}", addr, count);
        let mut vec = buf.to_vec();
        vec.truncate(count);
        let msg = String::from_utf8(vec).unwrap();
        log::debug!("Received message from {} : {}", addr, msg);
    }
}
