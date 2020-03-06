use crate::common::{BoxErrResult, RequestData, ResponseData};
use crate::script::js_engine::bindings;
use bytes::{Buf, Bytes};
use lazy_static::*;
use log::*;
use rusty_v8 as v8;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::os::raw::c_void;
use std::sync::Mutex;

lazy_static! {
    static ref INIT_LOCK: Mutex<u32> = Mutex::new(0);
}

static ROOT_MOD: &str = "___root_module__";

#[must_use]
struct SetupGuard {}
impl Drop for SetupGuard {
    fn drop(&mut self) {
        // clean
    }
}

#[inline]
fn setup() -> SetupGuard {
    let mut g = INIT_LOCK.lock().unwrap();
    *g += 1;
    if *g == 1 {
        v8::V8::initialize_platform(v8::new_default_platform());
        v8::V8::initialize();
    }
    SetupGuard {}
}

struct JsError {
    message: String,
    cause: Option<Box<dyn std::error::Error>>,
}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let result = write!(f, "{}", self.message);
        if let Some(c) = &self.cause {
            let _result = result?;
            write!(f, ", cause: {}", c)
        } else {
            result
        }
    }
}

impl std::fmt::Debug for JsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let result = write!(f, "{:?}", self.message);
        if let Some(c) = &self.cause {
            let _result = result?;
            write!(f, ", cause: {:?}", c)
        } else {
            result
        }
    }
}

impl std::error::Error for JsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_ref().map(|cause| &**cause)
    }
}

#[allow(dead_code)]
pub(crate) struct ModuleInfo {
    pub(crate) main: bool,
    pub(crate) init: bool,
    pub(crate) name: String,
    pub(crate) id: i32,
    pub(crate) handle: v8::Global<v8::Module>,
    pub(crate) imports: Box<Vec<String>>,
}

impl ModuleInfo {
    fn set_init(&mut self, is_init: bool) {
        self.init = is_init;
    }
}

pub(crate) struct Modules {
    mod_map: HashMap<i32, ModuleInfo>,
    name_map: HashMap<String, i32>,
}

pub(crate) struct Isolate {
    pub(crate) v8_isolate: v8::OwnedIsolate,
    pub(crate) global_context: v8::Global<v8::Context>,
    pub(crate) modules: Modules,
    pub(crate) pending_promise_exceptions: HashMap<i32, v8::Global<v8::Value>>,
}

impl Isolate {
    pub fn new() -> Box<Self> {
        let _setup_guard = setup();
        let mut params = v8::Isolate::create_params();
        params.set_array_buffer_allocator(v8::new_default_allocator());
        let mut v8_isolate = v8::Isolate::new(params);
        v8_isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 10);
        v8_isolate.set_promise_reject_callback(promise_reject_callback);
        let modules = Modules {
            mod_map: HashMap::new(),
            name_map: HashMap::new(),
        };
        let mut global_context = v8::Global::<v8::Context>::new();
        let mut hs = v8::HandleScope::new(&mut v8_isolate);
        let scope = hs.enter();
        let context = bindings::init_context(scope);
        global_context.set(scope, context);
        let pending_promise_exceptions = HashMap::new();
        let my_isolate = Self {
            v8_isolate,
            modules,
            global_context,
            pending_promise_exceptions,
        };
        let mut boxed_isolate = Box::new(my_isolate);
        {
            let isolate_ptr = Box::into_raw(boxed_isolate);
            boxed_isolate = unsafe { Box::from_raw(isolate_ptr) };
            unsafe {
                let v8_isolate = &mut boxed_isolate.v8_isolate;
                v8_isolate.set_data(0, isolate_ptr as *mut c_void)
            };
        }
        boxed_isolate
    }

    pub fn load_module_from_bytes(
        &mut self,
        source: Bytes,
        name: String,
        is_main: bool,
    ) -> BoxErrResult<i32> {
        match self.modules.name_map.get(&name).map(|id| *id) {
            Some(id) if name.ne(ROOT_MOD) => {
                debug!("[JS]  Module {} was already loaded", name);
                return Ok(id);
            }
            Some(id) if name.eq(ROOT_MOD) => {
                let m = &mut self.modules;
                m.name_map.remove(&name);
                m.mod_map.remove(&id);
            }
            _ => {}
        }

        let v8_isolate = self.v8_isolate.borrow_mut();
        let mut hs = v8::HandleScope::new(v8_isolate);
        let scope = hs.enter();
        let context = self.global_context.get(scope).unwrap();
        let mut cs = v8::ContextScope::new(scope, context);
        let scope = cs.enter();

        let name_str = v8::String::new(scope, &name).unwrap();
        let source_str =
            v8::String::new_from_utf8(scope, source.bytes(), v8::NewStringType::Normal).unwrap();
        let origin = module_origin(scope, name_str);
        let source_v8 = v8::script_compiler::Source::new(source_str, &origin);

        let mut try_catch = v8::TryCatch::new(scope);
        let tc = try_catch.enter();

        let maybe_module = v8::script_compiler::compile_module(scope, source_v8);

        if tc.has_caught() {
            assert!(maybe_module.is_none());
            if let Some(err) = tc.message() {
                let err_ref = &err;
                print_error(scope, err_ref);
            }
            let err_struct = JsError {
                message: format!("Cannot compile module: {}", name),
                cause: None,
            };
            return Err(err_struct.into());
        }

        let module = maybe_module.unwrap();
        let id = module.get_identity_hash();

        let mut imports: Vec<String> = vec![];
        let imports_num = module.get_module_requests_length();
        if imports_num > 0 {
            for i in 0..imports_num {
                let import_specifier = module.get_module_request(i).to_rust_string_lossy(scope);
                imports.push(import_specifier);
            }
        };
        let imports = Box::new(imports);

        let mut handle = v8::Global::<v8::Module>::new();
        handle.set(scope, module);
        let mod_info = ModuleInfo {
            main: is_main,
            init: false,
            id,
            name: name.clone(),
            handle,
            imports,
        };
        self.modules.mod_map.insert(id, mod_info);
        self.modules.name_map.insert(name.clone(), id);
        Ok(id)
    }

    pub async fn load_module(&mut self, specifier: String, is_main: bool) -> BoxErrResult<i32> {
        if let Some(id) = self.modules.name_map.get(&specifier) {
            debug!("[JS]  Module {} was already loaded", specifier);
            Ok(*id)
        } else {
            debug!("[JS]  Module {} will be loaded", specifier);
            let source = resolve_spec(specifier.clone()).await?;
            self.load_module_from_bytes(source, specifier, is_main)
        }
    }

    async fn load_module_vec(
        &mut self,
        specifiers: Vec<String>,
        is_main: bool,
    ) -> Vec<BoxErrResult<i32>> {
        let futs = specifiers.iter().map(|spec| {
            let iso = self as *mut Isolate;
            // TODO: maybe the unsafe block could make things wrong because of the more then
            // one task will occupied the isolate object and change the maps.
            unsafe { (*iso).load_module(spec.to_string(), is_main) }
        });
        futures::future::join_all(futs).await
    }

    pub async fn instantiate_module(&mut self, id: i32) -> BoxErrResult<()> {
        let module = {
            let modules = &mut self.modules;
            let map = &mut modules.mod_map;
            map.get_mut(&id)
        };
        if module.is_none() {
            let err = JsError {
                message: format!(
                    "Js module(id: {}) not found in memory, please try to load it",
                    id
                ),
                cause: None,
            };
            return Err(Box::new(err));
        }

        let module = module.unwrap();
        let name = &module.name.clone();
        if module.init && name.ne(ROOT_MOD) {
            debug!("[JS]  Module {} has been instantiated", name);
            return Ok(());
        }

        debug!("[JS]  Module {} will be instantiated", name);

        {
            // borrowed mut self must release after use, so let's make a temporary var here
            let module = {
                let modules = &mut self.modules;
                let map = &mut modules.mod_map;
                map.get_mut(&id)
            }
            .unwrap();
            let specs = module.imports.clone();
            if specs.len() > 0 {
                debug!("[JS]  Begin to handle imported modules for module {}", name);
                self.load_module_vec(*specs, false).await;
            }
        }

        let v8_isolate = &mut self.v8_isolate;

        let mut hs = v8::HandleScope::new(v8_isolate);
        let scope = hs.enter();
        assert!(!self.global_context.is_empty());
        let context = self.global_context.get(scope).unwrap();
        let mut cs = v8::ContextScope::new(scope, context);
        let scope = cs.enter();

        let mut try_catch = v8::TryCatch::new(scope);
        let tc = try_catch.enter();

        // another temporary var here
        let module = {
            let modules = &mut self.modules;
            let map = &mut modules.mod_map;
            map.get_mut(&id)
        }
        .unwrap();
        let mut real_module = module.handle.get(scope).unwrap();
        if real_module.get_status() == v8::ModuleStatus::Errored {
            let exception = real_module.get_exception();
            let message = v8::Exception::create_message(scope, exception);
            print_error(scope, &message);
            let err_str = message.get(scope).to_rust_string_lossy(scope);
            let err = JsError {
                message: err_str,
                cause: None,
            };
            return Err(Box::new(err));
        }

        let result = real_module.instantiate_module(context, module_resolve_callback);

        if result.is_none() || !result.unwrap() {
            let cause = if let Some(err) = tc.message() {
                let err_ref = &err;
                print_error(scope, err_ref);
                Some(Box::new(JsError {
                    message: err.get(scope).to_rust_string_lossy(scope),
                    cause: None,
                }) as Box<dyn std::error::Error>)
            } else {
                None
            };
            let err = JsError {
                message: format!("Module {} cannot be instantiated", name),
                cause,
            };
            return Err(Box::new(err));
        }
        module.set_init(true);

        Ok(())
    }

    fn module_evaluate(&mut self, mod_id: i32) -> BoxErrResult<ResponseData> {
        let v8_isolate = &mut self.v8_isolate;
        let mut hs = v8::HandleScope::new(v8_isolate);
        let scope = hs.enter();
        assert!(!self.global_context.is_empty());
        let context = self.global_context.get(scope).unwrap();
        let mut cs = v8::ContextScope::new(scope, context);
        let scope = cs.enter();

        let module = {
            let modules = &mut self.modules;
            let map = &mut modules.mod_map;
            map.get_mut(&mod_id).expect("ModuleInfo not found")
        };
        let name = module.name.clone();
        let mut real_module = module.handle.get(scope).expect("Empty module handle");
        let mut status = real_module.get_status();
        if status == v8::ModuleStatus::Instantiated {
            let result = real_module.evaluate(scope, context);
            // Update status after evaluating.
            status = real_module.get_status();
            if result.is_some() {
                assert!(
                    status == v8::ModuleStatus::Evaluated || status == v8::ModuleStatus::Errored
                );
            } else {
                assert_eq!(status, v8::ModuleStatus::Errored);
            }
            match status {
                v8::ModuleStatus::Evaluated => {
                    let result = result.unwrap();
                    bindings::make_response(scope, context, result)
                }
                v8::ModuleStatus::Errored => {
                    let exception = real_module.get_exception();
                    let message = v8::Exception::create_message(scope, exception);
                    print_error(scope, &message);
                    let err_str = message.get(scope).to_rust_string_lossy(scope);
                    let err = JsError {
                        message: err_str,
                        cause: None,
                    };
                    Err(Box::new(err))
                }
                other => {
                    panic!("Unexpected module status {:?}", other);
                }
            }
        } else {
            let err = JsError {
                message: format!("Module {} is not instantiated", name),
                cause: None,
            };
            Err(Box::new(err))
        }
    }

    pub async fn module_execute(
        &mut self,
        specifier: String,
        request: RequestData,
    ) -> BoxErrResult<ResponseData> {
        let v8_isolate = &mut self.v8_isolate;
        let mut hs = v8::HandleScope::new(v8_isolate);
        let scope = hs.enter();
        assert!(!self.global_context.is_empty());
        let context = self.global_context.get(scope).unwrap();
        let mut cs = v8::ContextScope::new(scope, context);
        let scope = cs.enter();

        let _ = bindings::make_request(scope, context, request)?;

        let cache_map = &mut self.modules;
        if let Some(&id) = cache_map.name_map.get(&specifier) {
            cache_map.name_map.remove(&specifier);
            cache_map.mod_map.remove(&id);
        }
        let root_mod = format!("import m from \"{}\"\nm", specifier);
        let root_bytes = Bytes::from(root_mod);
        let root_id = self.load_module_from_bytes(root_bytes, ROOT_MOD.to_string(), true)?;
        let _ = self.instantiate_module(root_id).await?;
        self.module_evaluate(root_id)
    }
}

fn module_resolve_callback<'s>(
    context: v8::Local<'s, v8::Context>,
    specifier: v8::Local<'s, v8::String>,
    referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {
    let mut scope = v8::CallbackScope::new_escapable(context);
    let mut scope = v8::EscapableHandleScope::new(scope.enter());
    let scope = scope.enter();

    let specifier_str = specifier.to_rust_string_lossy(scope);
    let referrer_id = referrer.get_identity_hash();

    let my_isolate: &mut Isolate = unsafe { &mut *(scope.isolate().get_data(0) as *mut Isolate) };

    let specifier_id =
        async_std::task::block_on(my_isolate.load_module(specifier_str.clone(), false));
    if let Err(e) = specifier_id {
        error!(
            "[JS]  Cannot resolve module: {}, cause: {}",
            specifier_str, e
        );
        return Some(referrer);
    }
    let specifier_id = specifier_id.unwrap();

    let modules = &my_isolate.modules;
    let referrer_name = &modules.mod_map.get(&referrer_id).unwrap().name;
    debug!(
        "[JS]  Handled imported module {} for {}",
        specifier_str, referrer_name
    );
    let module = modules.mod_map.get(&specifier_id).unwrap();
    module.handle.get(scope).map(|m| scope.escape(m))
}

async fn resolve_spec(specifier: String) -> BoxErrResult<Bytes> {
    if specifier.starts_with("http://") || specifier.starts_with("https://") {
        let data = reqwest::get(&specifier).await?.bytes().await?;
        Ok(data)
    } else {
        let file_real = if specifier.starts_with("file:///") {
            &specifier[7..]
        } else {
            &specifier
        };
        let bytes = async_std::fs::read(file_real).await?;
        Ok(Bytes::from(bytes))
    }
}

fn module_origin<'a>(
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

fn print_error<'a>(scope: &mut impl v8::ToLocal<'a>, err: &v8::Local<v8::Message>) {
    // TODO: this function need to be rewrite with full exception handle (not only message)
    let err_str = err.get(scope).to_rust_string_lossy(scope);
    let column_start = err.get_start_column();
    let column_end = err.get_end_column();
    let name = err
        .get_script_resource_name(scope)
        .map_or("<unknown>".to_string(), |name| {
            name.to_string(scope)
                .unwrap_or(v8::String::empty(scope))
                .to_rust_string_lossy(scope)
        });
    let row = err
        .get_line_number(scope.get_current_context().unwrap())
        .map_or("<unknown>".to_string(), |row| format!("{}", row));
    error!(
        "[JS]  Compile error: {} -- at [line {}: column {} - column {}] in {}",
        err_str, row, column_start, column_end, name
    );
    if let Some(stack) = err.get_stack_trace(scope) {
        let count = stack.get_frame_count();
        for i in 0..count {
            let frame = stack.get_frame(scope, i);
            if let Some(frame) = frame {
                let column = frame.get_column();
                let row = frame.get_line_number();
                let function = frame.get_function_name(scope);
                let function = function.map(|f| f.to_rust_string_lossy(scope));
                let function = function.unwrap_or("<unknown>".to_string());
                let script = frame
                    .get_script_name_or_source_url(scope)
                    .map(|s| s.to_rust_string_lossy(scope));
                let script = script.unwrap_or("<unknown>".to_string());
                error!(
                    "[JS]  function {} at {} - [{}:{}]",
                    function, script, row, column
                );
            }
        }
    }
}

pub extern "C" fn promise_reject_callback(message: v8::PromiseRejectMessage) {
    let mut cbs = v8::CallbackScope::new(&message);
    let mut hs = v8::HandleScope::new(cbs.enter());
    let scope = hs.enter();

    let laputa_isolate: &mut Isolate =
        unsafe { &mut *(scope.isolate().get_data(0) as *mut Isolate) };

    let context = laputa_isolate.global_context.get(scope).unwrap();
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let promise = message.get_promise();
    let promise_id = promise.get_identity_hash();

    match message.get_event() {
        v8::PromiseRejectEvent::PromiseRejectWithNoHandler => {
            let error = message.get_value();
            let mut error_global = v8::Global::<v8::Value>::new();
            error_global.set(scope, error);
            laputa_isolate
                .pending_promise_exceptions
                .insert(promise_id, error_global);
        }
        v8::PromiseRejectEvent::PromiseHandlerAddedAfterReject => {
            if let Some(mut handle) = laputa_isolate
                .pending_promise_exceptions
                .remove(&promise_id)
            {
                handle.reset(scope);
            }
        }
        v8::PromiseRejectEvent::PromiseRejectAfterResolved => {}
        v8::PromiseRejectEvent::PromiseResolveAfterResolved => {
            // nothing
        }
    };
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_js_error() {
        let source_err = JsError {
            message: "Source".to_string(),
            cause: None,
        };
        let err = JsError {
            message: "Current".to_string(),
            cause: Some(Box::new(source_err)),
        };

        assert_eq!(format!("{}", err), "Current, cause: Source".to_string())
    }
}
