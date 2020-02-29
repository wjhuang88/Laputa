use rlua::Lua;
use std::thread;

use crate::common::{make_channel, Result, ScriptEvent, Sender};
use bytes::{Buf, Bytes};
use futures::{SinkExt, StreamExt};
use serde::export::Option::Some;

pub fn start() -> Result<Sender<ScriptEvent>> {
    let (send, mut rev) = make_channel::<ScriptEvent>();
    let thread_builder = thread::Builder::new().name("lua-vm".into());
    thread_builder.spawn(move || {
        log::info!("Starting lua engine");
        async_std::task::block_on(async {
            let lua = Lua::new();
            while let Some(event) = rev.next().await {
                let mut sender = event.sender;
                let source = event.source;
                let result: String = lua
                    .context(|ctx| {
                        let global = ctx.globals();
                        ctx.load(source.bytes()).exec()?;
                        global.get::<_, String>("exports")
                    })
                    .unwrap_or(String::new());
                if let Err(e) = sender.send(Bytes::from(result)).await {
                    log::error!("Error in broker: {}", e);
                }

                sender.close_channel();
            }
        });
    })?;
    Ok(send)
}
