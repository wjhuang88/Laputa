use std::collections::HashMap;

pub mod common;
mod logger_config;
mod script;
mod server;
pub mod service;

pub fn new<'s>() -> server::Server<'s> {
    logger_config::print_banner();
    logger_config::init_log();

    log::info!("Start to initialize server");
    server::Server {
        route_map: HashMap::new(),
    }
}
