use async_std::task;
use laputa::common;
use laputa::service::Service;

fn main() -> common::Result<()> {
    task::block_on(async {
        let mut server = laputa::new();
        let lua_test = Service::Lua("testlua", "./deploy/ping_lua.lua");
        let static_test = Service::Static("file", "./deploy/ping_static.html");
        let js_test = Service::JavaScript("testjs", "./deploy/ping_javascript.js");
        server.register(lua_test);
        server.register(static_test);
        server.register(js_test);
        server.start().await?;
        Ok(())
    })
}
