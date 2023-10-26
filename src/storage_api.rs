//! The Storage API Service for [Anor](https://github.com/anor-rs)
//!
//! ## Project Stage
//!
//! **Research:** This project is at the design stage, with some sketches of work but nothing usable yet.

pub mod storage_api_service;

use anor::storage::storage_item::StorageItem;
use anor::storage::Storage;
use anor_common::utils::config::Config;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub trait StorageApi {
    fn with_config(storage: Storage, config: Arc<Config>) -> Self;
    fn start(&self, server_shutdown: Arc<AtomicBool>, server_ready: Arc<AtomicBool>);
    fn stop(&self);
    fn keys(&self) -> Vec<String>;
    fn set_item(&self, key: &str, item: StorageItem) -> bool;
    fn get_item(&self, key: &str) -> Option<StorageItem>;
    fn remove_item(&self, key: &str) -> bool;
}
