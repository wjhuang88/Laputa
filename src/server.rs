use crate::common;
use crate::service::Service;
use bytes::{BufMut, Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use tide::Response;

pub struct Server<'s> {
    pub(crate) route_map: HashMap<&'s str, Service<'s>>,
}

impl<'s> Server<'s> {
    pub async fn start(self) -> common::Result<()> {
        let lua_tx = crate::script::lua_engine::start()?;
        let state = common::ServiceState::new();
        let mut app = tide::with_state(state);
        let route_map = self.route_map;
        for (_k, v) in route_map.iter() {
            let engine_tx = lua_tx.clone();
            match v {
                &Service::Lua(route, path) => {
                    log::info!("Set router: /{}, for lua script form {}", route, path);
                    let path = std::path::PathBuf::from(path);
                    app.at(route).all(move |_| {
                        let path = path.clone();
                        let mut engine_tx = engine_tx.clone();
                        async {
                            let source_vec = async_std::fs::read(path).await;
                            if let Err(e) = source_vec {
                                log::error!("Error in task: {}", e);
                                return Response::new(500);
                            }
                            let source_vec = source_vec.unwrap();
                            let (result_tx, mut result_rx) = common::make_channel::<Bytes>();
                            common::spawn_and_log_error(async move {
                                let event = common::ScriptEvent {
                                    sender: result_tx.clone(),
                                    source: Bytes::from(source_vec),
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
                            let result_str =
                                String::from_utf8(result.to_vec()).unwrap_or(String::new());
                            Response::new(200).body_string(result_str)
                        }
                    });
                }
                _ => {}
            }
        }
        app.at("/").get(move |_| async { "hello" });
        app.listen("127.0.0.1:8080").await?;
        Ok(())
    }

    pub fn register(&mut self, service: Service<'s>) {
        let map = &mut self.route_map;
        let (route, serv) = service.get_route();
        map.insert(route, serv);
    }
}
