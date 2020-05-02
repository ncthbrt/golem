// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
use crate::isolate_core::IsolateCore;
use crate::isolate_core::ZeroCopyBuf;
use futures::{Future};
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;
use serde_json::Value;
use crate::ops::op_error::OpError;

mod op_error;
mod dispatch_json;
pub(crate) mod http_request;

pub type OpId = u32;

pub type Buf = Box<[u8]>;

pub type OpAsyncFuture = Pin<Box<dyn Future<Output=Buf>>>;


#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncArgs {
    promise_id: Option<u64>,
}


pub enum Op {
    Sync(Buf),
    Async(OpAsyncFuture),
    /// AsyncUnref is the variation of Async, which doesn't block the program
    /// exiting.
    AsyncUnref(OpAsyncFuture),
}

/// Main type describing op
pub type OpDispatcher =
dyn Fn(&mut IsolateCore, &[u8], Option<ZeroCopyBuf>) -> Op + 'static;

#[derive(Default)]
pub struct OpRegistry {
    dispatchers: Vec<Rc<OpDispatcher>>,
    name_to_id: HashMap<String, OpId>,
}

impl OpRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        let op_id = registry.register("ops", |isolate, _, _| {
            let buf = isolate.op_registry.json_map();
            Op::Sync(buf)
        });
        assert_eq!(op_id, 0);
        registry
    }

    pub fn register<F>(&mut self, name: &str, op: F) -> OpId
        where
            F: Fn(&mut IsolateCore, &[u8], Option<ZeroCopyBuf>) -> Op + 'static,
    {
        let op_id = self.dispatchers.len() as u32;

        let existing = self.name_to_id.insert(name.to_string(), op_id);
        assert!(
            existing.is_none(),
            format!("Op already registered: {}", name)
        );
        self.dispatchers.push(Rc::new(op));
        op_id
    }

    fn json_map(&self) -> Buf {
        let op_map_json = serde_json::to_string(&self.name_to_id).unwrap();
        op_map_json.as_bytes().to_owned().into_boxed_slice()
    }

    pub fn get(&self, op_id: OpId) -> Option<Rc<OpDispatcher>> {
        self.dispatchers.get(op_id as usize).map(Rc::clone)
    }
}

