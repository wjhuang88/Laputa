use std::sync::Mutex;
use actix::{Actor, Context as ActixCtx, Handler};
use crate::script::{ScriptMessage, ScriptHandler, DataType};
use std::io;
use rusty_v8 as v8;
use std::convert::TryInto;
use rusty_v8::ObjectTemplate;
use std::collections::HashMap;
use futures::executor::block_on;
use std::cell::RefCell;

lazy_static! {
    static ref INIT_LOCK: Mutex<u32> = Mutex::new(0);
}

thread_local! {
    static MODULE_MAP: RefCell<HashMap<String, Box<Vec<u8>>>> = RefCell::new(HashMap::new());
}

#[must_use]
struct SetupGuard {}
impl Drop for SetupGuard {
    fn drop(&mut self) {
        MODULE_MAP.with(|map| map.borrow_mut().clear());
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
        info!("Starting js engine(v8) for first invoking");
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

#[inline]
fn print_error<'a>(scope: &mut impl v8::ToLocal<'a>, err: &v8::Local<v8::Message>) {
    let err_str = err.get(scope).to_rust_string_lossy(scope);
    let column_start = err.get_start_column();
    let column_end = err.get_end_column();
    let name = err.get_script_resource_name(scope).map_or("<unknown>".to_string(), |name| name.to_string(scope).unwrap_or(v8::String::empty(scope)).to_rust_string_lossy(scope));
    let row = err.get_line_number(scope.get_current_context().unwrap()).map_or("<unknown>".to_string(), |row| format!("{}", row));
    error!("[JS]  Compile error: {} -- at [line {}: column {} - column {}] in {}", err_str, row, column_start, column_end, name);
    if let Some(stack) = err.get_stack_trace(scope) {
        let count = stack.get_frame_count();
        for i in 0..count {
            let frame = stack.get_frame(scope, i);
            if let Some(frame) = frame {
                let column = frame.get_column();
                let row = frame.get_line_number();
                let function = frame.get_function_name(scope);
                let function = function.unwrap_or(v8::String::empty(scope));
                let function = function.to_rust_string_lossy(scope);
                let script = frame.get_script_name_or_source_url(scope);
                let script = script.unwrap_or(v8::String::empty(scope));
                let script = script.to_rust_string_lossy(scope);
                error!("[JS]  function {} at {} - [{}:{}]", function, script, row, column);
            }
        }
    }
}

fn module_callback<'s>(
    context: v8::Local<'s, v8::Context>,
    specifier: v8::Local<'s, v8::String>,
    _referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {
    let mut cbs = v8::CallbackScope::new_escapable(context);
    let mut hs = v8::EscapableHandleScope::new(cbs.enter());
    let scope = hs.enter();
    let path = specifier.to_rust_string_lossy(scope);
    info!("[JS]  Loading imported module: {}", path);

    MODULE_MAP.with(|map| {
        if map.borrow().contains_key(&path) {
            if let Some(bytes) = map.borrow().get(&path) {
                let maybe_module = compile_module(scope,&**bytes, path);
                if let Some(mut module) = maybe_module {
                    module.instantiate_module(context, module_callback);
                }
                maybe_module
            } else {
                None
            }
        } else {
            warn!("[JS]  Imported module: {} not found in the map, something wrong happened at initializing process", path);
            None
        }
    })
}

async fn read_module(path: String) -> io::Result<Vec<u8>> {
    if path.starts_with("http://") || path.starts_with("https://") {
        use actix_web::client::Client;
        match Client::new().get(path).send().await {
            Ok(mut res) => {
                res.body().await.map(|b| b.to_vec()).map_err(|_e| io::Error::new(io::ErrorKind::Other, "File from network is empty"))
            },
            Err(_err) => Err(io::Error::new(io::ErrorKind::Other, "Network error"))
        }
    } else {
        let file_real = if path.starts_with("file:///") {
            &path[7..]
        } else {
            &path
        };
        std::fs::read(file_real)
    }
}

fn compile_module<'s>(scope: &'s mut impl v8::ToLocal<'s>, source: &Vec<u8>, name: String) -> Option<v8::Local<'s, v8::Module>> {
    let source = v8::String::new_from_utf8(scope, source.as_ref(), v8::NewStringType::Normal).unwrap();
    let name_local = v8::String::new(scope, name.as_str()).unwrap();
    let origin = script_origin(scope, name_local);
    let source = v8::script_compiler::Source::new(source, &origin);
    let compile = v8::script_compiler::compile_module(scope, source);

    let mut try_catch_scope = v8::TryCatch::new(scope);
    let try_catch = try_catch_scope.enter();
    if compile.is_none() {
        error!("[JS]  Cannot compile module: {}", name);
        if try_catch.has_caught() {
            if let Some(err) = try_catch.message() {
                let err_ref = &err;
                print_error(scope, err_ref);
            }
        }
        return None;
    }
    compile
}

async fn load_module_recursive<'s>(scope: &'s mut impl v8::ToLocal<'s>, source: Vec<u8>, name: String) {
    MODULE_MAP.with(|map| {
        if map.borrow().contains_key(&name) {
            info!("[JS]  Module: {} is in the map, skip", name);
            return;
        }
    });
    info!("[JS]  Loading module: {}", name);
    let mut hs = v8::EscapableHandleScope::new(scope);
    // Enter escaped handler scope
    let scope = hs.enter();

    let compile = compile_module(scope, &source, name.to_string());
    if compile.is_none() {
        return;
    }
    let module = compile.unwrap();
    MODULE_MAP.with(|map| {
        map.borrow_mut().insert(name.to_string(), Box::new(source.to_vec()));
    });
    for i in 0..module.get_module_requests_length() {
        let sub_file = module.get_module_request(i).to_rust_string_lossy(scope);
        match read_module(sub_file).await {
            Ok(bytes) => {load_module_recursive(scope, bytes, sub_file.to_string());},
            Err(err) => error!("[JS]  Cannot load module from {}, err: {}", sub_file, err)
        }
    }
}

struct ModuleInfo {
    main_module: String,
}
impl ModuleInfo {
    async fn load<'s>(scope: &'s mut impl v8::ToLocal<'s>, source: Vec<u8>, name: String) -> Option<ModuleInfo> {
        load_module_recursive(scope, source, name);
        Some(ModuleInfo {
            main_module: name.to_string(),
        })
    }

    pub fn init<'s>(&'s self, scope: &'s mut impl v8::ToLocal<'s>) -> bool {
        let maybe_bytes = MODULE_MAP.with(|map| { map.borrow().get(&self.main_module).map(|boxed| **boxed) });
        if let Some(bytes) = maybe_bytes {
            if let Some(mut m) = compile_module(scope, &bytes, self.main_module.to_string()) {
                m.instantiate_module(scope.get_current_context().unwrap(), module_callback).unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        }
    }
}

fn script_origin<'a>(
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
    let is_module = v8::Boolean::new(s, true);
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

        let module_info = ModuleInfo::load(scope, source, file_name);

        if let Some(mut module) = block_on(module_info) {
            module.init(scope);
        }
        // convert_value(result, scope)
        DataType::Undefined
    }
}