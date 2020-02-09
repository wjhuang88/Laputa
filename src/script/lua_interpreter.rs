use crate::script::ScriptHandler;
use crate::core::System;
use std::path::Path;
use std::sync::Arc;
use rlua::Lua;

pub struct LuaHandler {
    lua : Lua
}
impl LuaHandler {
    pub fn new() -> LuaHandler {
        LuaHandler {lua: Lua::new()}
    }
}

impl ScriptHandler for LuaHandler {
    fn register_scripts(&self, system: Arc<System>, dir: &str) {
        if let Ok(paths) = system.list_dir(dir) {
            for path in paths {
                if let Ok(bytes) = system.read_file(&path) {
                    let route = String::from(Path::new(&path).file_name().unwrap().to_str().unwrap()).replace(".lua", "");
                    let real_route : &str;
                    self.lua.context(|context| {
                        let globals = context.globals();
                        let loaded = context.load(&bytes).set_name(&path)?.exec();
                    });
                }
            }
        }
    }
}