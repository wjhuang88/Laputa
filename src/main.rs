mod core;
mod log;
mod plugin;
mod script;

use crate::core::{
    System, default::*
};
use crate::script::{LuaHandler, ScriptHandler};

fn main() {
    let system = System::create(
        LocalLogHandler::new(),
        LocalFileSystem::new(),
        MockDatabaseSystem::new(),
        DefaultLifeCycleHandler::new(),
        DefaultGovernance::new(),
        LocalCacheHandler::new(),
        LocalMessage::new()
    );

    LuaHandler::new().register_scripts(system, "./deploy")
}
