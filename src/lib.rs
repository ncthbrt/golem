use rusty_v8 as v8;
use std::convert::TryFrom;
use std::println;

mod bindings;

const SOURCE_CODE: &str = "
    function main(state, msg, ctx) {
        console.log(\"Hello World!\", { size: 32 }, [{a: 1},2,3,4]);
        console.error(\"Something has gone wrong!!\", { size: 32 }, [{a: 1},2,3,4]);
        return state + msg;
    }
";

pub fn run_v8() {
    let platform = v8::new_default_platform().unwrap();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let mut isolate = v8::Isolate::new(Default::default());
    let mut handle_scope = v8::HandleScope::new(&mut isolate);
    let scope = handle_scope.enter();

    let context = v8::Context::new(scope);
    let mut context_scope = v8::ContextScope::new(scope, context);
    let scope = context_scope.enter();

    bindings::inject_bindings(scope, &context);

    let code = v8::String::new(scope, SOURCE_CODE).unwrap();
    println!("javascript code: {}", code.to_rust_string_lossy(scope));
    let mut script = v8::Script::compile(scope, context, code, None).unwrap();
    script.run(scope, context).unwrap();
    let global = context.global(scope);
    let function_name = v8::String::new(scope, "main").unwrap();

    let main = global
        .get(scope, context, v8::Local::from(function_name))
        .unwrap();
    let main: v8::Local<v8::Function> = v8::Local::<v8::Function>::try_from(main).unwrap();
    let global: v8::Local<v8::Value> = context.global(scope).into();
    let arg1 = v8::Local::from(v8::Number::new(scope, 1.0));
    let arg2 = v8::Local::from(v8::Number::new(scope, 2.0));
    let result = main.call(scope, context, global, &[arg1, arg2]).unwrap();

    let result = result.to_string(scope).unwrap();
    println!("result: {}", result.to_rust_string_lossy(scope));
}

pub mod controllers;
