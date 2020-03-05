use crate::common::{make_channel, BoxErrResult, ScriptEvent, ScriptResultEvent, Sender};
use crate::script::js_engine::js_isolate::Isolate;
use futures::{SinkExt, StreamExt};
use serde::export::Result::Err;
use std::thread;

pub fn start() -> BoxErrResult<Sender<ScriptEvent>> {
    let (send, mut rev) = make_channel::<ScriptEvent>();
    let thread_builder = thread::Builder::new().name("v8-vm".into());
    thread_builder.spawn(move || {
        log::info!("Starting v8(js) engine");
        async_std::task::block_on(async {
            let mut isolate = Isolate::new();
            while let Some(event) = rev.next().await {
                let mut sender = event.sender;
                let location = event.location;
                let result = isolate
                    .module_execute(location)
                    .await
                    .map_err(|e| e.to_string());
                let r_event = ScriptResultEvent { result };
                if let Err(e) = sender.send(r_event).await {
                    log::error!("Error in broker: {}", e);
                }
                sender.close_channel();
            }
        });
    })?;
    Ok(send)
}
