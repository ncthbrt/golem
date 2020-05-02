#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

extern crate futures;

use rusty_v8 as v8;
use std::convert::TryFrom;
use crate::isolate_core::{StartupData, IsolateCore};
use crate::any_error::ErrBox;

mod ops;
mod bindings;
mod isolate_core;
mod resources;
mod shared_queue;
mod any_error;
mod js_errors;
mod golem_isolate;

const SOURCE_CODE: &str = "
    async function main(state, msg, ctx) {
        // await fetch('http://httpbin.org/post', { method: 'POST' }).catch(console.error).then(console.log);
        console.log('hello world');
        await http_request();
        return state + msg;
    }
";

pub async fn run_v8() -> Result<(), ErrBox> {
    let mut isolate = IsolateCore::new(StartupData::None, false);
    let result = isolate.execute("test.js", SOURCE_CODE);
    match result {
        Result::Ok(()) => println!("{}", "Executed correctly"),
        Result::Err(err) => println!("{}", "Executed incorrectly"),
    };

    let v8_isolate = isolate.v8_isolate.as_mut().unwrap();
    {
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
    }
    isolate.await

}

pub mod controllers;
