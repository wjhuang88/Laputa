use std;
use futures::channel::mpsc;
use std::future::Future;
use async_std::task;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct ServiceState {
    state_map: std::collections::HashMap<&'static str, &'static str>
}

impl ServiceState {
    pub fn new() -> ServiceState {
        ServiceState { state_map: std::collections::HashMap::new() }
    }
}

pub type Sender<T> = mpsc::UnboundedSender<T>;
pub type Receiver<T> = mpsc::UnboundedReceiver<T>;

pub fn make_channel<T>() -> (Sender<T>, Receiver<T>) {
    mpsc::unbounded()
}

pub fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
    where
        F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            log::error!("{}", e)
        }
    })
}