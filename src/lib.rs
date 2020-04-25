use rusty_v8 as v8;
use std::convert::TryFrom;
use std::println;

struct ActorDefintion {
    name: String,
    source_code: String,
    version: i32,
}

const SOURCE_CODE: &str = "
    function main(state, msg, ctx) {     
        console.log(\"Hello World!\");
        return state + msg;
    }
";

fn print(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) -> () {
    let arg_len = args.length();
    assert!(arg_len >= 0 && arg_len <= 2);
    let obj = args.get(0);
    let is_err_arg = args.get(1);
    let mut hs = v8::HandleScope::new(scope);
    let scope = hs.enter();
    let mut is_err = false;
    if arg_len == 2 {
        let int_val = is_err_arg
            .integer_value(scope)
            .expect("Unable to convert to integer");
        is_err = int_val != 0;
    };
    let mut try_catch = v8::TryCatch::new(scope);
    let _tc = try_catch.enter();
    let str_ = match obj.to_string(scope) {
        Some(s) => s,
        None => v8::String::new(scope, "").unwrap(),
    };
    if is_err {
        eprint!(
            "Hello err from js to rust: {}\n",
            str_.to_rust_string_lossy(scope)
        );
    } else {
        print!(
            "Hello from js to rust: {}\n",
            str_.to_rust_string_lossy(scope)
        );
    }
}

pub fn run_v8() {
    let platform = v8::new_default_platform();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let mut create_params = v8::Isolate::create_params();
    create_params.set_array_buffer_allocator(v8::new_default_allocator());
    let mut isolate = v8::Isolate::new(create_params);
    let mut handle_scope = v8::HandleScope::new(&mut isolate);
    let scope = handle_scope.enter();

    let context = v8::Context::new(scope);
    let mut context_scope = v8::ContextScope::new(scope, context);
    let scope = context_scope.enter();

    let global = context.global(scope);
    let console = v8::Object::new(scope);
    global.set(
        context,
        v8::String::new(scope, "console").unwrap().into(),
        console.into(),
    );
    let mut print_tmpl: v8::Local<v8::FunctionTemplate> = v8::FunctionTemplate::new(scope, print);
    let print_val = print_tmpl.get_function(scope, context).unwrap();
    console.set(
        context,
        v8::String::new(scope, "log").unwrap().into(),
        print_val.into(),
    );

    let code = v8::String::new(scope, SOURCE_CODE).unwrap();
    println!("javascript code: {}", code.to_rust_string_lossy(scope));
    let mut script = v8::Script::compile(scope, context, code, None).unwrap();
    script.run(scope, context).unwrap();
    let function_name = v8::String::new(scope, "main").unwrap();
    let global = context.global(scope);

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
