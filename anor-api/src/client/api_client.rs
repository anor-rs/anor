use anor_storage::storage::storage_item::StorageItem;
use anor_utils::config::Config;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::Arc;

pub trait SocketClient {
    fn with_config(config: Arc<Config>) -> Self;
    fn connect(&mut self) -> std::io::Result<()>;
    fn disconnect(&mut self) -> std::io::Result<()>;
    fn insert(&self, storage_item: StorageItem);
    fn update(&mut self, key: &str, storage_item: StorageItem) -> std::io::Result<()>;
    fn get(&mut self, key: &str) -> std::io::Result<StorageItem>;
    fn remove(&self, key: &str) -> bool;
    fn keys(&self) -> Vec<String>;
    fn clear(&self);
    fn flush(&self);
}
pub struct StorageApiClient {
    stream: Option<TcpStream>,
    config: Arc<Config>,
}

impl SocketClient for StorageApiClient {
    fn with_config(config: Arc<Config>) -> Self {
        StorageApiClient {
            stream: None,
            config,
        }
    }

    fn connect(&mut self) -> std::io::Result<()> {
        assert!(self.config.remote.is_some());
        let config_remote = self.config.remote.as_ref().unwrap();
        assert!(!config_remote.nodes.is_empty());
        let remote_address = config_remote.nodes[0];

        let stream = TcpStream::connect(remote_address)?;

        let local_addr = stream.local_addr().unwrap();
        log::info!("connected to {} as {}", remote_address, local_addr);

        stream.set_nodelay(true).expect("set_nodelay call failed");

        self.stream = Some(stream);
        Ok(())
    }

    fn disconnect(&mut self) -> std::io::Result<()> {
        let stream = self.stream.as_mut().unwrap();
        stream.flush()?;
        self.stream = None;
        Ok(())
    }

/*
    fn set_item(&mut self, key: String) -> std::io::Result<()> {
        let stream = self.stream.as_mut().unwrap();
        stream.write_all(key.as_bytes())?;
        // stream.shutdown(std::net::Shutdown::Write).unwrap();
        stream.flush().unwrap();
        Ok(())
    }

    fn get_item(&mut self, _key: String) -> std::io::Result<()> {
        let stream = self.stream.as_mut().unwrap();
        let mut buf = [0; 128];
        stream.write_all(&buf)?;
        stream.read_exact(&mut buf)?;
        Ok(())
    }
*/

    fn insert(&self, storage_item: StorageItem) {
        todo!()
    }

    fn update(&mut self, key: &str, storage_item: StorageItem) -> std::io::Result<()> {
        todo!()
    }

    fn get(&mut self, key: &str) -> std::io::Result<StorageItem> {
        todo!()
    }

    fn remove(&self, key: &str) -> bool {
        todo!()
    }

    fn keys(&self) -> Vec<String> {
        vec![]
    }

    fn clear(&self) {
        todo!()
    }

    fn flush(&self) {
        todo!()
    }
}
