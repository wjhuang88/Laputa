pub mod common;
mod inner_pages;
mod logger_config;
mod script;
mod server;
pub mod service;

pub fn new() -> server::Server {
    logger_config::print_banner();
    logger_config::init_log();

    log::info!("Start to initialize server");
    server::Server::new()
}
