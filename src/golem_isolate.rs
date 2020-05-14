use std::mem;

use crate::golem_isolate::IsolateCreationError::{FailedToCompileCode, NoMain};
use std::convert::TryFrom;
use std::borrow::BorrowMut;
use std::sync::{Arc, Mutex};
use deno_core::{Script, Snapshot, CoreIsolate, ErrBox, ZeroCopyBuf, Op, OpId};
use rusty_v8::{self as v8, Function, Global, Local, ContextScope, HandleScope, Value};
use deno_core::Snapshot::{JustCreated, Static};
use futures::io::IoSlice;
use std::ffi::CStr;
use std::mem::{transmute, forget};
use std::pin::Pin;
use actix_web::web::scope;
use std::future::Future;
use futures::task::{Context, Poll};
use futures::{TryFuture, FutureExt, TryFutureExt};
use std::io::Error;


pub enum IsolateCreationError {
    NoMain,
    FailedToRestoreSnapshot,
    FailedToCompileCode,
}

enum StartupData<'a> {
    Script(Script<'a>),
    NativeSnapshot(rusty_v8::StartupData),
    Snapshot(&'a GolemSnapshot),
}

#[derive(Debug)]
pub struct GolemSnapshot {
    data: Vec<u8>
}

impl Clone for GolemSnapshot {
    fn clone(&self) -> Self {
        let mut data = vec![0; self.data.len()];
        data.copy_from_slice(&self.data);
        GolemSnapshot {
            data
        }
    }
}


#[repr(C)]
#[no_mangle]
struct SnapshotData {
    data: *const libc::c_char,
    raw_size: libc::c_int,
}

impl GolemSnapshot {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn to_startup_data(&self) -> rusty_v8::StartupData {
        let vec = &self.data;
        let raw_size: libc::c_int = i32::try_from(vec.len()).unwrap();
        let data = vec.as_ptr() as *const libc::c_char;

        let data = SnapshotData {
            data,
            raw_size,
        };
        unsafe {
            transmute(data)
        }
    }

    pub fn from_startup_data(startup_data: &rusty_v8::StartupData) -> Self {
        let slice: &[u8] = &*startup_data;
        let mut vec = vec![0; slice.len()];
        vec.clone_from_slice(&slice);
        Self::new(vec)
    }
}

trait Invokeable {
    fn invoke_function(self: &mut Self, handle: &Global<Function>);
}

impl Invokeable for CoreIsolate {
    fn invoke_function(self: &mut Self, handle: &Global<Function>) {
        // let js_error_create_fn = &*self.js_error_create_fn;
        let v8_isolate = self.v8_isolate.as_mut().unwrap();

        let mut hs = v8::HandleScope::new(v8_isolate);
        let scope = hs.enter();
        assert!(!self.global_context.is_empty());
        let context = self.global_context.get(scope).unwrap();
        let mut cs = v8::ContextScope::new(scope, context);
        let scope = cs.enter();


        let mut try_catch = v8::TryCatch::new(scope);
        let tc = try_catch.enter();

        let arg1 = v8::Local::from(v8::Number::new(scope, 1.0));
        let arg2 = v8::Local::from(v8::Number::new(scope, 2.0));

        // let global = context.global(scope);

        // // v8::Local::
        // let main = global.get(scope, context, function_name);

        let function: v8::Local<v8::Function> = handle.get(scope).unwrap();
        // v8::Local::<v8::Function>::try_from(handle).unwrap();
        // let main: v8::Local<v8::Function> = v8::Local::<v8::Function>::try_from(handle).unwrap();

        // let global = context.global(scope);
        let this = v8::Object::new(scope);
        let result = function.call(scope, context, this.into(), &[arg1, arg2]).unwrap();
        // let result = scope.escape(result);
        // let result: Global<Value> = v8::Global::new_from(scope, result);

        let str = result.to_string(scope).unwrap();

        // Result::Ok(result)
    }
}


pub struct GolemIsolate {
    core_isolate: CoreIsolate,
    main_handle: Global<Function>,
    cache_handle: Option<Global<Function>>,
    snapshot: Option<GolemSnapshot>,
    state: Global<Value>,
}

// fn register_op<F>(
//     &mut self,
//     name: &'static str,
//     handler: impl Fn(State, u32, Option<ZeroCopyBuf>) -> F + Copy + 'static,
// ) where
//     F: TryFuture,
//     F::Ok: TryInto<i32>,
//     <F::Ok as TryInto<i32>>::Error: Debug,
// {
//     let state = self.state.clone();
//     let core_handler = move |_isolate: &mut CoreIsolate,
//                              control_buf: &[u8],
//                              zero_copy_buf: Option<ZeroCopyBuf>|
//                              -> Op {
//         let state = state.clone();
//         let record = Record::from(control_buf);
//         let is_sync = record.promise_id == 0;
//         assert!(!is_sync);
//
//         let fut = async move {
//             let op = handler(state, record.rid, zero_copy_buf);
//             let result = op
//                 .map_ok(|r| r.try_into().expect("op result does not fit in i32"))
//                 .unwrap_or_else(|_| -1)
//                 .await;
//             RecordBuf::from(Record { result, ..record })[..].into()
//         };
//
//         Op::Async(fut.boxed_local())
//     };
//
//     self.core_isolate.register_op(name, core_handler);
// }

//
// fn register_sync_op<F>(&mut self, name: &'static str, handler: F)
//     where
//         F: 'static + Fn(State, u32, Option<ZeroCopyBuf>) -> Result<u32, Error>,
// {
//     let state = self.state.clone();
//     let core_handler = move |_isolate: &mut CoreIsolate,
//                              control_buf: &[u8],
//                              zero_copy_buf: Option<ZeroCopyBuf>|
//                              -> Op {
//         let state = state.clone();
//         let record = Record::from(control_buf);
//         let is_sync = record.promise_id == 0;
//         assert!(is_sync);
//
//         let result: i32 = match handler(state, record.rid, zero_copy_buf) {
//             Ok(r) => r as i32,
//             Err(_) => -1,
//         };
//         let buf = RecordBuf::from(Record { result, ..record })[..].into();
//         Op::Sync(buf)
//     };
//
//     self.core_isolate.register_op(name, core_handler);
// }


// This is a local proof that an isolate was created with the provided code
// that passed the requirements to qualify as a golem isolate.
// These requirements are largely encoded in the GolemIsolate struct,
// but the most important of which is that the code contains a main method

fn create_and_setup_isolate(startup_data: StartupData) -> Result<Box<CoreIsolate>, IsolateCreationError> {
    let (core_startup_data, script) = match startup_data {
        StartupData::Script(script) => (deno_core::StartupData::None, Some(script)), //Won't run this script here as it may panic
        StartupData::Snapshot(snapshot) => {
            let data = snapshot.to_startup_data();
            (deno_core::StartupData::Snapshot(JustCreated(data)), None)
        }
        StartupData::NativeSnapshot(data) => {
            (deno_core::StartupData::Snapshot(JustCreated(data)), None)
        }
    };

    let mut core_isolate = CoreIsolate::new(core_startup_data, script.is_some());


    match script {
        Some(script) => {
            let result = core_isolate.execute(script.filename, script.source);
            match result {
                Err(_) => Err(FailedToCompileCode),
                Ok(_) => Ok(())
            }
        }
        None => Ok(())
    }?;

    Ok(core_isolate)
}


impl GolemIsolate {
    fn try_new(startup_data: StartupData) -> Result<Box<Self>, IsolateCreationError> {
        let (snapshot, startup_data) = match startup_data {
            StartupData::Snapshot(snapshot) => {
                Ok((None, StartupData::Snapshot(snapshot)))
            }
            StartupData::NativeSnapshot(_) => {
                Ok((None, startup_data))
            }
            StartupData::Script(script) => {
                let mut core_isolate = create_and_setup_isolate(StartupData::Script(script))?;
                let snapshot = core_isolate.snapshot();
                let golem_snapshot = GolemSnapshot::from_startup_data(&snapshot);

                Ok((Some(golem_snapshot), StartupData::NativeSnapshot(snapshot)))
            }
        }?;

        let mut core_isolate = create_and_setup_isolate(startup_data)?;

        let main_handle = Self::try_get_function_handle(&mut core_isolate, "main");
        let main_handle = match main_handle {
            Some(x) => Ok(x),
            None => Err(NoMain)
        }?;


        let cache_handle = Self::try_get_function_handle(&mut core_isolate, "cache");


        let state: Global<Value> = {
            let mut v8_isolate = core_isolate.v8_isolate.as_mut().unwrap();
            assert!(!core_isolate.global_context.is_empty());


            let mut hs = HandleScope::new(v8_isolate);
            let scope = hs.enter();
            let value: Local<Value> = v8::undefined(scope).into();
            Global::new_from(scope, value)
        };


        let golem = Self {
            core_isolate: *core_isolate,
            main_handle,
            cache_handle,
            snapshot,
            state,
        };

        Ok(Box::from(golem))
    }

    fn try_get_function_handle(core_isolate: &mut CoreIsolate, name: &str) -> Option<Global<Function>> {
        let mut v8_isolate = core_isolate.v8_isolate.as_mut().unwrap();
        assert!(!core_isolate.global_context.is_empty());


        let mut hs = HandleScope::new(v8_isolate);
        let scope = hs.enter();
        assert!(!core_isolate.global_context.is_empty());

        let context = core_isolate.global_context.get(scope).unwrap();

        let mut cs = ContextScope::new(scope, context);
        let scope = cs.enter();

        let context = core_isolate.global_context.get(scope).unwrap();
        let mut cs = ContextScope::new(scope, context);
        let scope = cs.enter();
        let global = context.global(scope);

        let function_name = rusty_v8::String::new(scope, name).unwrap().into();
        let function = global.get(scope, context, function_name);
        function
            .and_then(|function| if function.is_function() { Some(function) } else { None })
            .map(|function| Local::<Function>::try_from(function).unwrap())
            .map(|function: Local<Function>| Global::new_from(scope, function))
    }

    pub fn try_create_snapshot(script: Script) -> Result<GolemSnapshot, IsolateCreationError> {
        let mut golem = Self::try_new(StartupData::Script(script))?;
        let snapshot = golem.snapshot.take().unwrap();

        Ok(snapshot)
    }

    pub fn new(snapshot: GolemSnapshot) -> Box<Self> {
        let golem = Self::try_new(StartupData::Snapshot(&snapshot));

        match golem {
            Ok(g) => {
                // V8 takes ownership of the snapshot, so to prevent a doublefree, need to forget about the snapshot data
                forget(snapshot);
                g
            }
            Err(e) => panic!("Expected creation of GolemSnapshot to imply that this function has been correctly validated")
        }
    }

    pub fn invoke_main(&mut self) {
        &self.core_isolate.invoke_function(&self.main_handle);
    }

    pub async fn get_future(self) -> Result<(), ErrBox> {
        self.core_isolate.await
    }


    pub fn register_op<F>(&mut self, name: &str, handler: F) -> OpId
        where
            F: Fn(&mut CoreIsolate, &[u8], Option<ZeroCopyBuf>) -> Op + 'static
    {
        self.core_isolate.register_op(name, handler)
    }

    pub fn register_json_op<F>(&mut self, name: &str, handler: F) -> OpId
        where
            F: Fn(&mut CoreIsolate, &[u8], Option<ZeroCopyBuf>) -> Op + 'static
    {
        self.core_isolate.register_op(name, handler)
    }
}

