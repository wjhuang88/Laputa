use laputa_v8::V8Runtime;

fn main() {
    let mut runtime = V8Runtime::new();
    runtime.init_module("__laputa_runner", "'Hello' + ' World!'");
    let result = runtime.execute("__laputa_runner").unwrap();

    println!("result: {:?}", result.type_repr());
}