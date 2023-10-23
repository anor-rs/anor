# Anor In-Memory Data Storage

[![crates.io](https://img.shields.io/crates/v/anor)](https://crates.io/crates/anor)
[![docs](https://img.shields.io/docsrs/anor)](https://docs.rs/anor)
[![build & test](https://github.com/sheroz/anor/actions/workflows/ci.yml/badge.svg)](https://github.com/sheroz/anor/actions/workflows/ci.yml)
[![MIT](https://img.shields.io/github/license/sheroz/anor)](https://github.com/sheroz/anor/tree/main/LICENSE.txt)

<img src="docs/img/anor-wb.svg" width=25%>

Anor storage is an open-source, in-memory key-value data store written in Rust.

Anor storage supports point-in-time, snapshot-based persistence.

## Project Stage

**Development**: this project already has milestone releases, but is still under active development, you should not expect full stability yet.

## Usage

Please look at [samples](src/bin)

### Usage samples

- [Sample String](src/bin/sample_string.rs)
- [Sample Map](src/bin/sample_map.rs)

### Sample string [sample_string.rs](src/bin/sample_string.rs)

```rust
use anor::storage::{storage_item::*, Storage};

let key = "my_string";
let sample_string = String::from("abc");

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

    // storage would be dropped here as it going out from the scope
    // this will persist storage content
    // the storage can be manually dropped also by using: drop(storage)
}

// open the storage
let storage_loaded = Storage::open();

// get the string from the storage by key
let loaded_value = storage_loaded.get_inner_object::<String>(key).unwrap();
assert_eq!(loaded_value, "abcdef");

println!("Loaded object: {}: {:?}", key, loaded_value);
```

### Sample Map [sample_map.rs](src/bin/sample_map.rs)

```rust
use anor::storage::{storage_item::*, Storage};
use std::collections::HashMap;

let key = "my_map";

let mut sample_map = HashMap::<u8, String>::new();
sample_map.insert(1, "One".into());
sample_map.insert(2, "Two".into());
sample_map.insert(3, "Three".into());

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

println!("Loaded object: {}: {:?}", key, map_loaded);
```
