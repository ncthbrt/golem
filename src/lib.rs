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
use rusty_v8::{Promise, Local, ToLocal, InIsolate, Value, Global};
use std::sync::{Arc, Mutex};
use rusty_v8::scope::Entered;
use downcast_rs::DowncastSync;
use futures::{SinkExt, TryFutureExt};
use crate::golem_isolate::GolemIsolate;
use deno_core::Script;

// mod ops;
// mod bindings;
// mod isolate_core;
// mod resources;
// mod shared_queue;
// mod any_error;
// mod js_errors;
mod golem_isolate;
// mod promise_future_wrapper;

const SOURCE_CODE: &str = "
    function main(state, msg, ctx) {
        // console.log('hello world');
        // await http_request();
        return state + msg;
    }
";


pub async fn run_v8() -> Result<(), golem_isolate::IsolateCreationError> {
    let script = Script {
        source: SOURCE_CODE,
        filename: "test.js",
    };

    println!("{}", "Pre created snapshot");
    let snapshot = GolemIsolate::try_create_snapshot(script)?;
    // let snapshot = snapshot;
    println!("{}", "Created snapshot");


    println!("{}", "Pre create isolate");
    // let cloned = snapshot.clone();
    let mut isolate = GolemIsolate::new(snapshot);

    println!("{}", "Created golem isolate");

    isolate.invoke_main();
    println!("{}", "Invoked main");
    println!("{}", "Pre awaited isolate");


    let core_isolate = isolate.core_isolate;
    let result = core_isolate.await;
    println!("{:?}", result);

    println!("{}", "Awaited isolate");


    Ok(())
}

pub mod controllers;
