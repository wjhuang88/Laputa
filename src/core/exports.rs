use std::io;

pub trait LogHandler {}

pub trait FileSystem {
    fn list_dir(&self, path: &str) -> io::Result<Vec<String>>;
    fn read_file(&self, path: &str) -> io::Result<Vec<u8>>;
}

pub trait DatabaseSystem {}

pub trait LifeCycleHandler {}

pub trait Governance {}

pub trait CacheHandler {}

pub trait Message {}
