use rlua::Lua;
use std::thread;

use crate::common::{make_channel, BoxErrResult, ScriptEvent, ScriptResultEvent, Sender};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use serde::export::Option::Some;

pub fn start() -> BoxErrResult<Sender<ScriptEvent>> {
    let (send, mut rev) = make_channel::<ScriptEvent>();
    let thread_builder = thread::Builder::new().name("lua-vm".into());
    thread_builder.spawn(move || {
        log::info!("Starting lua engine");
        async_std::task::block_on(async {
            let lua = Lua::new();
            while let Some(event) = rev.next().await {
                let mut sender = event.sender;
                let location = event.location;
                let source = async_std::fs::read(location).await.unwrap();
                let result: String = lua
                    .context(|ctx| {
                        let global = ctx.globals();
                        ctx.load(source.as_slice()).exec()?;
                        global.get::<_, String>("exports")
                    })
                    .unwrap_or(String::new());
                let r_event = ScriptResultEvent {
                    result: Ok(Bytes::from(result)),
                };
                if let Err(e) = sender.send(r_event).await {
                    log::error!("Error in broker: {}", e);
                }

                sender.close_channel();
            }
        });
    })?;
    Ok(send)
}
