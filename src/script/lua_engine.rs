use rlua::{Lua, ThreadStatus};
use std::thread;

use crate::common::{
    make_channel, BoxErrResult, ResponseData, ScriptEvent, ScriptResultEvent, Sender,
};
use bytes::{BufMut, Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use serde::export::Option::Some;
use std::collections::HashMap;

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
                let eval: BoxErrResult<ResponseData> = lua.context(|ctx| {
                    let resp = ctx.load(source.as_slice()).call::<_, rlua::Table>(())?;
                    let run: rlua::Function = resp.get("run")?;
                    let result: rlua::MultiValue = run.call(())?;
                    let mut data = ResponseData {
                        status: 200,
                        headers: HashMap::new(),
                        body: Bytes::new(),
                    };
                    for value in result {
                        match value {
                            rlua::Value::Integer(int_value) => data.status = int_value as u16,
                            rlua::Value::Table(table) => {
                                let map = &mut data.headers;
                                for pair in table.pairs::<rlua::Value, rlua::Value>() {
                                    if let (rlua::Value::String(key), rlua::Value::String(value)) =
                                        pair?
                                    {
                                        map.insert(
                                            key.to_str()?.to_string(),
                                            value.to_str()?.to_string(),
                                        );
                                    }
                                }
                            }
                            rlua::Value::Function(function) => {
                                let mut body = BytesMut::new();
                                while let rlua::Value::String(line) =
                                    function.call::<_, rlua::Value>(())?
                                {
                                    let line_bytes = line.as_bytes();
                                    body.put(line_bytes);
                                }
                                data.body = body.freeze()
                            }
                            _ => log::error!("[LUA] Unsupported return type"),
                        }
                    }
                    Ok(data)
                });
                let r_event = ScriptResultEvent {
                    result: eval.map_err(|e| format!("[LUA] {}", e)),
                };
                if let Err(e) = sender.send(r_event).await {
                    log::error!("[LUA] Error in broker: {}", e);
                }

                sender.close_channel();
            }
        });
    })?;
    Ok(send)
}
