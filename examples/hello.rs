use async_std::task;
use laputa::common;
use laputa::service::ScriptType;
use tide::http::Method;
use tide::Response;

fn main() -> common::BoxErrResult<()> {
    task::block_on(async {
        let mut server = laputa::new();
        server.route_static(
            Method::GET,
            mime::TEXT_HTML_UTF_8,
            "static",
            "./deploy/ping_static.html",
        )?;
        server.route_script(ScriptType::Lua, Method::GET, "lua", "./deploy/ping_lua.lua")?;
        server.route_script(
            ScriptType::JavaScript,
            Method::GET,
            "js",
            "./deploy/ping_javascript.js",
        )?;
        server.route_fn(Method::GET, "fn", |_| async {
            Response::new(200).body_string("Hello World!".to_string())
        })?;
        server.start().await?;
        Ok(())
    })
}
