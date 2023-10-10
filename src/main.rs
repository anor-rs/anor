use anor::utils::config;

fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();
    log::info!("Anor Data Store v{} started", env!("CARGO_PKG_VERSION"));

    log::info!("Loading configuration...");
    let config = config::get_config();
    if log::log_enabled!(log::Level::Trace) {
        log::trace!("loaded configuration params: {:#?}", config);
    }
}