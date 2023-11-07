use log;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use anor_storage::{Storage, StorageItem};
use anor_utils::{Config, ThreadPool};

pub trait ApiService {
    fn with_config(storage: Arc<Storage>, config: Arc<Config>) -> Self;
    fn start(
        &self,
        server_shutdown: Arc<AtomicBool>,
        signal_server_ready: Sender<()>,
    ) -> Result<(), String>;
    fn stop(&self);
    fn keys(&self) -> Vec<String>;
    fn set_item(&self, key: &str, item: StorageItem) -> bool;
    fn get_item(&self, key: &str) -> Option<StorageItem>;
    fn remove_item(&self, key: &str) -> bool;
}

pub struct Service {
    storage: Arc<Storage>,
    config: Arc<Config>,
}

pub type ApiMutex<'a> = Arc<Mutex<Service>>;

impl ApiService for Service {
    fn with_config(storage: Arc<Storage>, config: Arc<Config>) -> Self {
        Service { storage, config }
    }

    fn start(
        &self,
        server_shutdown: Arc<AtomicBool>,
        signal_ready_sender: Sender<()>,
    ) -> Result<(), String> {
        assert!(self.config.api.is_some());
        let config_server = self.config.api.as_ref().unwrap();
        assert!(!config_server.listen_on.is_empty());
        let listen_on = config_server.listen_on[0];

        let listener = TcpListener::bind(listen_on).unwrap();

        // send the ready signal
        if let Err(err) = signal_ready_sender.send(()) {
            return Err(err.to_string());
        }

        log::info!("API service listening on {} ...", listen_on);
        // listener.set_nonblocking(true).unwrap();

        let pool = ThreadPool::new(2);

        while !server_shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let shutdown_clone = server_shutdown.clone();
                    pool.execute(move || {
                        handle_connection(stream, addr, shutdown_clone);
                    });
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

        Ok(())
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

fn handle_connection(mut stream: TcpStream, addr: SocketAddr, shutdown: Arc<AtomicBool>) {
    log::debug!("Client connected: {}", addr);
    let mut buf = [0; 1024];
    let addr = stream.peer_addr().unwrap();
    while !shutdown.load(Ordering::SeqCst) {
        let count = stream.read(&mut buf).unwrap();
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("Received bytes count from {} : {}", addr, count);
        }

        let mut vec = buf.to_vec();
        vec.truncate(count);
        let msg = String::from_utf8(vec).unwrap();

        if log::log_enabled!(log::Level::Trace) {
            log::trace!("Received message from {} : {}", addr, msg);
        }
    }
}
