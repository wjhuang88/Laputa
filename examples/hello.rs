use async_std::task;
use laputa::common;
use laputa::service::Service;

fn main() -> common::Result<()> {
    task::block_on(async {
        let mut server = laputa::new();
        let lua_test = Service::Lua("testlua", "./deploy/ping_lua.lua");
        server.register(lua_test);
        server.start().await?;
        Ok(())
    })
}
