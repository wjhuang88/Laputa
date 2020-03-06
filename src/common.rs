use async_std::task;
use futures::channel::mpsc;
use std;
use std::collections::HashMap;
use std::future::Future;

pub type BoxErrResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type StrErrResult<T> = std::result::Result<T, String>;

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
    F: Future<Output = BoxErrResult<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            log::error!("Error in task: {}", e)
        }
    })
}

pub struct ScriptEvent {
    pub(crate) sender: Sender<ScriptResultEvent>,
    pub(crate) location: String,
    pub(crate) request: RequestData,
}

pub struct ScriptResultEvent {
    pub(crate) result: StrErrResult<ResponseData>,
}

pub struct RequestData {
    pub(crate) headers: http::HeaderMap,
    pub(crate) body: bytes::Bytes,
    pub(crate) uri: String,
    pub(crate) query: String,
}

pub struct ResponseData {
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: bytes::Bytes,
    pub(crate) status: u16,
}
