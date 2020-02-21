use std::sync::Mutex;
use actix::{Actor, Context as ActixCtx, Handler};
use crate::script::{ScriptMessage, ScriptHandler, DataType};
use std::io;
use rusty_v8 as v8;
use std::convert::TryInto;
use rusty_v8::ObjectTemplate;

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
        Ok(self.exec(msg.source, msg.file).to_string())
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

fn convert_value<'s>(value: Option<v8::Local<v8::Value>>, scope: &mut impl v8::ToLocal<'s>) -> DataType {
    if let Some(value) = value {
        if value.is_undefined() {
            DataType::Undefined
        } else if value.is_boolean() {
            DataType::Boolean(value.is_true())
        } else if value.is_int32() {
            if let Some(int_value) = value.int32_value(scope) {
                DataType::Integer(int_value)
            } else {
                DataType::Undefined
            }
        } else if value.is_number() {
            if let Some(float_value) = value.number_value(scope) {
                DataType::Double(float_value)
            } else {
                DataType::Undefined
            }
        } else if value.is_string() {
            if let Some(value_str) = value.to_string(scope) {
                DataType::String(value_str.to_rust_string_lossy(scope))
            } else {
                DataType::Undefined
            }
        } else if value.is_function() {
            // TODO: implement
            let temp = value.to_string(scope).map(|v| v.to_rust_string_lossy(scope));
            DataType::String(temp.unwrap_or("".to_string()))
        } else if value.is_array() {
            let array: v8::Local<v8::Array> = value.try_into().unwrap();
            let len: u32 = array.length();
            let mut result: Vec<DataType> = Vec::new();
            for i in 0..len {
                let context = scope.get_current_context().unwrap();
                if let Some(item) = array.get_index(scope, context, i) {
                    result.push(convert_value(Some(item), scope))
                }
            }
            DataType::Array(result)
        } else if value.is_map() {
            // TODO: implement
            warn!("[JS]  Map value is not implemented yet");
            // let map: v8::Local<v8::Map> = value.try_into().unwrap();
            let temp = value.to_string(scope).map(|v| v.to_rust_string_lossy(scope));
            DataType::String(temp.unwrap_or("".to_string()))
        } else if value.is_set() {
            // TODO: implement
            warn!("[JS]  Set value is not implemented yet");
            // let set: v8::Local<v8::Set> = value.try_into().unwrap();
            let temp = value.to_string(scope).map(|v| v.to_rust_string_lossy(scope));
            DataType::String(temp.unwrap_or("".to_string()))
        } else if value.is_object() {
            // TODO: implement
            warn!("[JS]  Object value is not implemented yet");
            // let object: v8::Local<v8::Object> = value.try_into().unwrap();
            let temp = value.to_string(scope).map(|v| v.to_rust_string_lossy(scope));
            DataType::String(temp.unwrap_or("".to_string()))
        } else {
            DataType::Undefined
        }
    } else {
        DataType::Null
    }
}

fn js_log(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue
) {
    let v8str = args.get(0).to_string(scope).unwrap_or(v8::String::empty(scope));
    let rstr = v8str.to_rust_string_lossy(scope);
    info!("[JS]  {}", rstr);
    rv.set(v8str.into())
}

fn js_error(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue
) {
    let v8str = args.get(0).to_string(scope).unwrap_or(v8::String::empty(scope));
    let rstr = v8str.to_rust_string_lossy(scope);
    error!("[JS]  {}", rstr);
    rv.set(v8str.into())
}

impl ScriptHandler for JavaScriptHandler {
    type SourceType = Vec<u8>;
    type ValueType = DataType;

    fn exec(&mut self, source: Self::SourceType, file_name: String) -> Self::ValueType {
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

        let console_log = v8::FunctionTemplate::new(scope, js_log);
        let console_error = v8::FunctionTemplate::new(scope, js_error);
        let console_key = v8::String::new_from_utf8(scope, "console".as_bytes(), v8::NewStringType::Normal).unwrap();
        let console_log_key = v8::String::new_from_utf8(scope, "log".as_bytes(), v8::NewStringType::Normal).unwrap();
        let console_error_key = v8::String::new_from_utf8(scope, "error".as_bytes(), v8::NewStringType::Normal).unwrap();
        let console_obj = ObjectTemplate::new(scope);
        console_obj.set_with_attr(console_log_key.into(), console_log.into(), v8::READ_ONLY + v8::DONT_ENUM + v8::DONT_DELETE);
        console_obj.set_with_attr(console_error_key.into(), console_error.into(), v8::READ_ONLY + v8::DONT_ENUM + v8::DONT_DELETE);
        let global = context.global(scope);
        let console_instance = console_obj.new_instance(scope, context).unwrap();
        global.set(context, console_key.into(), console_instance.into());

        let source = v8::String::new_from_utf8(scope, source.as_ref(), v8::NewStringType::Normal).unwrap();
        let name = v8::String::new(scope, file_name.as_str()).unwrap();
        let origin = script_origin(scope, name);
        let mut script = v8::Script::compile(scope, context, source, Some(&origin)).unwrap();
        let value = script.run(scope, context);
        value.map(|v| scope.escape(v));
        convert_value(value, scope)
    }
}