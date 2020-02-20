pub trait CacheHandler : Copy + Send {}

#[derive(Copy, Clone)]
pub struct LocalCacheHandler;
impl LocalCacheHandler {
    pub fn new() -> LocalCacheHandler {
        LocalCacheHandler {}
    }
}
impl CacheHandler for LocalCacheHandler {}

#[derive(Copy, Clone)]
pub enum CacheSelector {
    Default(LocalCacheHandler)
}
impl Default for CacheSelector {
    fn default() -> Self {
        CacheSelector::Default(LocalCacheHandler::new())
    }
}