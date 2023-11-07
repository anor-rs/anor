use anor_api::{client::api_client, SocketClient};
use anor_utils::config;

fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();
    log::info!("api client/server test");

    // load the configuration
    let config = config::load();

    // api client tests
    let mut api_client1 = api_client::Client::with_config(config.clone());
    api_client1.connect().expect("client connection error");

    let keys = api_client1.keys();
    log::debug!("{:?}", keys);

    _ = api_client1.disconnect();

    /*
    let msg1 = String::from("Hi there1!");
    client1.set_item(msg1).expect("set item error");
    thread::sleep(Duration::from_millis(20));
    let msg2 = String::from("Hi there2!");
    client1.set_item(msg2).expect("set item error");

    let mut client2 = StorageApiClient::with_config(config.clone());
    client2.connect().expect("client connection error");

    let msg1 = String::from("Hi there1!");
    client2.set_item(msg1).expect("set item error");
    thread::sleep(Duration::from_millis(20));
    let msg2 = String::from("Hi there2!");
    client2.set_item(msg2).expect("set item error");
    */
}
