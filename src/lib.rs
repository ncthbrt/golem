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
use std::convert::{TryFrom, TryInto};
use crate::isolate_core::{StartupData, IsolateCore, Script};
use crate::any_error::ErrBox;
use rusty_v8::{Promise, Local, ToLocal, InIsolate, Value, Global};
use crate::promise_future_wrapper::PromiseFutureWrapper;
use std::sync::{Arc, Mutex};
use rusty_v8::scope::Entered;
use downcast_rs::DowncastSync;
use futures::{SinkExt, TryFutureExt};
use crate::golem_isolate::GolemIsolate;

mod ops;
mod bindings;
mod isolate_core;
mod resources;
mod shared_queue;
mod any_error;
mod js_errors;
mod golem_isolate;
mod promise_future_wrapper;

const SOURCE_CODE: &str = "
    async function main(state, msg, ctx) {
        console.log('hello world');
        await http_request();
        return state + msg;
    }
";


pub fn run_v8() -> Result<(), golem_isolate::IsolateCreationError> {
    // let mut isolateRef = Arc::new(Mutex::new(IsolateCore::new(StartupData::None, false)));
    let script = Script {
        source: SOURCE_CODE,
        filename: "test.js",
    };
    let snapshot = GolemIsolate::try_create_snapshot(script)?;
    println!("{}", "Created snapshot");

    {
        let isolate = GolemIsolate::new(snapshot);
        println!("{}", "Created golem isolate");
    }


    Ok(())

    // {
    //     let isolate = isolateRef.clone();
    //     let mut isolate = isolate.lock().unwrap();
    //     let result = isolate.execute("test.js", SOURCE_CODE);
    //     match result {
    //         Result::Ok(()) => println!("{}", "Executed correctly"),
    //         Result::Err(err) => println!("{}", "Executed incorrectly"),
    //     };
    // }
    //
    // {
    //     {
    //         let mut result = {
    //             let isolate = isolateRef.clone();
    //             let mut mutx = isolate.lock().unwrap();
    //             let mut isolate = &mut *mutx;
    //
    //             let v8_isolate = isolate.v8_isolate.as_mut().unwrap();
    //             let mut hs = v8::HandleScope::new(v8_isolate);
    //             let scope = hs.enter();
    //
    //             // let mut hs = v8::EscapableHandleScope::new(scope);
    //             // let scope = hs.enter();
    //
    //             let context = isolate.global_context.get(scope).unwrap();
    //             let mut cs = v8::ContextScope::new(scope, context);
    //             let scope = cs.enter();
    //
    //             let function_name = v8::String::new(scope, "main").unwrap().into();
    //             let global = context.global(scope);
    //             let main = global.get(scope, context, function_name);
    //
    //             if let Some(main) = main {
    //                 let arg1 = v8::Local::from(v8::Number::new(scope, 1.0));
    //                 let arg2 = v8::Local::from(v8::Number::new(scope, 2.0));
    //                 let main: v8::Local<v8::Function> = v8::Local::<v8::Function>::try_from(main).unwrap();
    //
    //                 // let global = context.global(scope);
    //                 let this = v8::Object::new(scope);
    //                 let result = main.call(scope, context, this.into(), &[arg1, arg2]).unwrap();
    //                 // let result = scope.escape(result);
    //                 let result: Global<Value> = v8::Global::new_from(scope, result);
    //                 Result::Ok(result)
    //             } else {
    //                 Result::Err("No main function")
    //             }
    //         };
    //
    //
    //         // result
    //         let async_isolate_ref = isolateRef.clone();
    //         async {
    //             println!("Entered async func");
    //             let isolate = async_isolate_ref;
    //             let mut mutx = isolate.lock().unwrap();
    //             println!("Got lock async func");
    //             let mut isolate = &mut *mutx;
    //             isolate.await;
    //         }.await;
    //
    //         {
    //             match result {
    //                 Result::Err(err) => println!("{}", err),
    //                 Result::Ok(result) => {
    //                     let isolate = isolateRef.clone();
    //                     let mut mutx = isolate.lock().unwrap();
    //                     let mut isolate = &mut *mutx;
    //
    //                     let v8_isolate = isolate.v8_isolate.as_mut().unwrap();
    //                     let mut hs = v8::HandleScope::new(v8_isolate);
    //                     let mut scope = hs.enter();
    //
    //                     let context = isolate.global_context.get(scope).unwrap();
    //                     let mut cs = v8::ContextScope::new(scope, context);
    //                     let scope = cs.enter();
    //
    //                     let result = result.get(scope).unwrap();
    //                     let result = if result.is_promise() {
    //                         let promise: Local<Promise> = result.try_into().unwrap();
    //                         let result = promise.result(scope);
    //                         println!("Promise state {:?}", promise.state());
    //                         let str = result.to_string(scope).unwrap();
    //                         str.to_rust_string_lossy(scope)
    //                     } else {
    //                         let str = result.to_string(scope).unwrap();
    //                         str.to_rust_string_lossy(scope)
    //                     };
    //                     println!("Final result: {}", result);
    //                 }
    //             };
    //         }
    //     }
    // }
}

pub mod controllers;
