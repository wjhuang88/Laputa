use async_std::task;
use futures::channel::mpsc;
use std;
use std::future::Future;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct ServiceState {
    #[allow(dead_code)]
    state_map: std::collections::HashMap<&'static str, &'static str>,
}

impl ServiceState {
    pub fn new() -> ServiceState {
        ServiceState {
            state_map: std::collections::HashMap::new(),
        }
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
            log::error!("Error in task: {}", e)
        }
    })
}

pub struct ScriptEvent {
    pub sender: Sender<bytes::Bytes>,
    pub source: bytes::Bytes,
}
