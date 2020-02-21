use crate::script::ScriptHandler;
use rlua::Lua;
use actix::{Actor, Context as ActixCtx, Handler};
use crate::script::interpreter::ScriptMessage;
use std::io;

pub struct LuaHandler {
    lua : Lua,
}
impl LuaHandler {
    pub fn new() -> Box<LuaHandler> {
        Box::new(LuaHandler { lua: Lua::new() })
    }
}

impl Actor for LuaHandler {
    type Context = ActixCtx<Self>;
}

impl Handler<ScriptMessage> for LuaHandler {
    type Result = Result<String, io::Error>;

    fn handle(&mut self, msg: ScriptMessage, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.exec(msg.source, msg.file))
    }
}

impl ScriptHandler for LuaHandler {
    type SourceType = Vec<u8>;
    type ValueType = String;

    fn exec(&mut self, source: Self::SourceType, file_name: String) -> Self::ValueType {
        self.lua.context(|ctx| {
            let globals = ctx.globals();
            ctx.load(&source).set_name(&file_name).unwrap().exec().unwrap();
            globals.get::<_, String>("exports").unwrap_or("".to_string())
        })
    }
}