# Laputa

项目重构中，敬请期待。

---

[![build](https://img.shields.io/github/workflow/status/wjhuang88/Laputa/Rust/master)](https://github.com/wjhuang88/laputa/actions)

## 依赖配置
*注意：由于v8引擎的源码编译耗费非常多的时间，现在使用源码依赖会花费大量的编译时间。后续考虑将各脚本引擎分features按需编译。本项目完善后的使用场景应当是直接使用独立的可执行文件启动服务，并计划提供云环境支持工具(docker/kubernete自动配置等)*
```toml
[dependencies]
laputa = { git = "https://github.com/wjhuang88/Laputa.git" }
```

## 使用
可以部署rust原生代码作为服务处理器，也可以部署JavaScript代码和Lua代码来处理请求，静态文件访问也是支持的。

### 示例脚本和文件
在根目录下创建目录：deploy，并创建如下文件：

deploy/ping_javascript.js:
```javascript
let uri = request.uri
let query = request.query
let testHeader = request.header("User-Agent")

let p1 = "<p>route uri: " + uri + "</p>"
let p2 = "<p>route query: " + query + "</p>"
let p3 = "<p>" + testHeader + "</p>"
let body = "<!DOCTYPE html><html><body>" + p1 + p2 + p3 + "</body></html>"

export default {
    status: 200,
    headers: {
        "Content-type": "text/html",
        "Custom-Test": "test header",
    },
    body
}
```


deploy/ping_lua.lua:
```lua
local _M = {}

function _M.run()
    local headers = { ["Content-type"] = "text/html", ["Custom"] = "test lua" }

    local function hello_text()
        coroutine.yield("<html><body>")
        coroutine.yield("<p>Hello Wsapi!</p>")
        coroutine.yield("</body></html>")
    end

    return 200, headers, coroutine.wrap(hello_text)
end

return _M
```

deploy/ping_static.html:
```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport"
        content="width=device-width, user-scalable=no, initial-scale=1.0, maximum-scale=1.0, minimum-scale=1.0">
  <meta http-equiv="X-UA-Compatible" content="ie=edge">
  <title>Document</title>
</head>
<body>
this is a static page
</body>
</html>
```

启动服务，部署处理器：
```rust
use async_std::task;
use laputa::common;
use laputa::service::ScriptType;
use tide::http::Method;
use tide::Response;

fn main() -> common::BoxErrResult<()> {
    task::block_on(async {
        // 新建服务实例
        let mut server = laputa::new();

        // 注册静态文件处理器
        server.route_static(
            Method::GET,
            mime::TEXT_HTML_UTF_8,
            "static",
            "deploy/ping_static.html",
        )?;
        // 注册lua脚本处理器
        server.route_script(
            ScriptType::Lua,
            Method::GET,
            "lua",
            "deploy/ping_lua.lua",
        )?;
        // 注册js脚本处理器
        server.route_script(
            ScriptType::JavaScript,
            Method::GET,
            "js",
            "deploy/ping_javascript.js",
        )?;
        // 注册rust函数处理器
        server.route_fn(Method::GET, "fn", |_| async {
            Response::new(200).body_string("Hello World!".to_string())
        })?;
        
        // 启动服务
        server.start().await?;
        Ok(())
    })
}
```



