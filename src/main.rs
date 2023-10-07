fn main() {
    log4rs::init_file("log.yaml", Default::default()).unwrap();
    log::info!("Anor Data Store v{} started", env!("CARGO_PKG_VERSION"));
}