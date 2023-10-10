use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

#[derive(Debug)]
pub struct Config {
    pub server: Option<Server>,
    pub file_server: Option<FileServer>,
    pub remote: Option<Remote>,
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
        server: None,
        file_server: None,
        remote: None,
    };

    let map_key = "server";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let listen_on = parse_listen_on(config_node);
        config.server = Some(Server{listen_on});
    }

    let map_key = "file_server";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let listen_on = parse_listen_on(config_node);
        config.file_server = Some(FileServer{listen_on});
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

fn parse_listen_on(node: &HashMap<String, String>) -> Vec::<SocketAddr> {
    let node_key = "listen_addresses";
    let listen_addresses = if node.contains_key(node_key) {
        node[node_key]
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<_>>()
    } else {
        vec!["127.0.0.1"]
    };

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("config: listen_addresses: {:?}", listen_addresses);
    }

    let node_key = "listen_port";
    let mut port: u16 = 7311;
    if node.contains_key(node_key) {
        port = node[node_key].parse().unwrap();
    }

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

fn parse_remote(node: &HashMap<String, String>) -> Remote {
    let node_key = "nodes";
    let remote_nodes = if node.contains_key(node_key) {
        node[node_key]
            .split(',')
            .map(|s| s.trim())
            .collect::<Vec<_>>()
    } else {
        vec![]
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
