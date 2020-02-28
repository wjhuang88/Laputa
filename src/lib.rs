pub mod common;
mod logger_config;
mod server;
mod service;
mod script;

pub fn new() -> server::Server {
    logger_config::print_banner();
    logger_config::init_log();

    log::info!("Start to initialize server");
    server::Server
}