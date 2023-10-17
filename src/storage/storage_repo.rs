use super::{storage_codec::*, storage_const::*, storage_item::*, storage_packet::*};
use crate::utils;
use fs2::FileExt;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, FileType},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

pub struct StorageRepo {
    storage: Storage,
    config: Arc<utils::config::Config>,
    lock: File,
    saved: bool,
}

type Storage = Arc<Mutex<StorageMap>>;
type StorageMap = HashMap<String, StorageItem>;

type StorageInfo = HashMap<String, (String, u64)>;

impl Default for StorageRepo {
    fn default() -> Self {
        Self::open()
    }
}

impl Drop for StorageRepo {
    fn drop(&mut self) {
        self.close();
    }
}

// #[allow(clippy::arc_with_non_send_sync)]
impl StorageRepo {
    pub fn open() -> Self {
        let config = utils::config::get_config();
        Self::open_with_config(config)
    }

    pub fn open_with_config(config: Arc<utils::config::Config>) -> Self {
        let mut storage_repo = Self::init(config.clone());
        if let Err(err) = storage_repo.load() {
            storage_repo.unlock();
            log::error!("{}", err);
            panic!("{}", err);
        }
        storage_repo
    }

    pub fn sync() {
        unimplemented!()
    }

    /// initializes the storage
    fn init(config: Arc<utils::config::Config>) -> StorageRepo {
        let storage_config = config.storage.as_ref().unwrap();
        let storage_path = storage_config.data_path.as_path();

        // create storage_path if not exists
        if let Err(err) = std::fs::create_dir_all(storage_path) {
            log::error!("{}", err);
            panic!("{}", err);
        };

        // try to lock the local storage for exclusive access
        // that prevents access to the stored data from other instances to ensure data consistency
        let lock_filepath = storage_path.join(FILE_STORAGE_LOCK);
        let lock = match fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_filepath)
        {
            Ok(file) => file,
            Err(err) => {
                log::error!("{}", err);
                panic!("{}", err);
            }
        };

        let mut lock_attempt_count = 50;
        while let Err(err) = lock.try_lock_exclusive() {
            if lock_attempt_count < 0 {
                let error_message = format!(
                    "Could not obtain a lock `{}` to open the local storage! Error Message: {}",
                    lock_filepath.to_string_lossy(),
                    err
                );
                log::error!("{}", error_message);
                panic!("{}", error_message);
            }
            thread::sleep(Duration::from_millis(100));
            lock_attempt_count -= 1;
        }

        StorageRepo {
            storage: Arc::new(Mutex::new(HashMap::new())),
            config,
            lock,
            saved: true,
        }
    }

    /// Loads the persisted data into storage
    pub fn load(&mut self) -> Result<(), String> {
        let mut storage = self.storage_lock();
        StorageRepo::clear(&mut storage);

        // load storage info
        match self.load_storage_info() {
            Ok(storage_info) => {
                // load items
                for (item_id, _) in storage_info.values() {
                    match self.load_item(item_id.clone()) {
                        Ok(storage_item) => {
                            // insert loaded item into storage
                            StorageRepo::insert(&mut storage, storage_item)
                        }
                        Err(err) => {
                            log::error!("{}", err);
                            return Err(err);
                        }
                    }
                }
            }
            Err(err) => {
                log::error!("{}", err);
            }
        };
        Ok(())
    }

    /// Persists the storage data
    pub fn flush(&mut self) -> Result<(), String> {
        let storage = self.storage_lock();

        // load locally persisted storage info
        let persisted_info = match self.load_storage_info() {
            Ok(objects) => Some(objects),
            Err(err) => {
                log::error!("{}", err);
                None
            }
        };

        let mut info_to_persist: StorageInfo = HashMap::new();
        for key in Self::object_keys(&storage) {
            if let Some(item) = Self::get(&storage, &key) {
                info_to_persist.insert(key, (item.id.clone(), item.version));
            }
        }

        // persist the storage info
        if let Err(err) = self.persist_storage_info(&info_to_persist) {
            log::error!("{}", err);
            return Err(err);
        }

        // create storage_blob_path if not exists
        let storage_blob_path = self.get_storage_blob_path();
        if let Err(err) = std::fs::create_dir_all(&storage_blob_path) {
            log::error!("{}", err);
            return Err(err.to_string());
        };

        // analyze existing blob files
        let item_ids: HashSet<_> = info_to_persist
            .values()
            .map(|v| v.0.to_ascii_lowercase())
            .collect();
        let mut to_remove = vec![];
        if let Ok(entries) = std::fs::read_dir(&storage_blob_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if FileType::is_file(&file_type) {
                        let filename = entry.file_name().to_string_lossy().to_ascii_lowercase();
                        if !item_ids.contains(&filename) {
                            to_remove.push(entry.path());
                        }
                    }
                }
            }
        }

        // remove blob files corresponding to removed items
        for path in to_remove {
            if let Err(err) = std::fs::remove_file(path) {
                log::error!("Could not remove unused item blob file: {}", err);
            }
        }

        for (item_key, (item_id, item_version)) in info_to_persist {
            if let Some(item) = Self::get(&storage, &item_key) {
                // check if item is replaced or updated
                let needs_persist = if let Some(prev) = &persisted_info {
                    if let Some((prev_id, prev_version)) = prev.get(&item.key) {
                        // need to check the id first as the item can be removed and a new item with the same key is created then
                        (item_id != *prev_id) || (item_version > *prev_version)
                    } else {
                        // new item needs persist
                        true
                    }
                } else {
                    // initial repo needs persist
                    true
                };

                if needs_persist {
                    if let Err(err) = self.persist_item(item) {
                        log::error!("{}", err);
                        return Err(err);
                    }
                }
            }
        }
        Ok(())
    }

    fn load_storage_info(&self) -> Result<StorageInfo, String> {
        let storage_config = self.config.storage.as_ref().unwrap();
        let storage_path = storage_config.data_path.as_path();
        let filepath = storage_path.join(FILE_STORAGE_INFO);
        decode_from_file(filepath)
    }

    fn persist_storage_info(&self, storage_info: &StorageInfo) -> Result<(), String> {
        let storage_config = self.config.storage.as_ref().unwrap();
        let storage_path = storage_config.data_path.as_path();
        let filepath = storage_path.join(FILE_STORAGE_INFO);
        encode_to_file(filepath, storage_info, StroragePacketType::StrorageInfo)
    }

    fn get_storage_blob_path(&self) -> PathBuf {
        let storage_config = self.config.storage.as_ref().unwrap();
        let storage_path = storage_config.data_path.as_path();
        storage_path.join(DIR_STORAGE_BLOB)
    }

    fn persist_item(&self, item: &StorageItem) -> Result<(), String> {
        let storage_blob_path = self.get_storage_blob_path();
        let filepath = storage_blob_path.join(&item.id);
        encode_to_file(filepath, item, StroragePacketType::StrorageItemBlob)
    }

    fn load_item(&self, item_id: String) -> Result<StorageItem, String> {
        let storage_blob_path = self.get_storage_blob_path();
        let filepath = storage_blob_path.join(item_id);
        decode_from_file(filepath)
    }

    /// Unlocks the storage
    fn unlock(&mut self) {
        if let Err(err) = self.lock.unlock() {
            log::error!("{}", err);
        }
    }

    /// Closes the storage
    pub fn close(&mut self) {
        if let Err(err) = self.flush() {
            log::error!("{}", err);
        }
        self.unlock();
    }

    /// Locks and returns a guarded access to the storage map
    pub fn storage_lock(&self) -> MutexGuard<StorageMap> {
        match self.storage.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // handle poisoned mutex
                let guard = poisoned.into_inner();
                if log::log_enabled!(log::Level::Warn) {
                    log::warn!("Mutex recovered from poisoning: {:?}", *guard);
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

    /// Returns the stored object keys
    pub fn object_keys(storage: &MutexGuard<StorageMap>) -> Vec<String> {
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
    use std::{path::PathBuf, thread, time::Duration};

    use super::*;
    use crate::storage::storage_type::*;

    const THREADS_COUNT: usize = 100;
    const MAP_ENTRIES_PER_THREAD: usize = 10;

    fn get_test_config() -> Arc<utils::config::Config> {
        // tmp dir is `/tmp` directory of the package root (anor)
        let tmp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tmp");
        let data_path = tmp_dir.join("anor");
        let storage = utils::config::Storage { data_path };
        Arc::new(utils::config::Config {
            storage: Some(storage),
            server: None,
            file_server: None,
            remote: None,
        })
    }

    #[test]
    pub fn storage_open_test() {
        let repo = StorageRepo::open_with_config(get_test_config());
        let mut storage = repo.storage_lock();

        // clean up the storage
        StorageRepo::clear(&mut storage);

        assert!(StorageRepo::object_keys(&storage).is_empty());
    }

    #[test]
    pub fn storage_insert_test() {
        let repo = StorageRepo::open_with_config(get_test_config());
        let mut storage = repo.storage_lock();

        // clean up the storage
        StorageRepo::clear(&mut storage);

        let key = "my_string1";
        let my_string = String::from("abc1");
        let storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        StorageRepo::insert(&mut storage, storage_item);

        let keys = StorageRepo::object_keys(&storage);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        // clean up the storage
        StorageRepo::clear(&mut storage);
    }

    #[test]
    pub fn storage_update_test() {
        let repo = StorageRepo::open_with_config(get_test_config());
        let mut storage = repo.storage_lock();

        // clean up the storage
        StorageRepo::clear(&mut storage);

        let key = "my_string2";
        let my_string = String::from("abc2");
        let mut storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();
        storage_item.description = Some("abc".to_string());

        StorageRepo::insert(&mut storage, storage_item);

        assert_eq!(StorageRepo::object_keys(&storage).len(), 1);
        let item = StorageRepo::get_mut(&mut storage, key).unwrap();
        assert_eq!(item.description, Some("abc".to_string()));

        item.description = Some("abcd".to_string());

        assert_eq!(
            StorageRepo::get(&storage, key).unwrap().description,
            Some("abcd".to_string())
        );

        // clean up the storage
        StorageRepo::clear(&mut storage);
    }

    #[test]
    pub fn storage_remove_test() {
        let repo = StorageRepo::open_with_config(get_test_config());
        let mut storage = repo.storage_lock();

        // clean up the storage
        StorageRepo::clear(&mut storage);

        let key = "my_string3";
        let my_string = String::from("abc3");
        let storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        StorageRepo::insert(&mut storage, storage_item);

        let keys = StorageRepo::object_keys(&storage);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        StorageRepo::remove(&mut storage, key);
        assert!(StorageRepo::object_keys(&storage).is_empty());
    }

    #[test]
    pub fn storage_clear_test() {
        let repo = StorageRepo::open_with_config(get_test_config());
        let mut storage = repo.storage_lock();

        // clean up the storage
        StorageRepo::clear(&mut storage);

        let key = "my_string4";
        let my_string = String::from("abc4");
        let storage_item =
            StorageItem::new(key, StorageType::Basic(BasicType::String), &my_string).unwrap();

        StorageRepo::insert(&mut storage, storage_item);

        let keys = StorageRepo::object_keys(&storage);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        StorageRepo::clear(&mut storage);
        assert!(StorageRepo::object_keys(&storage).is_empty());
    }

    #[test]
    pub fn storage_object_test() {
        let repo = StorageRepo::open_with_config(get_test_config());
        let mut storage = repo.storage_lock();

        // clean up the storage
        StorageRepo::clear(&mut storage);

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

        // clean up the storage
        StorageRepo::clear(&mut storage);
    }

    #[test]
    fn multithread_map_insert_test() {
        let key = "my_map";
        let repo = Arc::new(StorageRepo::open_with_config(get_test_config()));
        {
            // clean up the storage
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }

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

        {
            // clean up the storage
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }
    }

    #[test]
    fn multithread_map_get_test() {
        let key = "my_map";
        let repo = Arc::new(StorageRepo::open_with_config(get_test_config()));
        {
            // clean up the storage
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }

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
            let mut storage = repo.storage_lock();
            let map = StorageRepo::get_object::<HashMap<String, String>>(&storage, key).unwrap();
            assert_eq!(map.keys().count(), THREADS_COUNT * MAP_ENTRIES_PER_THREAD);

            // clean up the storage
            StorageRepo::clear(&mut storage);
        }
    }

    #[test]
    fn multithread_map_remove_test() {
        let key = "my_map";
        let repo = Arc::new(StorageRepo::open_with_config(get_test_config()));
        {
            // clean up the storage
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }

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
        let repo = Arc::new(StorageRepo::open_with_config(get_test_config()));
        {
            // clean up the storage
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }

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
            let object_keys = StorageRepo::object_keys(&storage);
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
            assert!(StorageRepo::object_keys(&storage).is_empty());
        }
    }

    #[test]
    fn multithread_scoped_multiobject_test() {
        let key_prefix = "my_map";
        let repo = Arc::new(StorageRepo::open_with_config(get_test_config()));
        {
            // clean up the storage
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }

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
            let object_keys = StorageRepo::object_keys(&storage);
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
            assert!(StorageRepo::object_keys(&storage).is_empty());
        }
    }

    #[test]
    fn storage_flush_load_test() {
        use std::fs;
        use std::path::Path;

        let mut repo = StorageRepo::open_with_config(get_test_config());
        {
            // clean up the storage
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }

        assert_eq!(repo.flush(), Ok(()));

        // check the storage info is empty
        let result = repo.load_storage_info();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        let storage_blob_path = repo.get_storage_blob_path();

        // check the storage blob directory exists
        assert!(Path::new(&storage_blob_path).exists());

        // check the storage blob directory is empty
        let paths = fs::read_dir(&storage_blob_path).unwrap();
        assert_eq!(paths.count(), 0);

        let key = "my_map1";
        let mut my_map1 = HashMap::<String, String>::new();
        my_map1.insert("1".into(), "One".into());
        my_map1.insert("2".into(), "Two".into());
        my_map1.insert("3".into(), "Three".into());

        // insert the map into storage
        {
            let storage_type =
                StorageType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
            let storage_item = StorageItem::new(key, storage_type, &my_map1).unwrap();
            let mut storage = repo.storage_lock();
            StorageRepo::insert(&mut storage, storage_item);
        }

        // persist the storage
        assert_eq!(repo.flush(), Ok(()));

        // check the storage info has the map
        let result = repo.load_storage_info();
        assert!(result.is_ok());

        let storage_info = result.unwrap();
        assert!(storage_info.contains_key(key));

        // check the storage blob directory exists
        assert!(Path::new(&storage_blob_path).exists());

        // check the storage blob directory has a single entry
        let paths = fs::read_dir(&storage_blob_path).unwrap();
        let entries: Vec<_> = paths.flatten().map(|v| v.file_name()).collect();
        assert_eq!(entries.len(), 1);

        // check the entry id
        let item_id = storage_info.get(key).unwrap().0.to_ascii_lowercase();
        assert_eq!(entries[0].to_string_lossy().to_ascii_lowercase(), item_id);

        // clean up the storage
        {
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);

            let object_keys = StorageRepo::object_keys(&storage);
            assert!(object_keys.is_empty());
        }

        // load storage
        assert_eq!(repo.load(), Ok(()));

        // verify loaded storage
        {
            let storage = repo.storage_lock();
            let object_keys = StorageRepo::object_keys(&storage);
            assert_eq!(object_keys.len(), 1);
            assert_eq!(object_keys[0], key);

            let map: HashMap<String, String> = StorageRepo::get_object(&storage, key).unwrap();
            assert_eq!(my_map1, map);
        }

        // clean up the storage
        {
            let mut storage = repo.storage_lock();
            StorageRepo::clear(&mut storage);
        }
    }
}
