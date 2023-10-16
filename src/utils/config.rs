use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

const DEFAULT_STORAGE_DATA_PATH: &str = "/var/anor";

const DEFAULT_SERVER_LISTEN_ADDRESS: &str = "127.0.0.1";
const DEFAULT_SERVER_LISTEN_PORT: u16 = 7311;

const DEFAULT_FILE_SERVER_LISTEN_ADDRESS: &str = "127.0.0.1";
const DEFAULT_FILE_SERVER_LISTEN_PORT: u16 = 8181;

const DEFAULT_REMOTE_NODE: &str = "127.0.0.1:9191";

#[derive(Debug)]
pub struct Config {
    pub storage: Option<Storage>,
    pub server: Option<Server>,
    pub file_server: Option<FileServer>,
    pub remote: Option<Remote>,
}

#[derive(Debug)]
pub struct Storage {
    pub data_path: PathBuf,
}

#[derive(Debug)]
pub struct Server {
    pub listen_on: Vec<SocketAddr>,
}

#[derive(Debug)]
pub struct FileServer {
    pub listen_on: Vec<SocketAddr>,
}

#[derive(Debug)]
pub struct Remote {
    pub nodes: Vec<SocketAddr>,
}

pub fn get_config() -> Arc<Config> {
    let config_file = std::fs::File::open("config.yaml").expect("Could not open config file.");
    let config_map: HashMap<String, HashMap<String, String>> =
        serde_yaml::from_reader(config_file).expect("Could not parse config file.");

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("loaded config:\n{:#?}", config_map);
    }

    let mut config = Config {
        storage: None,
        server: None,
        file_server: None,
        remote: None,
    };

    let map_key = "storage";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let data_path = parse_storage_path(config_node);
        config.storage = Some(Storage {
            data_path
        });
    }

    let map_key = "server";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let listen_on = parse_listen_on(
            config_node,
            DEFAULT_SERVER_LISTEN_ADDRESS,
            DEFAULT_SERVER_LISTEN_PORT,
        );
        config.server = Some(Server {
            listen_on,
        });
    }

    let map_key = "file_server";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let listen_on = parse_listen_on(
            config_node,
            DEFAULT_FILE_SERVER_LISTEN_ADDRESS,
            DEFAULT_FILE_SERVER_LISTEN_PORT,
        );
        config.file_server = Some(FileServer { listen_on });
    }

    let map_key = "remote";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let remote = parse_remote(config_node);
        config.remote = Some(remote);
    }

    if log::log_enabled!(log::Level::Debug) {
        log::debug!("parsed config:\n{:#?}", config);
    }

    Arc::new(config)
}

fn parse_listen_on(
    node: &HashMap<String, String>,
    default_listen_address: &str,
    default_listen_port: u16,
) -> Vec<SocketAddr> {
    let node_key = "listen_addresses";
    let listen_addresses = if node.contains_key(node_key) {
        node[node_key]
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<_>>()
    } else {
        vec![default_listen_address]
    };

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("config: listen_addresses: {:?}", listen_addresses);
    }

    let node_key = "listen_port";
    let port = if node.contains_key(node_key) {
        node[node_key].parse().unwrap()
    } else {
        default_listen_port
    };

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("config: listen_port: {}", port);
    }

    let mut listen_on = Vec::<SocketAddr>::with_capacity(listen_addresses.len());
    for listen_addres in listen_addresses {
        let ip_address: IpAddr = listen_addres.parse().unwrap();
        let socket_addres = SocketAddr::new(ip_address, port);
        listen_on.push(socket_addres);
    }

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("parsed: listen_on: {:?}", listen_on);
    }

    listen_on
}

fn parse_storage_path(node: &HashMap<String, String>) -> PathBuf {
    let node_key = "data_path";
    let storage_path = if node.contains_key(node_key) {
        node[node_key].parse().unwrap()
    } else {
        String::from(DEFAULT_STORAGE_DATA_PATH)
    };

    PathBuf::from(storage_path)
}

fn parse_remote(node: &HashMap<String, String>) -> Remote {
    let node_key = "nodes";
    let remote_nodes = if node.contains_key(node_key) {
        node[node_key]
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<_>>()
    } else {
        vec![DEFAULT_REMOTE_NODE]
    };

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("config: remote nodes: {:?}", remote_nodes);
    }

    let mut nodes = Vec::<SocketAddr>::with_capacity(remote_nodes.len());
    for node in remote_nodes {
        let socket_addr: SocketAddr = node.parse().unwrap();
        nodes.push(socket_addr);
    }

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("parsed: remote nodes: {:?}", nodes);
    }

    Remote { nodes }
}
