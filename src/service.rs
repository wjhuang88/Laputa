use crate::common::ServiceState;
use std::future::Future;
use tide::{Request, Response};

pub enum Service<'s> {
    Native(
        &'s str,
        &'s (dyn Fn(Request<ServiceState>) -> dyn Future<Output = Response>),
    ),
    Lua(&'s str, &'s str),
    JavaScript(&'s str, &'s str),
}

impl<'s> Service<'s> {
    pub fn get_route(self) -> (&'s str, Self) {
        match self {
            Service::Lua(router, _) => (router, self),
            Service::JavaScript(router, _) => (router, self),
            Service::Native(router, _) => (router, self),
        }
    }
}
