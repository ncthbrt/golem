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
use std::time::Instant;

mod dispatch_json;
mod dispatch_minimal;
mod op_error;
mod golem_isolate;
mod global_timer;
mod state;
mod ops;

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

    let snapshot = GolemIsolate::try_create_snapshot(script)?;

    let global_start_time = Instant::now();
    for _ in 0..1000 {
        let cloned = snapshot.clone();
        let mut isolate = GolemIsolate::new(cloned);
        isolate.invoke_main();
        isolate.get_future().await;
    }
    let global_end_time = Instant::now();
    let delta_time = global_end_time - global_start_time;
    println!("Total Run Time: {}ms", delta_time.as_millis());
    println!("Average Run Time: {} microsecs", delta_time.as_micros() / 1000);

    Ok(())
}

pub mod controllers;
