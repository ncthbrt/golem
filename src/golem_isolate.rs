use std::mem;
// use crate::isolate_core::{IsolateCore, StartupData as CoreStartupData, Script, Snapshot};
// use rusty_v8::{self as v8, Function, Global, Local};
use crate::golem_isolate::IsolateCreationError::{FailedToCompileCode, NoMain};
use std::convert::TryFrom;
use std::borrow::BorrowMut;
use std::sync::Arc;
use deno_core::{Script, Snapshot, CoreIsolate};
use rusty_v8::{Function, Global, Local, ContextScope, HandleScope};
use deno_core::Snapshot::{JustCreated, Static};
use futures::io::IoSlice;
use std::ffi::CStr;
use std::mem::transmute;
use std::pin::Pin;

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

#[derive(Debug, Clone)]
pub struct GolemSnapshot {
    data: Vec<u8>
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
        println!("Slice length {}", slice.len());
        let mut vec = vec![0; slice.len()];
        vec.clone_from_slice(&slice);
        Self::new(vec)
    }
}


pub struct GolemIsolate {
    core_isolate: Arc<Box<CoreIsolate>>,
    main_handle: Option<Global<Function>>,
    cache_handle: Option<Global<Function>>,
    snapshot: Option<GolemSnapshot>,
}


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
    println!("{}", "Pre create final isolate");
    let mut core_isolate = CoreIsolate::new(core_startup_data, script.is_some());
    println!("{}", "Post created final isolate");

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
    fn try_new(startup_data: StartupData) -> Result<Pin<Box<Self>>, IsolateCreationError> {
        let (snapshot, startup_data) = match startup_data {
            StartupData::Snapshot(_) => {
                Ok((None, startup_data))
            }
            StartupData::NativeSnapshot(_) => {
                Ok((None, startup_data))
            }
            StartupData::Script(script) => {
                let mut core_isolate = create_and_setup_isolate(StartupData::Script(script))?;
                let snapshot = core_isolate.snapshot();
                let golem_snapshot = GolemSnapshot::from_startup_data(&snapshot);
                println!("{}", "Created golem snapshot");
                Ok((Some(golem_snapshot), StartupData::NativeSnapshot(snapshot)))
            }
        }?;

        let mut core_isolate = create_and_setup_isolate(startup_data)?;

        let main_handle = {
            let mut v8_isolate = core_isolate.v8_isolate.as_mut().unwrap();
            assert!(!core_isolate.global_context.is_empty());
            println!("{}", "Unwrapped the isolate");


            let mut hs = HandleScope::new(v8_isolate);
            let scope = hs.enter();
            assert!(!core_isolate.global_context.is_empty());

            let context = core_isolate.global_context.get(scope).unwrap();

            let mut cs = ContextScope::new(scope, context);
            let scope = cs.enter();

            let global = context.global(scope);

            let main_function_name = rusty_v8::String::new(scope, "main").unwrap().into();
            let main = global.get(scope, context, main_function_name);

            let main = main
                .and_then(|main| if main.is_function() { Some(main) } else { None })
                .map(|main| Local::<Function>::try_from(main).unwrap());


            match main {
                Some(main) => {
                    let mut m = Global::<Function>::new();
                    m.set(scope, main);
                    Ok(m)
                }
                None => Err(IsolateCreationError::NoMain),
            }
        }?;
        let main_handle = Some(main_handle);
        // let main_handle = None;

        println!("Got the main handle {}", main_handle.is_some());

        // let cache_handle = None;

        let cache_handle = {
            let mut v8_isolate = core_isolate.v8_isolate.as_mut().unwrap();
            assert!(!core_isolate.global_context.is_empty());
            println!("{}", "Unwrapped the isolate");


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

            let cache_function_name = rusty_v8::String::new(scope, "cache").unwrap().into();
            let cache_function = global.get(scope, context, cache_function_name);
            cache_function
                .and_then(|cache_function| if cache_function.is_function() { Some(cache_function) } else { None })
                .map(|cache_function| Local::<Function>::try_from(cache_function).unwrap())
                .map(|cache_function: Local<Function>| Global::new_from(scope, cache_function))
        };

        println!("Got the cache handle {}", cache_handle.is_some());

        core_isolate.execute("test.js", " ");

        let golem = Self {
            core_isolate: Arc::new(core_isolate),
            main_handle,
            cache_handle,
            snapshot,
        };

        Ok(Box::pin(golem))
    }

    pub fn try_create_snapshot(script: Script) -> Result<GolemSnapshot, IsolateCreationError> {
        println!("{}", "Start try_create_snapshot");
        let mut golem = Self::try_new(StartupData::Script(script))?;
        let snapshot = golem.snapshot.take().unwrap();
        println!("{}", "Returning snapshot from try_create_snapshot");
        Ok(snapshot)
    }

    pub fn new(snapshot: &GolemSnapshot) -> Pin<Box<Self>> {
        let golem = Self::try_new(StartupData::Snapshot(snapshot));
        match golem {
            Ok(g) => g,
            Err(e) => panic!("Expected creation of GolemSnapshot to imply that this function has been correctly validated")
        }
    }
}


fn try_drop(v: Option<impl Drop>) {
    if let Some(value) = v {
        drop(value)
    };
}

impl Drop for GolemIsolate {
    fn drop(&mut self) {
        println!("{}", "Start drop");
        // self.snapshot.take();
        try_drop(self.cache_handle.take());
        println!("{}", "dropped cache handle");
        try_drop(self.main_handle.take());
        println!("{}", "dropped main handle");
        // drop(self.core_isolate);
    }
}