use anor_utils::config::{self, Config};
use fs2::FileExt;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, FileType},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, RwLock},
    thread::{self, ThreadId},
    time::Duration,
};

pub mod storage_codec;
pub mod storage_const;
pub mod storage_item;
pub mod storage_persistence;
pub mod storage_packet;

use storage_codec::*;
use storage_const::*;
use storage_item::*;
use storage_packet::*;

macro_rules! take_guard {
    ($g:expr) => {
        match $g {
            Ok(guard) => guard,
            Err(_) => {
                // poisoned, log and terminate
                let err = format!("{} is poisoned", stringify!($g));
                tracing::error!("{}", err);
                panic!("{}", err);
                /*
                let guard = poisoned.into_inner();
                tracing::warn!(
                    "{} recovered from poisoning: {:?}",
                    stringify!($g),
                    *guard
                );
                guard
                */
            }
        }
    };
}

pub struct Storage {
    storage_map: Arc<Mutex<StorageMap>>,
    config: Arc<Config>,
    instance_lock: File,
    global_lock: Mutex<()>,
    global_lock_param: RwLock<Option<ThreadId>>,
    method_lock_sync: Mutex<()>,
    // saved: bool,
}

type StorageMap = HashMap<String, StorageItem>;
type StorageInfo = HashMap<String, (String, u64)>;

pub struct GlobalLock<'a> {
    storage: &'a Storage,
    guard: Option<MutexGuard<'a, ()>>,
}

impl Drop for GlobalLock<'_> {
    fn drop(&mut self) {
        self.unlock();
    }
}
impl GlobalLock<'_> {
    /// Returns an exclusive access to the storage operations
    pub fn lock(storage: &Storage) -> GlobalLock {
        let guard = take_guard!(storage.global_lock.lock());
        Self::set_global_lock_param(storage, Some(thread::current().id()));
        GlobalLock {
            storage,
            guard: Some(guard),
        }
    }

    /// Unlocks the exclusive access to the storage
    pub fn unlock(&mut self) {
        Self::set_global_lock_param(self.storage, None);
        self.guard = None;
    }

    fn set_global_lock_param(storage: &Storage, option: Option<ThreadId>) {
        let mut guard = take_guard!(storage.global_lock_param.write());
        *guard = option;
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::open()
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        self.close();
    }
}

// #[allow(clippy::arc_with_non_send_sync)]
impl Storage {
    /// Opens a storage and loads persisted data
    pub fn open() -> Self {
        let config = config::load();
        Self::open_with_config(config)
    }

    /// Opens a storage with specified configuration and loads persisted data
    pub fn open_with_config(config: Arc<Config>) -> Self {
        let mut storage = Self::init(config.clone());
        if let Err(err) = storage.load() {
            storage.unlock();
            tracing::error!("{}", err);
            panic!("{}", err);
        }
        storage
    }

    pub fn sync() {
        unimplemented!()
    }

    /// initialize the storage
    fn init(config: Arc<Config>) -> Storage {
        let storage_config = config.storage.as_ref().unwrap();
        let storage_path = storage_config.data_path.as_path();

        // create storage_path if not exists
        if let Err(err) = std::fs::create_dir_all(storage_path) {
            tracing::error!("{}", err);
            panic!("{}", err);
        };

        // try to lock the local storage for exclusive access
        // that prevents access to the stored data from other instances to ensure data consistency
        let lock_filepath = storage_path.join(FILE_STORAGE_LOCK);
        let instance_lock = match fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_filepath)
        {
            Ok(file) => file,
            Err(err) => {
                tracing::error!("{}", err);
                panic!("{}", err);
            }
        };

        let mut lock_try_count = 100;
        let lock_try_duration =
            Duration::from_millis((INSTANCE_LOCK_TIMEOUT_MILLISECONDS / lock_try_count) as u64);

        while let Err(err) = instance_lock.try_lock_exclusive() {
            if lock_try_count == 0 {
                let error_message = format!(
                    "Could not obtain a lock `{}` to open the local storage! Error Message: {}",
                    lock_filepath.to_string_lossy(),
                    err
                );
                tracing::error!("{}", error_message);
                panic!("{}", error_message);
            }
            thread::sleep(lock_try_duration);
            lock_try_count -= 1;
        }

        Storage {
            storage_map: Arc::new(Mutex::new(HashMap::new())),
            config,
            instance_lock,
            global_lock: Mutex::new(()),
            global_lock_param: RwLock::new(None),
            method_lock_sync: Mutex::new(()),
            // saved: true,
        }
    }

    /// Loads persisted data into storage
    pub fn load(&mut self) -> Result<(), String> {
        let mut global_lock = self.global_lock();
        self.clear();

        // load storage info
        match self.load_storage_info() {
            Ok(storage_info) => {
                // load items
                for (item_id, _) in storage_info.values() {
                    match self.load_item(item_id.clone()) {
                        Ok(storage_item) => {
                            // insert loaded item into storage
                            self.insert(storage_item)
                        }
                        Err(err) => {
                            tracing::error!("{}", err);
                            return Err(err);
                        }
                    }
                }
            }
            Err(err) => {
                tracing::error!("{}", err);
            }
        };
        global_lock.unlock();
        Ok(())
    }

    /// Persists storage data
    pub fn flush(&mut self) -> Result<(), String> {
        let mut global_lock = self.global_lock();

        // load locally persisted storage info
        let persisted_info = match self.load_storage_info() {
            Ok(objects) => Some(objects),
            Err(err) => {
                tracing::error!("{}", err);
                None
            }
        };

        let mut info_to_persist: StorageInfo = HashMap::new();
        for key in self.keys() {
            if let Some(item) = self.get(&key.clone()) {
                info_to_persist.insert(key, (item.id.clone(), item.version));
            }
        }

        // persist the storage info
        if let Err(err) = self.persist_storage_info(&info_to_persist) {
            tracing::error!("{}", err);
            return Err(err);
        }

        // create storage_data_path if not exists
        let storage_data_path = self.get_storage_data_path();
        if let Err(err) = std::fs::create_dir_all(&storage_data_path) {
            tracing::error!("{}", err);
            return Err(err.to_string());
        };

        // analyze existing blob files
        let item_ids: HashSet<_> = info_to_persist
            .values()
            .map(|v| v.0.to_ascii_lowercase())
            .collect();
        let mut to_remove = vec![];
        if let Ok(entries) = std::fs::read_dir(&storage_data_path) {
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
                tracing::error!("Could not remove unused item blob file: {}", err);
            }
        }

        for (item_key, (item_id, item_version)) in info_to_persist {
            if let Some(item) = self.get(&item_key) {
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
                    // initial storage needs persist
                    true
                };

                if needs_persist {
                    if let Err(err) = self.persist_item(&item) {
                        tracing::error!("{}", err);
                        return Err(err);
                    }
                }
            }
        }
        global_lock.unlock();
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

    fn get_storage_data_path(&self) -> PathBuf {
        let storage_config = self.config.storage.as_ref().unwrap();
        let storage_path = storage_config.data_path.as_path();
        storage_path.join(DIR_STORAGE_DATA)
    }

    fn persist_item(&self, item: &StorageItem) -> Result<(), String> {
        let storage_data_path = self.get_storage_data_path();
        let filepath = storage_data_path.join(&item.id);
        encode_to_file(filepath, item, StroragePacketType::StrorageItem)
    }

    fn load_item(&self, item_id: String) -> Result<StorageItem, String> {
        let storage_data_path = self.get_storage_data_path();
        let filepath = storage_data_path.join(item_id);
        decode_from_file(filepath)
    }

    /// Unlocks the storage
    fn unlock(&mut self) {
        if let Err(err) = self.instance_lock.unlock() {
            tracing::error!("{}", err);
        }
    }

    /// Closes the storage
    fn close(&mut self) {
        if let Err(err) = self.flush() {
            tracing::error!("{}", err);
        }
        self.unlock();
    }

    /// Returns a guarded lock to access to the storage operations
    pub fn lock(&self) -> MutexGuard<StorageMap> {
        // this method needs synchronization as is has a critical execution point not covered by other locks
        let guard_method_lock = take_guard!(self.method_lock_sync.lock());

        // when global lock used, only the thread that owns the global lock should have access to storage operations
        // other threads need to wait until global lock released
        // (1) making the decision about the need of a global lock
        let wait_for_global_lock_release = {
            // RwLockReadGuard needs to drop before obtaining a global lock to avoid deadlocks
            let read_guard = take_guard!(self.global_lock_param.read());
            if let Some(global_lock_thread_id) = read_guard.to_owned() {
                global_lock_thread_id != thread::current().id()
            } else {
                false
            }
        };

        // -> this critical execution point protected with `method_lock_sync`
        // there is a moment between (1) making the decision and (2) taking the actual lock phases

        let mut option_global_lock = None;
        if wait_for_global_lock_release {
            // (2) taking the global lock
            option_global_lock = Some(self.global_lock());
        }

        let guard_storage = take_guard!(self.storage_map.lock());

        if let Some(mut global_lock) = option_global_lock {
            global_lock.unlock();
        }

        // release the method_lock_sync
        drop(guard_method_lock);

        guard_storage
    }

    /// Returns a global lock to exclusive thread access to the storage operations
    pub fn global_lock(&self) -> GlobalLock {
        GlobalLock::lock(self)
    }

    /// Inserts an item into the storage
    /// If the storage has an item with the key present, the item will be updated
    pub fn insert(&self, storage_item: StorageItem) {
        self.lock().insert(storage_item.key.clone(), storage_item);
    }

    /// Updates an item into the storage
    /// The item will be inserted if the storage does not have an item with the key present
    pub fn update(&self, storage_item: StorageItem) {
        self.insert(storage_item);
    }

    /// Gets an item from the storage corresponding to the key
    pub fn get(&self, key: &str) -> Option<StorageItem> {
        self.lock().get(key).cloned()
    }

    /// Removes an item from the storage
    pub fn remove(&self, key: &str) {
        self.lock().remove(key);
    }

    /// Clears the storage, removing all items
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Returns the keys of the stored items
    pub fn keys(&self) -> Vec<String> {
        self.lock().keys().cloned().collect()
    }

    /// Returns the inner object of the item corresponding to the key
    pub fn get_inner_object<T: bincode::Decode>(&self, key: &str) -> Option<T> {
        if let Some(item) = self.get(key) {
            let object: Option<T> = item.get_object();
            return object;
        }
        None
    }

    /// Updates the inner object of the item corresponding to the key
    pub fn update_inner_object<T: bincode::Encode>(&self, key: &str, obj: &T) -> bool {
        let mut guard = self.lock();
        if let Some(item) = guard.get_mut(key) {
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

    const THREADS_COUNT: usize = 100;
    const MAP_ENTRIES_PER_THREAD: usize = 10;

    #[test]
    fn storage_open_test() {
        let storage = Storage::open();

        // clean up the storage
        storage.clear();
        assert!(storage.keys().is_empty());
    }

    #[test]
    fn storage_insert_test() {
        let storage = Storage::open();

        // clean up the storage
        storage.clear();

        let key = "my_string1";
        let my_string = String::from("abc1");
        let storage_item =
            StorageItem::with_type(key, ItemType::Basic(BasicType::String), &my_string).unwrap();

        storage.insert(storage_item);

        let keys = storage.keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        // clean up the storage
        storage.clear();
    }

    #[test]
    fn storage_update_test() {
        let storage = Storage::open();

        // clean up the storage
        storage.clear();

        let key = "my_string2";
        let my_string = String::from("abc2");
        let mut storage_item =
            StorageItem::with_type(key, ItemType::Basic(BasicType::String), &my_string).unwrap();
        storage_item.description = Some("abc".to_string());

        storage.insert(storage_item);

        assert_eq!(storage.keys().len(), 1);
        let mut item = storage.get(key).unwrap();
        assert_eq!(item.description, Some("abc".to_string()));

        item.description = Some("abcd".to_string());
        storage.update(item);
        assert_eq!(
            storage.get(key).unwrap().description,
            Some("abcd".to_string())
        );

        // clean up the storage
        storage.clear();
    }

    #[test]
    fn storage_remove_test() {
        let storage = Storage::open();

        // clean up the storage
        storage.clear();

        let key = "my_string3";
        let my_string = String::from("abc3");
        let storage_item =
            StorageItem::with_type(key, ItemType::Basic(BasicType::String), &my_string).unwrap();

        storage.insert(storage_item);

        let keys = storage.keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        storage.remove(key);
        assert!(storage.keys().is_empty());
    }

    #[test]
    fn storage_clear_test() {
        let storage = Storage::open();

        // clean up the storage
        storage.clear();

        let key = "my_string4";
        let my_string = String::from("abc4");
        let storage_item =
            StorageItem::with_type(key, ItemType::Basic(BasicType::String), &my_string).unwrap();

        storage.insert(storage_item);

        let keys = storage.keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], key);

        storage.clear();
        assert!(storage.keys().is_empty());
    }

    #[test]
    fn storage_object_test() {
        let storage = Storage::open();

        // clean up the storage
        storage.clear();

        let key = "my_map1";

        let mut my_map1 = HashMap::<String, String>::new();
        my_map1.insert("1".into(), "One".into());
        my_map1.insert("2".into(), "Two".into());
        my_map1.insert("3".into(), "Three".into());

        let storage_type =
            ItemType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
        let storage_item = StorageItem::with_type(key, storage_type, &my_map1).unwrap();

        storage.insert(storage_item);

        let decoded_map1: HashMap<String, String> = storage.get_inner_object(key).unwrap();
        assert_eq!(my_map1, decoded_map1);

        my_map1.insert("4".into(), "Four".into());
        assert!(storage.update_inner_object(key, &my_map1));

        let decoded_map2 = storage
            .get_inner_object::<HashMap<String, String>>(key)
            .unwrap();
        assert_eq!(my_map1, decoded_map2);

        // clean up the storage
        storage.clear();
    }

    #[test]
    fn multithread_map_insert_test() {
        let key = "my_map";
        let storage = Arc::new(Storage::open());

        // clean up the storage
        storage.clear();

        // create a new map and insert into storage
        let my_map = HashMap::<String, String>::new();

        let storage_type =
            ItemType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
        let storage_item = StorageItem::with_type(key, storage_type, &my_map).unwrap();

        storage.insert(storage_item);

        // inserting map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let storage_clone = storage.clone();
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut global_lock = storage_clone.global_lock();
                let mut map: HashMap<String, String> = storage_clone.get_inner_object(key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    map.insert(entry_key, entry_value);
                }
                storage_clone.update_inner_object(key, &map);
                global_lock.unlock();
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // verify entries
        let map = storage
            .get_inner_object::<HashMap<String, String>>(key)
            .unwrap();
        assert_eq!(map.keys().count(), THREADS_COUNT * MAP_ENTRIES_PER_THREAD);
        for thread_number in 0..THREADS_COUNT {
            for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                let entry_key = format!("{}-{}", thread_number, entry_number);
                let entry_value = format!("{}", thread_number * entry_number);
                assert_eq!(map.get(&entry_key).unwrap(), &entry_value);
            }
        }

        // clean up the storage
        storage.clear();
    }

    #[test]
    fn multithread_map_get_test() {
        let storage = Arc::new(Storage::open());

        // clean up the storage
        storage.clear();

        // create a new map and insert entries
        let key = "my_map";
        let mut my_map = HashMap::<String, String>::new();

        for thread_number in 0..THREADS_COUNT {
            for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                let entry_key = format!("{}-{}", thread_number, entry_number);
                let entry_value = format!("{}", thread_number * entry_number);
                my_map.insert(entry_key, entry_value);
            }
        }

        let storage_type =
            ItemType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
        let storage_item = StorageItem::with_type(key, storage_type, &my_map).unwrap();

        storage.insert(storage_item);

        // get map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let storage_clone = storage.clone();
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let map: HashMap<String, String> = storage_clone.get_inner_object(key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    assert_eq!(map.get(&entry_key).unwrap(), &entry_value);
                }
                storage_clone.update_inner_object(key, &map);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // check entries count
        let map = storage
            .get_inner_object::<HashMap<String, String>>(key)
            .unwrap();
        assert_eq!(map.keys().count(), THREADS_COUNT * MAP_ENTRIES_PER_THREAD);

        // clean up the storage
        storage.clear();
    }

    #[test]
    fn multithread_map_remove_test() {
        let storage = Arc::new(Storage::open());

        // clean up the storage
        storage.clear();

        // create a new map and insert entries
        let key = "my_map";
        let mut my_map = HashMap::<String, String>::new();

        for thread_number in 0..THREADS_COUNT {
            for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                let entry_key = format!("{}-{}", thread_number, entry_number);
                let entry_value = format!("{}", thread_number * entry_number);
                my_map.insert(entry_key, entry_value);
            }
        }

        let storage_type =
            ItemType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
        let storage_item = StorageItem::with_type(key, storage_type, &my_map).unwrap();

        storage.insert(storage_item);

        // verify and remove map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let storage_clone = storage.clone();
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut global_lock = storage_clone.global_lock();
                let mut map: HashMap<String, String> = storage_clone.get_inner_object(key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    assert_eq!(map.remove(&entry_key).unwrap(), entry_value);
                }
                storage_clone.update_inner_object(key, &map);
                global_lock.unlock();
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // ensure the map is empty
        let map = storage
            .get_inner_object::<HashMap<String, String>>(key)
            .unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn multithread_multiobject_test() {
        let storage = Arc::new(Storage::open());

        // clean up the storage
        storage.clear();

        let key_prefix = "my_map";

        // creating and inserting map objects in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let storage_clone = storage.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let handler = thread::spawn(move || {
                let map = HashMap::<String, String>::new();
                let storage_type =
                    ItemType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
                let storage_item = StorageItem::with_type(&object_key, storage_type, &map).unwrap();

                storage_clone.insert(storage_item);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // verify inserted objects
        let object_keys = storage.keys();
        assert_eq!(object_keys.len(), THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let object_key = format!("{}-{}", key_prefix, thread_number);
            assert!(object_keys.contains(&object_key));
        }

        // inserting map entires in multiple threads
        let mut threads = Vec::with_capacity(THREADS_COUNT);
        for thread_number in 0..THREADS_COUNT {
            let storage_clone = storage.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut map: HashMap<String, String> =
                    storage_clone.get_inner_object(&object_key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    map.insert(entry_key, entry_value);
                }
                storage_clone.update_inner_object(&object_key, &map);
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
            let storage_clone = storage.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let entries_count = MAP_ENTRIES_PER_THREAD;
            let handler = thread::spawn(move || {
                let mut map: HashMap<String, String> =
                    storage_clone.get_inner_object(&object_key).unwrap();
                for entry_number in 0..entries_count {
                    let entry_key = format!("{}-{}", thread_number, entry_number);
                    let entry_value = format!("{}", thread_number * entry_number);
                    assert_eq!(map.remove(&entry_key).unwrap(), entry_value);
                }
                storage_clone.update_inner_object(&object_key, &map);
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
            let storage_clone = storage.clone();
            let object_key = format!("{}-{}", key_prefix, thread_number);
            let handler = thread::spawn(move || {
                let map: HashMap<String, String> =
                    storage_clone.get_inner_object(&object_key).unwrap();
                assert!(map.is_empty());

                // remove storage object
                storage_clone.remove(&object_key);
                thread::sleep(Duration::from_millis(1));
            });
            threads.push(handler);
        }

        // wait until the finish of all the spawned threads
        for handler in threads {
            handler.join().unwrap();
        }

        // ensure empty storage
        assert!(storage.keys().is_empty());
    }

    #[test]
    fn multithread_scoped_multiobject_test() {
        let storage = Arc::new(Storage::open());

        // clean up the storage
        storage.clear();

        let key_prefix = "my_map";

        // create and insert map objects into storage in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let storage_clone = storage.clone();
                scope.spawn(move || {
                    let map = HashMap::<String, String>::new();
                    let storage_type =
                        ItemType::Complex(ComplexType::Map(BasicType::String, BasicType::String));

                    let object_key = format!("{}-{}", key_prefix, thread_number);
                    let storage_item =
                        StorageItem::with_type(&object_key, storage_type, &map).unwrap();

                    storage_clone.insert(storage_item);
                });
            }
        });

        // verify inserted objects
        {
            let object_keys = storage.keys();
            assert_eq!(object_keys.len(), THREADS_COUNT);
            for thread_number in 0..THREADS_COUNT {
                let object_key = format!("{}-{}", key_prefix, thread_number);
                assert!(object_keys.contains(&object_key));
            }
        }

        // inserting map entires in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let storage_clone = storage.clone();
                scope.spawn(move || {
                    let object_key = format!("{}-{}", key_prefix, thread_number);

                    let mut map: HashMap<String, String> =
                        storage_clone.get_inner_object(&object_key).unwrap();

                    for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                        let entry_key = format!("{}-{}", thread_number, entry_number);
                        let entry_value = format!("{}", thread_number * entry_number);
                        map.insert(entry_key, entry_value);
                    }

                    storage_clone.update_inner_object(&object_key, &map);
                });
            }
        });

        // verify and remove map entires in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let storage_clone = storage.clone();
                scope.spawn(move || {
                    let object_key = format!("{}-{}", key_prefix, thread_number);
                    let mut map: HashMap<String, String> =
                        storage_clone.get_inner_object(&object_key).unwrap();

                    for entry_number in 0..MAP_ENTRIES_PER_THREAD {
                        let entry_key = format!("{}-{}", thread_number, entry_number);
                        let entry_value = format!("{}", thread_number * entry_number);
                        assert_eq!(map.remove(&entry_key).unwrap(), entry_value);
                    }

                    storage_clone.update_inner_object(&object_key, &map);
                });
            }
        });

        // verify and remove storage items in multiple threads
        thread::scope(|scope| {
            for thread_number in 0..THREADS_COUNT {
                let storage_clone = storage.clone();
                scope.spawn(move || {
                    let object_key = format!("{}-{}", key_prefix, thread_number);
                    let map: HashMap<String, String> =
                        storage_clone.get_inner_object(&object_key).unwrap();
                    assert!(map.is_empty());

                    // remove storage object
                    storage_clone.remove(&object_key);
                });
            }
        });

        // ensure empty storage
        assert!(storage.keys().is_empty());
    }

    #[test]
    fn storage_flush_load_test() {
        use std::fs;
        use std::path::Path;

        let mut storage = Storage::open();

        // clean up the storage
        storage.clear();

        assert_eq!(storage.flush(), Ok(()));

        // check the storage info is empty
        let result = storage.load_storage_info();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        let storage_data_path = storage.get_storage_data_path();

        // check the storage blob directory exists
        assert!(Path::new(&storage_data_path).exists());

        // check the storage blob directory is empty
        let paths = fs::read_dir(&storage_data_path).unwrap();
        assert_eq!(paths.count(), 0);

        let key = "my_map1";
        let mut my_map1 = HashMap::<String, String>::new();
        my_map1.insert("1".into(), "One".into());
        my_map1.insert("2".into(), "Two".into());
        my_map1.insert("3".into(), "Three".into());

        // insert the map into storage
        let storage_type =
            ItemType::Complex(ComplexType::Map(BasicType::String, BasicType::String));
        let storage_item = StorageItem::with_type(key, storage_type, &my_map1).unwrap();
        storage.insert(storage_item);

        // persist the storage
        assert_eq!(storage.flush(), Ok(()));

        // check the storage info has the map
        let result = storage.load_storage_info();
        assert!(result.is_ok());

        let storage_info = result.unwrap();
        assert!(storage_info.contains_key(key));

        // check the storage blob directory exists
        assert!(Path::new(&storage_data_path).exists());

        // check the storage blob directory has a single entry
        let paths = fs::read_dir(&storage_data_path).unwrap();
        let entries: Vec<_> = paths.flatten().map(|v| v.file_name()).collect();
        assert_eq!(entries.len(), 1);

        // check the entry id
        let item_id = storage_info.get(key).unwrap().0.to_ascii_lowercase();
        assert_eq!(entries[0].to_string_lossy().to_ascii_lowercase(), item_id);

        // clean up the storage
        storage.clear();

        let object_keys = storage.keys();
        assert!(object_keys.is_empty());

        // load storage
        assert_eq!(storage.load(), Ok(()));

        // verify loaded storage
        let object_keys = storage.keys();
        assert_eq!(object_keys.len(), 1);
        assert_eq!(object_keys[0], key);

        let map: HashMap<String, String> = storage.get_inner_object(key).unwrap();
        assert_eq!(my_map1, map);

        // clean up the storage
        storage.clear();
    }
}
