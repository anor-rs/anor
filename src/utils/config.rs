use std::collections::HashMap;
use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use super::{build_profile, envsubst};

const DEFAULT_CONFIG_FILENAME_RELEASE: &str = "anor-config.yaml";
const DEFAULT_CONFIG_FILENAME_DEBUG: &str = "anor-config.debug";
const DEFAULT_CONFIG_FILENAME_TEST: &str = "anor-config.test";

const DEFAULT_STORAGE_DATA_PATH: &str = "/var/anor";

const DEFAULT_API_SERVICE_LISTEN_ADDRESS: &str = "127.0.0.1";
const DEFAULT_API_SERVICE_LISTEN_PORT: u16 = 7311;
const DEFAULT_API_SERVICE_ENABLED: bool = false;

const DEFAULT_HTTP_SERVICE_LISTEN_ADDRESS: &str = "127.0.0.1";
const DEFAULT_HTTP_SERVICE_LISTEN_PORT: u16 = 8181;
const DEFAULT_HTTP_SERVICE_ENABLED: bool = false;

const DEFAULT_REMOTE_NODE: &str = "127.0.0.1:9191";

#[derive(Debug)]
pub struct Config {
    pub storage: Option<StorageConfig>,
    pub api: Option<ApiConfig>,
    pub http: Option<HttpConfig>,
    pub remote: Option<RemoteConfig>,
}

#[derive(Debug)]
pub struct StorageConfig {
    pub data_path: PathBuf,
}

#[derive(Debug)]
pub struct ApiConfig {
    pub listen_on: Vec<SocketAddr>,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct HttpConfig {
    pub listen_on: Vec<SocketAddr>,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct RemoteConfig {
    pub nodes: Vec<SocketAddr>,
}

pub fn get_config() -> Arc<Config> {
    let config_filename = get_config_filename();
    let mut config_file = std::fs::File::open(config_filename)
        .unwrap_or_else(|_| panic!("Could not open {} file.", config_filename));

    let mut config_content = String::new();
    if let Err(err) = config_file.read_to_string(&mut config_content) {
        log::error!("{}", err);
        panic!("{}", err);
    }

    let config_substituted = envsubst::substitute(&config_content);

    let config_map: HashMap<String, HashMap<String, String>> =
        serde_yaml::from_str(&config_substituted)
            .unwrap_or_else(|_| panic!("Could not parse {} file.", config_filename));

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("loaded config:\n{:#?}", config_map);
    }

    let mut config = Config {
        storage: None,
        api: None,
        http: None,
        remote: None,
    };

    let map_key = "storage";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let data_path = parse_storage_path(config_node);
        config.storage = Some(StorageConfig { data_path });
    }

    let map_key = "api";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let listen_on = parse_listen_on(
            config_node,
            DEFAULT_API_SERVICE_LISTEN_ADDRESS,
            DEFAULT_API_SERVICE_LISTEN_PORT,
        );
        let enabled = parse_enabled(config_node).unwrap_or(DEFAULT_API_SERVICE_ENABLED);
        config.api = Some(ApiConfig { listen_on, enabled });
    }

    let map_key = "http";
    if config_map.contains_key(map_key) {
        let config_node = &config_map[map_key];
        let listen_on = parse_listen_on(
            config_node,
            DEFAULT_HTTP_SERVICE_LISTEN_ADDRESS,
            DEFAULT_HTTP_SERVICE_LISTEN_PORT,
        );
        let enabled = parse_enabled(config_node).unwrap_or(DEFAULT_HTTP_SERVICE_ENABLED);
        config.http = Some(HttpConfig { listen_on, enabled });
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

fn parse_remote(node: &HashMap<String, String>) -> RemoteConfig {
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

    RemoteConfig { nodes }
}

fn parse_enabled(node: &HashMap<String, String>) -> Option<bool> {
    let node_key = "enabled";
    if node.contains_key(node_key) {
        Some(node[node_key].parse().unwrap())
    } else {
        None
    }
}

fn get_config_filename() -> &'static str {
    if build_profile::debug_mode() {
        if build_profile::is_cargo_test() {
            DEFAULT_CONFIG_FILENAME_TEST
        } else {
            DEFAULT_CONFIG_FILENAME_DEBUG
        }
    } else {
        DEFAULT_CONFIG_FILENAME_RELEASE
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn config_file_test() {
        assert!(build_profile::is_cargo_test());
        assert_eq!(get_config_filename(), DEFAULT_CONFIG_FILENAME_TEST);
    }

    #[test]
    fn config_storage_test() {
        let config = get_config();
        assert!(config.storage.is_some());

        let storage = config.storage.as_ref().unwrap();
        let data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("tmp")
            .join("anor");
        assert_eq!(storage.data_path, data_path);
    }

    #[test]
    fn config_api_test() {
        let config = get_config();
        assert!(config.api.is_some());

        let api = config.api.as_ref().unwrap();
        assert_eq!(api.listen_on.len(), 1);
        assert_eq!(api.listen_on[0], "127.0.0.1:9191".parse().unwrap());
        assert!(api.enabled);
    }

    #[test]
    fn config_http_test() {
        let config = get_config();
        assert!(config.http.is_some());

        let http = config.http.as_ref().unwrap();
        assert_eq!(http.listen_on.len(), 1);
        assert_eq!(http.listen_on[0], "127.0.0.1:8181".parse().unwrap());
        assert!(http.enabled);
    }

    #[test]
    fn config_remote_test() {
        let config = get_config();
        assert!(config.remote.is_some());

        let remote = config.remote.as_ref().unwrap();
        assert_eq!(remote.nodes.len(), 1);
        assert_eq!(remote.nodes[0], "127.0.0.1:9191".parse().unwrap());
    }
}
