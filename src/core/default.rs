use std::io::Error;
use std::iter::FromIterator;
use crate::core::{
    FileSystem,
    LogHandler,
    DatabaseSystem,
    LifeCycleHandler,
    Governance,
    CacheHandler,
    Message
};

pub struct LocalFileSystem;
impl LocalFileSystem {
    pub fn new() -> LocalFileSystem {
        LocalFileSystem{}
    }
}
impl FileSystem for LocalFileSystem {
    fn list_dir(&self, path: &str) -> Result<Vec<String>, Error> {
        match std::fs::read_dir(path) {
            Ok(paths) => {
                let itr = paths.map(move |item| {
                    if let Ok(dir) = item {
                        if let Some(path) = dir.path().to_str() {
                            String::from(path)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                }).filter(|item| {
                    !item.is_empty()
                });
                let result = Vec::from_iter(itr);
                Ok(result)
            },
            Err(e) => Err(e)
        }
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, Error> {
        std::fs::read(path)
    }
}

pub struct LocalLogHandler;
impl LocalLogHandler {
    pub fn new() -> LocalLogHandler {
        LocalLogHandler {}
    }
}
impl LogHandler for LocalLogHandler {}

pub struct MockDatabaseSystem;
impl MockDatabaseSystem {
    pub fn new() -> MockDatabaseSystem {
        MockDatabaseSystem {}
    }
}
impl DatabaseSystem for MockDatabaseSystem {}

pub struct DefaultLifeCycleHandler;
impl DefaultLifeCycleHandler {
    pub fn new() -> DefaultLifeCycleHandler {
        DefaultLifeCycleHandler {}
    }
}
impl LifeCycleHandler for DefaultLifeCycleHandler {}

pub struct DefaultGovernance;
impl DefaultGovernance {
    pub fn new() -> DefaultGovernance {
        DefaultGovernance {}
    }
}
impl Governance for DefaultGovernance {}

pub struct LocalCacheHandler;
impl LocalCacheHandler {
    pub fn new() -> LocalCacheHandler {
        LocalCacheHandler {}
    }
}
impl CacheHandler for LocalCacheHandler {}

pub struct LocalMessage;
impl LocalMessage {
    pub fn new() -> LocalMessage {
        LocalMessage {}
    }
}
impl Message for LocalMessage {}

