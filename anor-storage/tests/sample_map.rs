#[test]
fn sample_map() {
    use anor_storage::storage::{storage_item::*, Storage};
    use std::collections::HashMap;

    let key = "my_map";

    let mut sample_map = HashMap::<u8, String>::new();
    sample_map.insert(1, "One".into());
    sample_map.insert(2, "Two".into());
    sample_map.insert(3, "Three".into());

    // storage would be dropped after leaving the scope
    {
        // open a storage according to the configuration given in config.yaml
        let storage = Storage::open();

        // define item type
        let storage_type = ItemType::Complex(ComplexType::Map(BasicType::U8, BasicType::String));

        // create a new item with an inner map object
        let mut storage_item = StorageItem::with_type(key, storage_type, &sample_map).unwrap();
        storage_item.set_description("My sample spelling dictionary");
        storage_item.add_tag("dictionary");
        storage_item.add_metafield("language", "en");

        // insert item into storage
        storage.insert(storage_item);

        // get the map from the storage by key
        let mut map: HashMap<u8, String> = storage.get_inner_object(key).unwrap();
        assert_eq!(map, sample_map);

        // modify the map
        map.insert(4, "Four".into());

        // update the storage
        storage.update_inner_object(key, &map);

        // storage would be dropped here as it going out from the scope
        // this will persist storage content
        // the storage can be manually dropped also by using: drop(storage)
    }

    // open the storage
    let storage_loaded = Storage::open();

    // get the map from the storage by key
    let map_loaded: HashMap<u8, String> = storage_loaded.get_inner_object(key).unwrap();
    assert_eq!(
        map_loaded,
        HashMap::from([
            (1, "One".into()),
            (2, "Two".into()),
            (3, "Three".into()),
            (4, "Four".into())
        ])
    );
}
