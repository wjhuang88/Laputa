use crate::common::ServiceState;
use std::future::Future;
use tide::{Request, IntoResponse, Response};
use std::process::Output;

pub enum Service<'s> {
    Native(&'s str, &'s (dyn Fn(Request<ServiceState>) -> dyn Future<Output=Response>)),
    Lua(&'s str, &'s async_std::path::Path),
    JavaScript(&'s str, &'s async_std::path::Path),
}