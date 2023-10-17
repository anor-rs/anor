use super::{storage_codec::*, storage_location::*, storage_packet::*, storage_type::*};
use std::collections::HashMap;
use uuid::Uuid;

/// Storage Item
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct StorageItem {
    pub id: String,
    pub key: String,
    pub version: u64,
    pub description: Option<String>,
    pub storage_type: StorageType,
    pub data: Vec<u8>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
    pub may_expire: bool,
    pub expires_on: Option<String>,
    pub storage_locations: Vec<StorageLocation>,

    /// defines the number of required replications in the cluster
    pub redundancy: u8,
}

impl StorageItem {
    pub fn new<T: bincode::Encode>(key: &str, storage_type: StorageType, obj: &T) -> Option<Self> {
        encode_to_binary(obj, StrorageCodecType::default()).map(|data| StorageItem {
            id: Uuid::new_v4().to_string(),
            key: key.to_owned(),
            version: 0,
            description: None,
            storage_type,
            storage_locations: vec![StorageLocation::Memory],
            data,
            tags: None,
            metadata: None,
            may_expire: false,
            expires_on: None,
            redundancy: 0,
        })
    }

    pub fn update_object<T: bincode::Encode>(&mut self, obj: &T) -> bool {
        if let Some(encoded) = encode_to_binary(obj, StrorageCodecType::default()) {
            self.data = encoded;
            return true;
        }
        false
    }

    pub fn get_object<T: bincode::Decode>(&self) -> Option<T> {
        decode_from_binary(&self.data, StrorageCodecType::default())
    }
}
