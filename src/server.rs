use crate::common::{BoxErrResult, ScriptEvent, ScriptResultEvent, Sender, ServiceState};
use crate::service::ScriptType;
use crate::{common, inner_pages};
use bytes::{BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use mimalloc::MiMalloc;
use mime::Mime;
use std::collections::HashMap;
use tide::{Endpoint, Response};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub struct Server {
    pub(crate) app: tide::Server<ServiceState>,
    sender_map: HashMap<ScriptType, Sender<ScriptEvent>>,
}

async fn script_handle(path: std::path::PathBuf, mut engine_tx: Sender<ScriptEvent>) -> Response {
    let (result_tx, mut result_rx) = common::make_channel::<ScriptResultEvent>();
    let path_clone = path.clone();
    common::spawn_and_log_error(async move {
        let event = ScriptEvent {
            sender: result_tx.clone(),
            location: path_clone.to_string_lossy().to_string(),
        };
        engine_tx.send(event).await?;
        Ok(())
    });
    let mut result = BytesMut::new();
    while let Some(r_event) = result_rx.next().await {
        if let Err(e) = r_event.result {
            log::error!("Error: {:?}, script: {:?}", e, path);
            return Response::new(500).body_string(e.to_string());
        }
        let bytes = r_event.result.unwrap();
        for byte in bytes {
            result.put_u8(byte);
        }
    }
    let result_str = String::from_utf8(result.to_vec()).unwrap_or(String::new());
    Response::new(200).body_string(result_str)
}

impl Server {
    pub(crate) fn new() -> Self {
        let state = common::ServiceState::new();
        let mut app = tide::with_state(state);
        app.at("/").get(move |_| async {
            let html = inner_pages::INDEX_PAGE.to_string();
            Response::new(200)
                .body_string(html)
                .set_mime(mime::TEXT_HTML)
        });
        Self {
            sender_map: HashMap::new(),
            app,
        }
    }

    pub async fn start(self) -> common::BoxErrResult<()> {
        self.app.listen("127.0.0.1:8080").await?;

        log::info!("Shutting down server");
        drop(self.sender_map);
        Ok(())
    }

    pub fn route_fn(
        &mut self,
        method: tide::http::Method,
        route: &str,
        ep: impl Endpoint<ServiceState>,
    ) -> BoxErrResult<()> {
        let mut route = route;
        if route.starts_with('/') {
            route = &route[1..]
        }
        log::info!("Route /{} for native function", route);
        self.app.at(route).method(method, ep);
        Ok(())
    }

    pub fn route_static(
        &mut self,
        method: tide::http::Method,
        mime: mime::Mime,
        route: &str,
        path: &str,
    ) -> BoxErrResult<()> {
        let mut route = route;
        if route.starts_with('/') {
            route = &route[1..]
        }
        log::info!("Router /{} for static file {}", route, path);
        let path = std::path::PathBuf::from(path);
        self.app.at(route).method(method, move |_| {
            let path = path.clone();
            let mime = mime.clone();
            async {
                let content = async_std::fs::read_to_string(path).await;
                if let Err(e) = content {
                    log::error!("Error in load static file: {}", e);
                    return Response::new(404);
                }
                let content = content.unwrap();
                Response::new(200).body_string(content).set_mime(mime)
            }
        });
        Ok(())
    }

    pub fn route_script(
        &mut self,
        script_type: ScriptType,
        method: tide::http::Method,
        route: &str,
        path: &str,
    ) -> BoxErrResult<()> {
        let mut route = route;
        if route.starts_with('/') {
            route = &route[1..]
        }
        let map = &mut self.sender_map;
        let tx = map
            .entry(script_type)
            .or_insert(script_type.start_engine()?);
        log::info!("Route /{} for {} code form {}", route, script_type, path);
        let path = std::path::PathBuf::from(path);
        let engine_tx = tx.clone();
        self.app.at(route).method(method, move |_| {
            let path = path.clone();
            script_handle(path, engine_tx.clone())
        });
        Ok(())
    }
}
