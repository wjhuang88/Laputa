pub trait LogHandler : Copy + Send {}

#[derive(Copy, Clone)]
pub struct LocalLogHandler;
impl LocalLogHandler {
    pub fn new() -> LocalLogHandler {
        LocalLogHandler {}
    }
}
impl LogHandler for LocalLogHandler {}

#[derive(Copy, Clone)]
pub enum LogSelector {
    Default(LocalLogHandler)
}
impl Default for LogSelector {
    fn default() -> Self {
        LogSelector::Default(LocalLogHandler::new())
    }
}