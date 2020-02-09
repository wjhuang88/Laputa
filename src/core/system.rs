use std::io;
use std::sync::Arc;

use crate::core::{
    CacheHandler,
    DatabaseSystem,
    FileSystem,
    Governance,
    LifeCycleHandler,
    LogHandler,
    Message,
};

pub struct System {
    log_handler: Arc<dyn LogHandler>,
    file_system: Arc<dyn FileSystem>,
    database_system: Arc<dyn DatabaseSystem>,
    life_cycle_handler: Arc<dyn LifeCycleHandler>,
    governance: Arc<dyn Governance>,
    cache_handler: Arc<dyn CacheHandler>,
    message: Arc<dyn Message>,
}

impl System  {
    pub fn create(
        log_handler: impl LogHandler + 'static,
        file_system: impl FileSystem + 'static,
        database_system: impl DatabaseSystem + 'static,
        life_cycle_handler: impl LifeCycleHandler + 'static,
        governance: impl Governance + 'static,
        cache_handler: impl CacheHandler + 'static,
        message: impl Message + 'static
    ) -> Arc<System> {
        let system = System {
            log_handler: Arc::new(log_handler),
            file_system: Arc::new(file_system),
            database_system: Arc::new(database_system),
            life_cycle_handler: Arc::new(life_cycle_handler),
            governance: Arc::new(governance),
            cache_handler: Arc::new(cache_handler),
            message: Arc::new(message),
        };
        Arc::new(system)
    }

    pub fn list_dir(&self, path: &str) -> io::Result<Vec<String>> {
        self.file_system.list_dir(path)
    }

    pub fn read_file(&self, path: &str) -> io::Result<Vec<u8>> {
        self.file_system.read_file(path)
    }
}