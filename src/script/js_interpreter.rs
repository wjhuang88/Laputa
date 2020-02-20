use std::sync::Mutex;
use actix::{Actor, Context as ActixCtx, Handler};
use crate::script::{ScriptMessage, ScriptHandler};
use std::io;
use rusty_v8 as v8;

lazy_static! {
  static ref INIT_LOCK: Mutex<u32> = Mutex::new(0);
}

#[must_use]
struct SetupGuard {}
impl Drop for SetupGuard {
    fn drop(&mut self) {
        // TODO shutdown process cleanly.
    }
}

fn setup() -> SetupGuard {
    let mut g = INIT_LOCK.lock().unwrap();
    *g += 1;
    if *g == 1 {
        v8::V8::initialize_platform(v8::new_default_platform());
        v8::V8::initialize();
    }
    SetupGuard {}
}

pub struct JavaScriptHandler {
    isolate: v8::OwnedIsolate
}

impl JavaScriptHandler {
    pub(crate) fn new() -> Self {
        let _setup_guard = setup();
        let mut params = v8::Isolate::create_params();
        params.set_array_buffer_allocator(v8::new_default_allocator());
        let isolate = v8::Isolate::new(params);
        Self {
            isolate
        }
    }
}

impl Actor for JavaScriptHandler {
    type Context = ActixCtx<Self>;
}

impl Handler<ScriptMessage> for JavaScriptHandler {
    type Result = Result<String, io::Error>;

    fn handle(&mut self, msg: ScriptMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.exec(msg.source, msg.file)
    }
}

pub fn script_origin<'a>(
    s: &mut impl v8::ToLocal<'a>,
    resource_name: v8::Local<'a, v8::String>,
) -> v8::ScriptOrigin<'a> {
    let resource_line_offset = v8::Integer::new(s, 0);
    let resource_column_offset = v8::Integer::new(s, 0);
    let resource_is_shared_cross_origin = v8::Boolean::new(s, false);
    let script_id = v8::Integer::new(s, 123);
    let source_map_url = v8::String::new(s, "source_map_url").unwrap();
    let resource_is_opaque = v8::Boolean::new(s, true);
    let is_wasm = v8::Boolean::new(s, false);
    let is_module = v8::Boolean::new(s, false);
    v8::ScriptOrigin::new(
        resource_name.into(),
        resource_line_offset,
        resource_column_offset,
        resource_is_shared_cross_origin,
        script_id,
        source_map_url.into(),
        resource_is_opaque,
        is_wasm,
        is_module,
    )
}

impl ScriptHandler for JavaScriptHandler {
    type SourceType = Vec<u8>;
    type ValueType = String;

    fn exec(&mut self, source: Self::SourceType, file_name: String) -> io::Result<Self::ValueType> {
        let mut hs = v8::HandleScope::new(&mut self.isolate);
        // Enter handler scope
        let scope = hs.enter();
        let context = v8::Context::new(scope);
        let mut cs = v8::ContextScope::new(scope, context);
        // Enter context scope
        let scope = cs.enter();
        let mut hs = v8::EscapableHandleScope::new(scope);
        // Enter escaped handler scope
        let scope = hs.enter();

        let source = v8::String::new_from_utf8(scope, source.as_ref(), v8::NewStringType::Normal).unwrap();
        let name = v8::String::new(scope, file_name.as_str()).unwrap();
        let origin = script_origin(scope, name);
        let mut script = v8::Script::compile(scope, context, source, Some(&origin)).unwrap();
        let r = script.run(scope, context);
        r.map(|v| scope.escape(v));
        let result = r.unwrap().to_string(scope).unwrap().to_rust_string_lossy(scope);
        Ok(result)
    }
}