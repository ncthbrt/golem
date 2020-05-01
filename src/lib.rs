#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate futures;

use rusty_v8 as v8;
use std::convert::TryFrom;
use crate::isolate_core::{StartupData, IsolateCore};
use rusty_v8::InIsolate;

mod ops;
mod bindings;
mod isolate_core;
mod resources;
mod shared_queue;
mod any_error;
mod js_errors;


const SOURCE_CODE: &str = "
    function main(state, msg, ctx) {
        // await fetch('http://httpbin.org/post', { method: 'POST' }).catch(console.error).then(console.log);
        return state + msg;
    }
";

pub fn run_v8() {
    let mut isolate = IsolateCore::new(StartupData::None, false);
    let result = isolate.execute("test.js", SOURCE_CODE);
    match result {
        Result::Ok(()) => println!("{}", "Executed correctly"),
        Result::Err(err) => println!("{}", "Executed incorrectly"),
    };


    let v8_isolate = isolate.v8_isolate.as_mut().unwrap();

    let mut hs = v8::HandleScope::new(v8_isolate);
    let scope = hs.enter();
    let context = isolate.global_context.get(scope).unwrap();
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let function_name = v8::String::new(scope, "main").unwrap().into();
    let global = context.global(scope);

    let main = global.get(scope, context, function_name);

    if let Some(main) = main {
        let arg1 = v8::Local::from(v8::Number::new(scope, 1.0));
        let arg2 = v8::Local::from(v8::Number::new(scope, 2.0));
        let main: v8::Local<v8::Function> = v8::Local::<v8::Function>::try_from(main).unwrap();

        let this = v8::Object::new(scope);

        let result = main.call(scope, context, this.into(), &[arg1, arg2]).unwrap();
        let result = result.to_string(scope).unwrap();
        println!("result: {}", result.to_rust_string_lossy(scope));
    } else {
        println!("{}", "No main function");
    };


    // let platform = v8::new_default_platform().unwrap();
    // v8::V8::initialize_platform(platform);
    // v8::V8::initialize();
    //
    // let mut isolate = v8::Isolate::new(Default::default());
    // let mut handle_scope = v8::HandleScope::new(&mut isolate);
    // let scope = handle_scope.enter();
    //
    // let context = v8::Context::new(scope);
    // let mut context_scope = v8::ContextScope::new(scope, context);
    // let scope = context_scope.enter();
    //
    // // bindings::inject_bindings(scope, &context);
    //
    // let code = v8::String::new(scope, SOURCE_CODE).unwrap();
    // let mut script = v8::Script::compile(scope, context, code, None).unwrap();
    // script.run(scope, context).unwrap();
    // let global = context.global(scope);
    // let function_name = v8::String::new(scope, "main").unwrap();
    //
    // let main = global
    //     .get(scope, context, v8::Local::from(function_name))
    //     .unwrap();
    // let main: v8::Local<v8::Function> = v8::Local::<v8::Function>::try_from(main).unwrap();
    // let global: v8::Local<v8::Value> = context.global(scope).into();
    // let arg1 = v8::Local::from(v8::Number::new(scope, 1.0));
    // let arg2 = v8::Local::from(v8::Number::new(scope, 2.0));
    // let result = main.call(scope, context, global, &[arg1, arg2]).unwrap();
    //
    // // let result = result.to_string(scope).unwrap();
    // // println!("result: {}", result.to_rust_string_lossy(scope));
    //
    //
    // if result.is_object() && (result.is_async_function() || result.is_promise()) {
    //     println!("{}", "Result is async");
    // } else {
    //     println!("{}", "not a promise");
    // };
}

pub mod controllers;
