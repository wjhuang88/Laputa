mod core;
//mod log;
//mod plugin;
mod script;

#[macro_use]
extern crate log;
extern crate chrono;
extern crate env_logger;

#[macro_use]
extern crate lazy_static;

use std::path::PathBuf;
use std::collections::HashMap;
use std::{io, fs};
use actix::prelude::*;
use actix_web::{HttpServer, web, HttpRequest, App, HttpResponse, Responder};

use crate::script::*;
use std::borrow::Borrow;

lazy_static! {
    static ref JS_ACTOR: actix::Addr<JavaScriptHandler> = JavaScriptHandler::new().start();
    static ref LUA_ACTOR: actix::Addr<LuaHandler> = LuaHandler::new().start();

    static ref SCRIPT_MAP: HashMap<String, Box<ScriptFile>> = {
        let mut map: HashMap<String, Box<ScriptFile>> = HashMap::new();

        for file in list_dir("./deploy").unwrap() {
            if let Some(ext) = file.extension() {
                if ext.eq("js") {
                    let des = ScriptFile {file_path: file, script_type: "js".to_string()};
                    let route = des.route_path();
                    info!("Setting route path: '{}'", route);
                    map.insert(route, Box::new(des));
                } else if ext.eq("lua") {
                    let des = ScriptFile {file_path: file, script_type: "lua".to_string()};
                    let route = des.route_path();
                    info!("Setting route path: '{}'", route);
                    map.insert(route, Box::new(des));
                }
            }
        }
        map
    };
}

fn init_log() {
    use chrono::Local;
    use std::io::Write;

    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args()
            )
        })
        .init();

    info!("Logger config initialized.");
}

#[inline]
fn list_dir(path: &str) -> io::Result<Vec<PathBuf>> {
    fs::read_dir(path)?
        .map(|res| res.map(|e| e.path()))
        .collect()
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    init_log();

    async fn handle(req: HttpRequest) -> impl Responder {
        let name = req.path().trim_matches(|x| x == '/').to_string();
        let script: &ScriptFile = SCRIPT_MAP.get(&name).unwrap();
        let src = script.get_source().unwrap();
        let msg = ScriptMessage {request: RequestInfo {path: name.clone()}, source: src, file: script.get_file_name()};
        let result = match script.script_type.as_str() {
            "js" => JS_ACTOR.borrow().send(msg).await.unwrap().unwrap(),
            "lua" => LUA_ACTOR.borrow().send(msg).await.unwrap().unwrap(),
            _ => "".to_string()
        };
        HttpResponse::Ok().body(result)
    }

    HttpServer::new(move || {
        let mut app = App::new();
        for route in SCRIPT_MAP.keys() {
            app = app.route(route, web::route().to(handle));
        }
        app
    })
    .bind("127.0.0.1:8000")?
    .run().await
}
