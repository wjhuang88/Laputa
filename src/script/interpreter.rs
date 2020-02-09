use crate::core::System;
use std::sync::Arc;

pub trait ScriptHandler {
    fn register_scripts(&self, system: Arc<System>, dir: &str);
}