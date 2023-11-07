use super::{storage_codec::*, storage_persistence::*, storage_packet::*};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum ItemType {
    /// Custom type
    /// Client specific custom type, defined on the client side according to the associated item key
    Custom,

    /// Basic type
    Basic(BasicType),

    /// Complex type
    Complex(ComplexType),
}

/// Basic Type
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum BasicType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Char,
    String,
}

/// Complex Type
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub enum ComplexType {
    Array(BasicType),
    Set(BasicType),
    Map(BasicType, BasicType),
    Blob,
    Json,
    Xml,
    File,
    Folder,
    Path,
}

/// Storage Item
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct StorageItem {
    pub id: String,
    pub key: String,
    pub version: u64,
    pub data: Vec<u8>,
    pub item_type: ItemType,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metafields: Option<HashMap<String, String>>,

    /// `expires_on` - timestamp, defines expiry datetime
    pub expires_on: Option<u64>,
    pub persistence: StoragePersistence,

    /// defines the number of required replications in the cluster
    pub redundancy: u8,
}

impl StorageItem {
    pub fn new<T: bincode::Encode>(key: &str, obj: &T) -> Option<Self> {
        encode_to_binary(obj, StrorageCodecType::default()).map(|data| StorageItem {
            id: Uuid::new_v4().to_string(),
            key: key.to_owned(),
            version: 0,
            description: None,
            item_type: ItemType::Custom,
            persistence: StoragePersistence::Memory,
            data,
            tags: None,
            metafields: None,
            expires_on: None,
            redundancy: 0,
        })
    }

    pub fn with_type<T: bincode::Encode>(
        key: &str,
        storage_type: ItemType,
        obj: &T,
    ) -> Option<Self> {
        encode_to_binary(obj, StrorageCodecType::default()).map(|data| StorageItem {
            id: Uuid::new_v4().to_string(),
            key: key.to_owned(),
            version: 0,
            description: None,
            item_type: storage_type,
            persistence: StoragePersistence::Memory,
            data,
            tags: None,
            metafields: None,
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

    pub fn set_description(&mut self, description: &str) {
        self.description = Some(description.to_string());
    }

    pub fn add_tag(&mut self, tag: &str) {
        match self.tags.as_mut() {
            Some(tags) => {
                tags.push(tag.into());
            }
            None => {
                self.tags = Some(vec![tag.into()]);
            }
        }
    }

    pub fn add_metafield(&mut self, key: &str, value: &str) {
        match self.metafields.as_mut() {
            Some(metafields) => {
                metafields.insert(key.to_string(), value.to_string());
            }
            None => {
                let mut metafields = HashMap::new();
                metafields.insert(key.to_string(), value.to_string());
                self.metafields = Some(metafields);
            }
        };
    }
}
