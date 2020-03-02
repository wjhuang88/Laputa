use crate::common;
use crate::common::{ScriptEvent, Sender};
use crate::service::Service;
use bytes::{BufMut, Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use tide::Response;

pub struct Server<'s> {
    pub(crate) route_map: HashMap<&'s str, Service<'s>>,
}

async fn script_handle(path: std::path::PathBuf, mut engine_tx: Sender<ScriptEvent>) -> Response {
    let source_vec = async_std::fs::read(path.clone()).await;
    if let Err(e) = source_vec {
        log::error!("Error in task: {}", e);
        return Response::new(404);
    }
    let source_vec = source_vec.unwrap();
    let (result_tx, mut result_rx) = common::make_channel::<Bytes>();
    common::spawn_and_log_error(async move {
        let event = ScriptEvent {
            sender: result_tx.clone(),
            source: Bytes::from(source_vec),
            name: path.to_string_lossy().to_string(),
        };
        engine_tx.send(event).await?;
        Ok(())
    });
    let mut result = BytesMut::new();
    while let Some(bytes) = result_rx.next().await {
        for byte in bytes {
            result.put_u8(byte);
        }
    }
    let result_str = String::from_utf8(result.to_vec()).unwrap_or(String::new());
    Response::new(200).body_string(result_str)
}

impl<'s> Server<'s> {
    pub async fn start(self) -> common::Result<()> {
        let state = common::ServiceState::new();
        let mut app = tide::with_state(state);
        let mut route_map = self.route_map;
        if !route_map.contains_key("/") {
            log::info!("Router for index('/') not found, set to default.");
            app.at("/").get(move |_| async {
                let head_style = format!("h1 {{ {};{};{};{}; }}", "font-size: 1.5em", "line-height: 2em", "margin-left: 1em", "color: cornflowerblue");
                let banner_style = format!("pre {{ {}; }}", "color: darkgray");
                let html_style = format!("html,body {{ {};{};{}; }}", "height: 100%", "width: 100%", "background: whitesmoke");
                let body_style = format!("body {{ {};{};{}; }}", "display: flex", "flex-direction: column", "align-items: center");
                let style = format!("\n{}\n{}\n{}\n{}\n", head_style, banner_style, html_style, body_style);
                let html = format!(
                    "<!DOCTYPE html>\n<html>\n<head>\n<title>Welcome to Laputa!</title>\n<style>{}</style>\n</head>\n<body>\n<h1>~~ Welcome to Laputa ~~</h1>\n<pre>{}</pre>\n</body>\n</html>",
                    style,
                    crate::logger_config::BANNER
                );
                Response::new(200).body_string(html).set_mime(mime::TEXT_HTML)
            });
        }
        let mut lua_tx: Option<Sender<ScriptEvent>> = None;
        let mut js_tx: Option<Sender<ScriptEvent>> = None;
        for (_k, v) in route_map.iter() {
            // handle services routers
            match v {
                &Service::JavaScript(route, path) => {
                    if js_tx.is_none() {
                        log::info!("Found javascript code routers");
                        js_tx.replace(crate::script::js_engine::start()?);
                    }
                    log::info!("Set router: /{} for javascript code form {}", route, path);
                    let path = std::path::PathBuf::from(path);
                    let engine_tx = js_tx.clone().unwrap().clone();
                    app.at(route).all(move |_| {
                        let path = path.clone();
                        script_handle(path, engine_tx.clone())
                    });
                }
                &Service::Lua(route, path) => {
                    if lua_tx.is_none() {
                        log::info!("Found lua code routers");
                        lua_tx.replace(crate::script::lua_engine::start()?);
                    }
                    log::info!("Set router: /{} for lua code form {}", route, path);
                    let path = std::path::PathBuf::from(path);
                    let engine_tx = lua_tx.clone().unwrap().clone();
                    app.at(route).all(move |_| {
                        let path = path.clone();
                        script_handle(path, engine_tx.clone())
                    });
                }
                &Service::Static(route, path) => {
                    log::info!("Set router: /{} for static file {}", route, path);
                    let path = std::path::PathBuf::from(path);
                    app.at(route).get(move |_| {
                        let path = path.clone();
                        async {
                            let content = async_std::fs::read_to_string(path).await;
                            if let Err(e) = content {
                                log::error!("Error in load static file: {}", e);
                                return Response::new(404);
                            }
                            let content = content.unwrap();
                            Response::new(200)
                                .body_string(content)
                                .set_mime(mime::TEXT_HTML)
                        }
                    });
                }
                _ => {}
            }
        }
        app.listen("127.0.0.1:8080").await?;

        log::info!("Shutting down server");
        route_map.clear();
        drop(route_map);
        drop(lua_tx);
        Ok(())
    }

    pub fn register(&mut self, service: Service<'s>) {
        let map = &mut self.route_map;
        let (mut route, serv) = service.get_route();
        if route.starts_with('/') {
            route = &route[1..]
        }
        map.insert(route, serv);
    }
}
