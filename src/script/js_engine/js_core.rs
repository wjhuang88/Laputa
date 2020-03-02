use crate::common::{make_channel, Result, ScriptEvent, Sender};
use crate::script::js_engine::js_isolate::Isolate;
use futures::{SinkExt, StreamExt};
use serde::export::Result::Err;
use std::thread;

pub fn start() -> Result<Sender<ScriptEvent>> {
    let (send, mut rev) = make_channel::<ScriptEvent>();
    let thread_builder = thread::Builder::new().name("v8-vm".into());
    thread_builder.spawn(move || {
        log::info!("Starting v8(js) engine");
        async_std::task::block_on(async {
            let mut isolate = Isolate::new();
            while let Some(event) = rev.next().await {
                let mut sender = event.sender;
                let source = event.source;
                let name = event.name;
                let module_id = isolate.load_module_from_bytes(source, name, true).await;
                if let Err(e) = module_id {
                    log::error!("{}", e);
                    continue;
                }
                let module_id = module_id.unwrap();
                if let Err(e) = isolate.instantiate_module(module_id).await {
                    log::error!("{}", e);
                    continue;
                }
                // let result: String;
                // if let Err(e) = sender.send(Bytes::from(result)).await {
                //     log::error!("Error in broker: {}", e);
                // }
                sender.close_channel();
                todo!("finish it!");
            }
        });
    })?;
    Ok(send)
}
