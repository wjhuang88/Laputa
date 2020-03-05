use crate::common::{BoxErrResult, ScriptEvent, Sender, ServiceState};
use serde::export::fmt::Error;
use serde::export::Formatter;
use std::future::Future;
use std::pin::Pin;
use tide::{Request, Response};

static TYPE_NAMES: [&'static str; 2] = ["lua", "javascript"];

#[repr(usize)]
#[derive(Copy, Debug, Hash, Eq)]
pub enum ScriptType {
    Lua = 0,
    JavaScript,
}

impl ScriptType {
    pub(crate) fn start_engine(&self) -> BoxErrResult<Sender<ScriptEvent>> {
        match self {
            ScriptType::Lua => crate::script::lua_engine::start(),
            ScriptType::JavaScript => crate::script::js_engine::start(),
        }
    }
}

impl PartialEq for ScriptType {
    fn eq(&self, other: &Self) -> bool {
        *self as usize == *other as usize
    }
}

impl Clone for ScriptType {
    #[inline]
    fn clone(&self) -> ScriptType {
        *self
    }
}

impl std::fmt::Display for ScriptType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(TYPE_NAMES[*self as usize])
    }
}
