use std::collections::HashMap;

pub struct V8Modules {
    module_map: HashMap<String, v8::Global<v8::Module>>
}

pub struct V8Runtime {
    isolate: v8::OwnedIsolate,
    modules: V8Modules
}

impl V8Runtime {
    pub fn new() -> V8Runtime {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let isolate = v8::Isolate::new(Default::default());

        let module_list = V8Modules { module_map: HashMap::new() };
        V8Runtime { isolate, modules: module_list }
    }

    pub fn init_module(&mut self, name: impl AsRef<str>, source: impl AsRef<str>) {

        let hc = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Context::new(hc);
        let scope = &mut v8::ContextScope::new(hc, context);

        let name_str = name.as_ref();
        let source_str = source.as_ref();

        let code_string = v8::String::new(scope, source_str).unwrap();
        let name_string = v8::String::new(scope, name_str).unwrap();
        let map_url = v8::String::new(scope, "").unwrap();

        let origin = v8::ScriptOrigin::new(
            scope, name_string.into(),0, 0, false, 
            0, map_url.into(), false, false, true);
        let source = v8::script_compiler::Source::new(code_string, Some(&origin));

        let module = v8::script_compiler::compile_module(scope, source).unwrap();
        let _ = module.instantiate_module(scope, |_m1,_m2,_m3,_m4| None);
        let module_handle = v8::Global::<v8::Module>::new(scope, module);
        self.modules.module_map.insert(name_str.to_string(), module_handle);
    }

    pub fn execute(&mut self, name: impl AsRef<str>) -> Option<v8::Local<'_, v8::Value>> {
        let hc = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Context::new(hc);
        let scope = &mut v8::ContextScope::new(hc, context);

        let m = self.modules.module_map.get_mut(name.as_ref())?.open(scope);
        m.evaluate(scope)
    }
}