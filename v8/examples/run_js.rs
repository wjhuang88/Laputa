use laputa_v8::V8Runtime;

fn main() {
    let mut runtime = V8Runtime::new();
    runtime.import_module("__laputa_runner", include_str!("./helloworld.js"));
    let result = runtime.execute_module("__laputa_runner").unwrap();

    println!("result: {:?}", result);
}
