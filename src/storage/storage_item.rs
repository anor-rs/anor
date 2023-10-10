use super::{storage_location::StorageLocation, storage_type::StorageType};
use std::{cell::RefCell, collections::HashMap};

/// Storage Item
#[derive(Debug, Clone)]
pub struct StorageItem {
    pub key: String,
    pub description: Option<String>,
    pub storage_type: StorageType,
    pub data: RefCell<Vec<u8>>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
    pub may_expire: bool,
    pub expires_on: Option<std::time::Instant>,
    pub storage_locations: Vec<StorageLocation>,

    /// defines the number of required replications in the cluster
    pub redundancy: u8,
}

impl StorageItem {
    pub fn new<T: bincode::Encode>(key: &str, storage_type: StorageType, obj: &T) -> Option<Self> {
        Self::binary_encode(obj).map(|data| StorageItem {
                key: key.to_owned(),
                description: None,
                storage_type,
                storage_locations: vec![StorageLocation::Memory],
                data: RefCell::new(data),
                tags: None,
                metadata: None,
                may_expire: false,
                expires_on: None,
                redundancy: 0,
            })
    }

    fn binary_encode<T: bincode::Encode>(obj: &T) -> Option<Vec<u8>> {
        let bincode_config = bincode::config::standard();
        match bincode::encode_to_vec(obj, bincode_config) {
            Ok(arr) => Some(arr),
            Err(msg) => {
                log::error!("Object to Binary encode error: {}", msg.to_string());
                None
            }
        }
    }

    pub fn update_object<T: bincode::Encode>(&self, obj: &T) -> bool {
        if let Some(encoded) = Self::binary_encode(obj) {
            *self.data.borrow_mut() = encoded;
            return true;
        }
        false
    }

    pub fn get_object<T: bincode::Decode>(&self) -> Option<T> {
        let bincode_config = bincode::config::standard();
        match bincode::decode_from_slice(&self.data.borrow(), bincode_config) {
            Ok(r) => {
                let (decoded, _len): (T, usize) = r;
                Some(decoded)
            },
            Err(msg) => {
                log::error!("Binary to Object decode error: {}", msg.to_string());
                None
            }
        }
    }
}
