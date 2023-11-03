#[test]
fn sample_string() {
    use anor_storage::storage::{storage_item::*, Storage};

    let key = "my_string";
    let sample_string = String::from("abc");

    // storage would be dropped after leaving the scope
    {
        // open a storage according to the configuration given in config.yaml
        let storage = Storage::open();

        // create a new item with an inner string object
        let storage_item = StorageItem::new(key, &sample_string).unwrap();

        // insert item into storage
        storage.insert(storage_item);

        // get the string from the storage by key
        let mut string_value: String = storage.get_inner_object(key).unwrap();
        assert_eq!(string_value, sample_string);

        // modify the string
        string_value += "def";

        // update the storage
        storage.update_inner_object(key, &string_value);

        // `storage` would be dropped here as it going out from the scope
        // this will persist storage content
        // the storage can be manually dropped also by using: drop(storage)
    }

    // open the storage
    let storage_loaded = Storage::open();

    // get the string from the storage by key
    let loaded_value = storage_loaded.get_inner_object::<String>(key).unwrap();
    assert_eq!(loaded_value, "abcdef");
}
