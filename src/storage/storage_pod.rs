use crate::storage::storage_item::*;
use std::collections::HashMap;

pub struct StoragePod {
    map: HashMap<String, StorageItem>,
}

impl Default for StoragePod {
    fn default() -> Self {
        Self::new()
    }
}

impl StoragePod {
    pub fn new() -> Self {
        StoragePod {
            map: HashMap::new(),
        }
    }

    pub fn sync() {
        unimplemented!()
    }

    pub fn load() {
        unimplemented!()
    }

    pub fn flush() {
        unimplemented!()
    }

    pub fn close() {
        unimplemented!()
    }

    /// Inserts an item into the storage.
    /// If the storage did have an item with the key present, the item is updated.
    pub fn insert(&mut self, storage_item: StorageItem) {
        self.map.insert(storage_item.key.clone(), storage_item);
    }

    /// Returns a reference to the item corresponding to the key.
    pub fn get(&self, key: &str) -> Option<&StorageItem> {
        self.map.get(key)
    }

    /// Returns a mutable reference to the item corresponding to the key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut StorageItem> {
        self.map.get_mut(key)
    }

    /// Removes an item from the storage.
    pub fn remove(&mut self, key: &str) {
        self.map.remove(key);
    }

    /// Clears the storage, removing all items.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns storage keys
    pub fn keys(&self) -> Vec<String> {
        self.map.keys().cloned().collect()
    }

    /// Returns an object of the item corresponding to the key.
    pub fn get_object <T: bincode::Decode> (&self, key: &str) -> Option<T> {
        if let Some(item) = self.map.get(key) {
            let object: Option<T> = item.get_object();
            return object;
        }
        None
    }

    /// Updates the object of the item corresponding to the key
    pub fn update_object<T: bincode::Encode>(&self, key: &str, obj: &T) -> bool {
        if let Some(item) = self.map.get(key) {
            item.update_object(obj);
            return true;
        } 
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::storage_type::*;

    #[test]
    pub fn storage_new_test() {
        let storage = StoragePod::new();
        assert!(storage.keys().is_empty());
    }

    #[test]
    pub fn storage_insert_test() {
        let key = "my_string1";
        let my_string = String::from("abc1");
        let storage_item = StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        let mut storage = StoragePod::new();
        storage.insert(storage_item);

        let keys = storage.keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);
    }

    #[test]
    pub fn storage_update_test() {
        let key = "my_string2";
        let my_string = String::from("abc2");
        let mut storage_item = StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();
        storage_item.description = Some("abc".to_string());

        let mut storage = StoragePod::new();
        storage.insert(storage_item);

        assert_eq!(storage.keys().len(), 1);
        let mut item = storage.get(key).unwrap().clone();
        assert_eq!(item.description, Some("abc".to_string()));

        item.description = Some("abcd".to_string());
        storage.insert(item);

        assert_eq!(
            storage.get(key).unwrap().description,
            Some("abcd".to_string())
        );
    }

    #[test]
    pub fn storage_remove_test() {
        let key = "my_string3";
        let my_string = String::from("abc3");
        let storage_item = StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        let mut storage = StoragePod::new();
        storage.insert(storage_item);

        let keys = storage.keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        storage.remove(key);
        assert!(storage.keys().is_empty());
    }

    #[test]
    pub fn storage_clear_test() {
        let key = "my_string4";
        let my_string = String::from("abc4");
        let storage_item = StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        let mut storage = StoragePod::new();
        storage.insert(storage_item);

        let keys = storage.keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        storage.clear();
        assert!(storage.keys().is_empty());
    }

    #[test]
    pub fn storage_object_test() {
        let key = "my_map1";

        let mut my_map1 = HashMap::<String, String>::new();
        my_map1.insert("1".into(), "One".into());
        my_map1.insert("2".into(), "Two".into());
        my_map1.insert("3".into(), "Three".into());

        let storage_type =
            StorageType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
        let storage_item = StorageItem::new(key, storage_type, &my_map1).unwrap();

        let mut storage = StoragePod::new();
        storage.insert(storage_item);

        let decoded_map1: HashMap<String, String> = storage.get_object(key).unwrap();
        assert_eq!(my_map1, decoded_map1);

        my_map1.insert("4".into(), "Four".into());
        assert!(storage.update_object(key, &my_map1));

        let decoded_map2 = storage.get_object::<HashMap<String, String>>(key).unwrap();
        assert_eq!(my_map1, decoded_map2);
    }
}
