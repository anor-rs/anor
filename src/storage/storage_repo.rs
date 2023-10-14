use crate::storage::storage_item::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

pub struct StorageRepo {
    storage: Storage,
}

type Storage = Arc<Mutex<StorageMap>>;
type StorageMap = HashMap<String, StorageItem>;

impl Default for StorageRepo {
    fn default() -> Self {
        Self::new()
    }
}

// #[allow(clippy::arc_with_non_send_sync)]
impl StorageRepo {
    pub fn new() -> Self {
        StorageRepo {
            storage: Arc::new(Mutex::new(HashMap::new())),
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

    /// Locks and returns a guarded access to the storage
    pub fn storage_lock(&self) -> MutexGuard<StorageMap> {
        match self.storage.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // handle poisoned mutex
                let guard = poisoned.into_inner();
                if log::log_enabled!(log::Level::Warn) {
                    log::warn!("Mutex poisoning recovered: {:?}", *guard);
                }
                guard
            }
        }
    }

    /// Inserts an item into the storage.
    /// If the storage did have an item with the key present, the item is updated.
    pub fn insert(storage: &mut MutexGuard<StorageMap>, storage_item: StorageItem) {
        storage.insert(storage_item.key.clone(), storage_item);
    }

    /// Gets an item from the storage corresponding to the key
    pub fn get<'a>(storage: &'a MutexGuard<StorageMap>, key: &str) -> Option<&'a StorageItem> {
        storage.get(key)
    }

    /// Gets a mutable item from the storage corresponding to the key
    pub fn get_mut<'a>(
        storage: &'a mut MutexGuard<StorageMap>,
        key: &str,
    ) -> Option<&'a mut StorageItem> {
        storage.get_mut(key)
    }

    /// Removes an item from the storage
    pub fn remove(storage: &mut MutexGuard<StorageMap>, key: &str) {
        storage.remove(key);
    }

    /// Clears the storage, removing all items
    pub fn clear(storage: &mut MutexGuard<StorageMap>) {
        storage.clear();
    }

    /// Returns storage keys
    pub fn keys(storage: &MutexGuard<StorageMap>) -> Vec<String> {
        storage.keys().cloned().collect()
    }

    /// Returns an object of the item corresponding to the key
    pub fn get_object<T: bincode::Decode>(
        storage: &MutexGuard<StorageMap>,
        key: &str,
    ) -> Option<T> {
        if let Some(item) = StorageRepo::get(storage, key) {
            let object: Option<T> = item.get_object();
            return object;
        }
        None
    }

    /// Updates the object of the item corresponding to the key
    pub fn update_object<T: bincode::Encode>(
        storage: &mut MutexGuard<StorageMap>,
        key: &str,
        obj: &T,
    ) -> bool {
        if let Some(item) = StorageRepo::get_mut(storage, key) {
            item.update_object(obj);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;
    use crate::storage::storage_type::*;

    const THREADS_COUNT: usize = 100;
    const MAP_ENTRIES_PER_THREAD: usize = 10;

    #[test]
    pub fn storage_new_test() {
        let repo = StorageRepo::new();
        let storage = repo.storage_lock();

        assert!(StorageRepo::keys(&storage).is_empty());
    }

    #[test]
    pub fn storage_insert_test() {
        let repo = StorageRepo::new();
        let mut storage = repo.storage_lock();

        let key = "my_string1";
        let my_string = String::from("abc1");
        let storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        StorageRepo::insert(&mut storage, storage_item);

        let keys = StorageRepo::keys(&storage);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);
    }

    #[test]
    pub fn storage_update_test() {
        let repo = StorageRepo::new();
        let mut storage = repo.storage_lock();

        let key = "my_string2";
        let my_string = String::from("abc2");
        let mut storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();
        storage_item.description = Some("abc".to_string());

        StorageRepo::insert(&mut storage, storage_item);

        assert_eq!(StorageRepo::keys(&storage).len(), 1);
        let item = StorageRepo::get_mut(&mut storage, key).unwrap();
        assert_eq!(item.description, Some("abc".to_string()));

        item.description = Some("abcd".to_string());

        assert_eq!(
            StorageRepo::get(&storage, key).unwrap().description,
            Some("abcd".to_string())
        );
    }

    #[test]
    pub fn storage_remove_test() {
        let repo = StorageRepo::new();
        let mut storage = repo.storage_lock();

        let key = "my_string3";
        let my_string = String::from("abc3");
        let storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        StorageRepo::insert(&mut storage, storage_item);

        let keys = StorageRepo::keys(&storage);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        StorageRepo::remove(&mut storage, key);
        assert!(StorageRepo::keys(&storage).is_empty());
    }

    #[test]
    pub fn storage_clear_test() {
        let repo = StorageRepo::new();
        let mut storage = repo.storage_lock();

        let key = "my_string4";
        let my_string = String::from("abc4");
        let storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        StorageRepo::insert(&mut storage, storage_item);

        let keys = StorageRepo::keys(&storage);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        StorageRepo::clear(&mut storage);
        assert!(StorageRepo::keys(&storage).is_empty());
    }

    #[test]
    pub fn storage_object_test() {
        let repo = StorageRepo::new();
        let mut storage = repo.storage_lock();

        let key = "my_map1";

        let mut my_map1 = HashMap::<String, String>::new();
        my_map1.insert("1".into(), "One".into());
        my_map1.insert("2".into(), "Two".into());
        my_map1.insert("3".into(), "Three".into());

        let storage_type =
            StorageType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
        let storage_item = StorageItem::new(key, storage_type, &my_map1).unwrap();

        StorageRepo::insert(&mut storage, storage_item);

        let decoded_map1: HashMap<String, String> = StorageRepo::get_object(&storage, key).unwrap();
        assert_eq!(my_map1, decoded_map1);

        my_map1.insert("4".into(), "Four".into());
        assert!(StorageRepo::update_object(&mut storage, key, &my_map1));

        let decoded_map2 =
            StorageRepo::get_object::<HashMap<String, String>>(&storage, key).unwrap();
        assert_eq!(my_map1, decoded_map2);
    }

    #[test]
    fn multithread_map_insert_test() {
        let key = "my_map";
        let repo = Arc::new(StorageRepo::new());
        // create a new map and insert into storage
        {
            let my_map = HashMap::<String, String>::new();

            let storage_type =
                StorageType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
            let storage_item = StorageItem::new(key, storage_type, &my_map).unwrap();

            let mut storage = repo.storage_lock();
            StorageRepo::insert(&mut storage, storage_item);
        }

        // inserting map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let repo_cloned = repo.clone();
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut storage = repo_cloned.storage_lock();
                let mut map: HashMap<String, String> =
                    StorageRepo::get_object(&storage, key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    map.insert(entry_key, entry_value);
                }
                StorageRepo::update_object(&mut storage, key, &map);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // verify entries
        {
            let storage = repo.storage_lock();
            let map = StorageRepo::get_object::<HashMap<String, String>>(&storage, key).unwrap();
            assert_eq!(map.keys().count(), THREADS_COUNT * MAP_ENTRIES_PER_THREAD);
            for thread_number in 0..THREADS_COUNT {
                for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    assert_eq!(map.get(&entry_key).unwrap(), &entry_value);
                }
            }
        }
    }

    #[test]
    fn multithread_map_get_test() {
        let key = "my_map";
        let repo = Arc::new(StorageRepo::new());

        // create a new map and insert entries
        {
            let mut my_map = HashMap::<String, String>::new();

            for thread_number in 0..THREADS_COUNT {
                for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    my_map.insert(entry_key, entry_value);
                }
            }

            let storage_type =
                StorageType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
            let storage_item = StorageItem::new(key, storage_type, &my_map).unwrap();

            let mut storage = repo.storage_lock();
            StorageRepo::insert(&mut storage, storage_item);
        }

        // get map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let repo_cloned = repo.clone();
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut storage = repo_cloned.storage_lock();
                let map: HashMap<String, String> = StorageRepo::get_object(&storage, key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    assert_eq!(map.get(&entry_key).unwrap(), &entry_value);
                }
                StorageRepo::update_object(&mut storage, key, &map);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // check entries count
        {
            let storage = repo.storage_lock();
            let map = StorageRepo::get_object::<HashMap<String, String>>(&storage, key).unwrap();
            assert_eq!(map.keys().count(), THREADS_COUNT * MAP_ENTRIES_PER_THREAD);
        }
    }

    #[test]
    fn multithread_map_remove_test() {
        let key = "my_map";
        let repo = Arc::new(StorageRepo::new());

        // create a new map and insert entries
        {
            let mut my_map = HashMap::<String, String>::new();

            for thread_number in 0..THREADS_COUNT {
                for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    my_map.insert(entry_key, entry_value);
                }
            }

            let storage_type =
                StorageType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
            let storage_item = StorageItem::new(key, storage_type, &my_map).unwrap();

            let mut storage = repo.storage_lock();
            StorageRepo::insert(&mut storage, storage_item);
        }

        // verify and remove map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let repo_cloned = repo.clone();
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut storage = repo_cloned.storage_lock();
                let mut map: HashMap<String, String> =
                    StorageRepo::get_object(&storage, key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    assert_eq!(map.remove(&entry_key).unwrap(), entry_value);
                }
                StorageRepo::update_object(&mut storage, key, &map);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // ensure the map is empty
        {
            let storage = repo.storage_lock();
            let map = StorageRepo::get_object::<HashMap<String, String>>(&storage, key).unwrap();
            assert!(map.is_empty());
        }
    }

    #[test]
    fn multithread_multiobject_test() {
        let key_prefix = "my_map";
        let repo = Arc::new(StorageRepo::new());

        // creating and inserting map objects in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let repo_cloned = repo.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let handler = thread::spawn(move || {
                let mut storage = repo_cloned.storage_lock();

                let map = HashMap::<String, String>::new();
                let storage_type =
                    StorageType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
                let storage_item = StorageItem::new(&object_key, storage_type, &map).unwrap();

                StorageRepo::insert(&mut storage, storage_item);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // verify inserted objects
        {
            let storage = repo.storage_lock();
            let object_keys = StorageRepo::keys(&storage);
            assert_eq!(object_keys.len(), THREADS_COUNT);
            for thread_number in 0..THREADS_COUNT {
                let object_key = format!("{}-{}", key_prefix, thread_number);
                assert!(object_keys.contains(&object_key));
            }
        }

        // inserting map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let repo_cloned = repo.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut storage = repo_cloned.storage_lock();

                let mut map: HashMap<String, String> =
                    StorageRepo::get_object(&storage, &object_key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    map.insert(entry_key, entry_value);
                }
                StorageRepo::update_object(&mut storage, &object_key, &map);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // verify and remove map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let repo_cloned = repo.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut storage = repo_cloned.storage_lock();
                let mut map: HashMap<String, String> =
                    StorageRepo::get_object(&storage, &object_key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    assert_eq!(map.remove(&entry_key).unwrap(), entry_value);
                }
                StorageRepo::update_object(&mut storage, &object_key, &map);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // verify and remove storage items in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let repo_cloned = repo.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let handler = thread::spawn(move || {
                let mut storage = repo_cloned.storage_lock();
                let map: HashMap<String, String> =
                    StorageRepo::get_object(&storage, &object_key).unwrap();
                assert!(map.is_empty());

                // remove storage object
                StorageRepo::remove(&mut storage, &object_key);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // ensure empty storage
        {
            let storage = repo.storage_lock();
            assert!(StorageRepo::keys(&storage).is_empty());
        }
    }

    #[test]
    fn multithread_scoped_multiobject_test() {
        let key_prefix = "my_map";
        let repo = Arc::new(StorageRepo::new());

        // create and insert map objects into storage in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let repo_cloned = repo.clone();
                scope.spawn(move || {
                    let mut storage = repo_cloned.storage_lock();

                    let map = HashMap::<String, String>::new();
                    let storage_type = StorageType::Complex(ComplexType::Map(
                        BasicType::String,
                        BasicType::String,
                    ));

                    let object_key = format!("{}-{}", key_prefix, thread_number);
                    let storage_item = StorageItem::new(&object_key, storage_type, &map).unwrap();

                    StorageRepo::insert(&mut storage, storage_item);
                });
            }
        });

        // verify inserted objects
        {
            let storage = repo.storage_lock();
            let object_keys = StorageRepo::keys(&storage);
            assert_eq!(object_keys.len(), THREADS_COUNT);
            for thread_number in 0..THREADS_COUNT {
                let object_key = format!("{}-{}", key_prefix, thread_number);
                assert!(object_keys.contains(&object_key));
            }
        }

        // inserting map entires in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let repo_cloned = repo.clone();
                scope.spawn(move || {
                    let mut storage = repo_cloned.storage_lock();
                    let object_key = format!("{}-{}", key_prefix, thread_number);

                    let mut map: HashMap<String, String> =
                        StorageRepo::get_object(&storage, &object_key).unwrap();

                    for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                        let entry_key = format!("{}-{}", thread_number, entry_number);
                        let entry_value = format!("{}", thread_number * entry_number);
                        map.insert(entry_key, entry_value);
                    }

                    StorageRepo::update_object(&mut storage, &object_key, &map);
                });
            }
        });

        // verify and remove map entires in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let repo_cloned = repo.clone();
                scope.spawn(move || {
                    let mut storage = repo_cloned.storage_lock();
                    let object_key = format!("{}-{}", key_prefix, thread_number);
                    let mut map: HashMap<String, String> =
                        StorageRepo::get_object(&storage, &object_key).unwrap();

                    for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                        let entry_key = format!("{}-{}", thread_number, entry_number);
                        let entry_value = format!("{}", thread_number * entry_number);
                        assert_eq!(map.remove(&entry_key).unwrap(), entry_value);
                    }

                    StorageRepo::update_object(&mut storage, &object_key, &map);
                });
            }
        });

        // verify and remove storage items in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let repo_cloned = repo.clone();
                scope.spawn(move || {
                    let mut storage = repo_cloned.storage_lock();
                    let object_key = format!("{}-{}", key_prefix, thread_number);
                    let map: HashMap<String, String> =
                        StorageRepo::get_object(&storage, &object_key).unwrap();
                    assert!(map.is_empty());

                    // remove storage object
                    StorageRepo::remove(&mut storage, &object_key);
                });
            }
        });

        // ensure empty storage
        {
            let storage = repo.storage_lock();
            assert!(StorageRepo::keys(&storage).is_empty());
        }
    }
}
