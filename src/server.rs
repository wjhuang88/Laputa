use crate::common;
use crate::service::Service;
use async_std::fs;
use bytes::Bytes;
use futures::SinkExt;

use async_std::prelude::*;

pub struct Server;

impl Server {
    pub async fn start(self) -> common::Result<()> {
        let lua_tx = crate::script::lua_engine::start()?;
        let state = common::ServiceState::new();
        let mut app = tide::with_state(state);
        app.at("/").get(move |_| {
            let mut tx = lua_tx.clone();
            common::spawn_and_log_error(async move {
                tx.send(Bytes::from(&b"test channel"[..])).await?;
                Ok(())
            });
            async { "Hello, world!" }
        });
        app.listen("127.0.0.1:8080").await?;
        Ok(())
    }

    pub fn register(service: Service) {
        match service {
            Service::Lua(router, file_path) => {
                // TODO: add lua handler
            }
            Service::JavaScript(router, file_path) => {
                // TODO: add javascript handler
            }
            _ => { /* do nothing. */ }
        }
    }
}