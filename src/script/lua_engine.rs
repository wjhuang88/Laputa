use rlua::Lua;
use std::thread;

use crate::common::{Receiver, Result, make_channel, Sender, spawn_and_log_error};
use bytes::Bytes;
use serde::export::Option::Some;
use std::sync::atomic::AtomicUsize;
use futures::StreamExt;

pub fn start() -> Result<Sender<Bytes>> {
    let (send, mut rev) = make_channel::<Bytes>();
    let thread_builder = thread::Builder::new().name("lua-vm".into());
    thread_builder.spawn(move || {
        let counter = AtomicUsize::new(0);
        let lua = Lua::new();
        log::info!("Starting lua engine");
        async_std::task::block_on(async {
            while let Some(bytes) = rev.next().await {
                let string = String::from_utf8(bytes.to_vec());
                spawn_and_log_error(async {
                    log::info!("get msg: {}", string?);
                    Ok(())
                });
            }
        });
    })?;
    Ok(send)
}