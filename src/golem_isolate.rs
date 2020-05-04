use std::mem;
use crate::isolate_core::{IsolateCore, StartupData as CoreStartupData, Script, Snapshot};
use rusty_v8::{self as v8, Function, Global, Local};
use crate::golem_isolate::IsolateCreationError::{FailedToCompileCode, NoMain};
use std::convert::TryFrom;
use std::borrow::BorrowMut;
use std::sync::Arc;

pub enum IsolateCreationError {
    NoMain,
    FailedToRestoreSnapshot,
    FailedToCompileCode,
}

enum StartupData<'a> {
    Script(Script<'a>),
    Snapshot(Snapshot),
}

pub struct GolemIsolate {
    isolate_core: Option<Box<IsolateCore>>,
    main_handle: Option<Global<Function>>,
    cache_handle: Option<Global<Function>>,
    snapshot: Option<v8::StartupData>,
}

// This is a local proof that an isolate was created with the provided code
// that passed the requirements to qualify as a golem isolate.
// These requirements are largely encoded in the GolemIsolate struct,
// but the most important of which is that the code contains a main method
pub struct GolemSnapshot(Snapshot);

impl GolemIsolate {
    fn try_new(startup_data: StartupData, save_snapshot: bool) -> Result<Self, IsolateCreationError> {
        let (core_startup_data, script) = match startup_data {
            StartupData::Script(script) => (CoreStartupData::None, Some(script)), //Won't run this script here as it may panic
            StartupData::Snapshot(snapshot) => (CoreStartupData::Snapshot(snapshot), None),
        };
        let will_snapshot = save_snapshot;
        let mut isolate_core = IsolateCore::new(core_startup_data, will_snapshot);

        match script {
            Some(script) => {
                let result = isolate_core.execute(script.filename, script.source);
                match result {
                    Err(_) => Err(FailedToCompileCode),
                    Ok(_) => Ok(())
                }
            }
            None => Ok(())
        }?;

        let snapshot = if (save_snapshot) {
            Some(isolate_core.snapshot())
        } else {
            None
        };
        let v8_isolate = isolate_core.v8_isolate.as_mut().unwrap();


        let mut hs = v8::HandleScope::new(v8_isolate);
        let scope = hs.enter();


        let main_handle = {
            let context = isolate_core.global_context.get(scope).unwrap();
            let mut cs = v8::ContextScope::new(scope, context);
            let scope = cs.enter();
            let global = context.global(scope);

            let main_function_name = v8::String::new(scope, "main").unwrap().into();
            let main = global.get(scope, context, main_function_name);

            let main = main
                .and_then(|main| if main.is_function() { Some(main) } else { None })
                .map(|main| v8::Local::<v8::Function>::try_from(main).unwrap());


            match main {
                Some(main) => {
                    let mut m = v8::Global::<v8::Function>::new();
                    m.set(scope, main);
                    Ok(m)
                }
                None => Err(IsolateCreationError::NoMain),
            }
        }?;
        let main_handle = Some(main_handle);


        let cache_handle = {
            let context = isolate_core.global_context.get(scope).unwrap();
            let mut cs = v8::ContextScope::new(scope, context);
            let scope = cs.enter();
            let global = context.global(scope);

            let cache_function_name = v8::String::new(scope, "cache").unwrap().into();
            let cache_function = global.get(scope, context, cache_function_name);
            cache_function
                .and_then(|cache_function| if cache_function.is_function() { Some(cache_function) } else { None })
                .map(|cache_function| v8::Local::<v8::Function>::try_from(cache_function).unwrap())
                .map(|cache_function: Local<Function>| Global::new_from(scope, cache_function))
        };


        Ok(Self {
            isolate_core: Some(isolate_core),
            main_handle,
            cache_handle,
            snapshot,
        })
    }

    pub fn try_create_snapshot(script: Script) -> Result<GolemSnapshot, IsolateCreationError> {
        let snapshot = {
            println!("{}", "pre create");
            let mut golem = Self::try_new(StartupData::Script(script), true)?;
            println!("{}", "post create");
            let snapshot = golem.snapshot.take().unwrap();
            println!("{}", "took snapshot");
            Ok(Snapshot::JustCreated(snapshot))
        }?;

        Ok(GolemSnapshot(snapshot))
    }

    pub fn new(GolemSnapshot(snapshot): GolemSnapshot) -> Self {
        let golem = Self::try_new(StartupData::Snapshot(snapshot), false);
        golem.unwrap_or_else(|_| panic!("Expected creation of GolemSnapshot to imply that this functino has been correctly validated"))
    }
}

// fn try_drop(x: Option<impl Drop>) {
//     if let Some(value) = x {
//         drop(value);
//     }
// }

// impl Drop for GolemIsolate {
//     fn drop(&mut self) {
//         try_drop(self.main_handle.take());
//         try_drop(self.isolate_core.take());
//     }
// }