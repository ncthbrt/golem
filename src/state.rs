// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
use crate::global_timer::GlobalTimer;
use crate::op_error::OpError;

use deno_core::Buf;
use deno_core::CoreIsolate;
use deno_core::ErrBox;
use deno_core::ModuleLoadId;
use deno_core::ModuleLoader;
use deno_core::ModuleSpecifier;
use deno_core::Op;
use deno_core::ZeroCopyBuf;
use futures::future::FutureExt;
use futures::Future;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::str;
use std::thread::JoinHandle;
use std::time::Instant;
use crate::dispatch_json::{json_op, JsonOp};
use crate::dispatch_minimal::{MinimalOp, minimal_op};

#[derive(Clone)]
pub struct State(Rc<RefCell<StateInner>>);

impl Deref for State {
    type Target = Rc<RefCell<StateInner>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(stutter))]
pub struct StateInner {
    /// When flags contains a `.import_map_path` option, the content of the
    /// import map file will be resolved and set.
    pub global_timer: GlobalTimer,
    pub start_time: Instant,
    pub seeded_rng: Option<StdRng>,
}

impl State {
    pub fn stateful_json_op<D>(
        &self,
        dispatcher: D,
    ) -> impl Fn(&mut deno_core::CoreIsolate, &[u8], Option<ZeroCopyBuf>) -> Op
        where
            D: Fn(&State, Value, Option<ZeroCopyBuf>) -> Result<JsonOp, OpError>,
    {
        json_op(self.stateful_op(dispatcher))
    }

    pub fn stateful_json_op2<D>(
        &self,
        dispatcher: D,
    ) -> impl Fn(&mut deno_core::CoreIsolate, &[u8], Option<ZeroCopyBuf>) -> Op
        where
            D: Fn(
                &mut deno_core::CoreIsolate,
                &State,
                Value,
                Option<ZeroCopyBuf>,
            ) -> Result<JsonOp, OpError>,
    {
        json_op(self.stateful_op2(dispatcher))
    }


    /// This is a special function that provides `state` argument to dispatcher.
    ///
    /// NOTE: This only works with JSON dispatcher.
    /// This is a band-aid for transition to `CoreIsolate.register_op` API as most of our
    /// ops require `state` argument.
    pub fn stateful_op<D>(
        &self,
        dispatcher: D,
    ) -> impl Fn(
        &mut deno_core::CoreIsolate,
        Value,
        Option<ZeroCopyBuf>,
    ) -> Result<JsonOp, OpError>
        where
            D: Fn(&State, Value, Option<ZeroCopyBuf>) -> Result<JsonOp, OpError>,
    {
        let state = self.clone();
        move |_isolate: &mut deno_core::CoreIsolate,
              args: Value,
              zero_copy: Option<ZeroCopyBuf>|
              -> Result<JsonOp, OpError> { dispatcher(&state, args, zero_copy) }
    }

    pub fn stateful_op2<D>(
        &self,
        dispatcher: D,
    ) -> impl Fn(
        &mut deno_core::CoreIsolate,
        Value,
        Option<ZeroCopyBuf>,
    ) -> Result<JsonOp, OpError>
        where
            D: Fn(
                &mut deno_core::CoreIsolate,
                &State,
                Value,
                Option<ZeroCopyBuf>,
            ) -> Result<JsonOp, OpError>,
    {
        let state = self.clone();
        move |isolate: &mut deno_core::CoreIsolate,
              args: Value,
              zero_copy: Option<ZeroCopyBuf>|
              -> Result<JsonOp, OpError> {
            dispatcher(isolate, &state, args, zero_copy)
        }
    }


    pub fn stateful_minimal_op2<D>(
        &self,
        dispatcher: D,
    ) -> impl Fn(&mut deno_core::CoreIsolate, &[u8], Option<ZeroCopyBuf>) -> Op
        where
            D: Fn(
                &mut deno_core::CoreIsolate,
                &State,
                bool,
                i32,
                Option<ZeroCopyBuf>,
            ) -> MinimalOp,
    {
        let state = self.clone();
        minimal_op(
            move |isolate: &mut deno_core::CoreIsolate,
                  is_sync: bool,
                  rid: i32,
                  zero_copy: Option<ZeroCopyBuf>|
                  -> MinimalOp {
                dispatcher(isolate, &state, is_sync, rid, zero_copy)
            },
        )
    }
}

impl State {
    /// If `shared_permission` is None then permissions from globa state are used.
    pub fn new() -> Result<Self, ErrBox> {
        let seeded_rng = None;

        let state = Rc::new(RefCell::new(StateInner {
            global_timer: GlobalTimer::new(),
            start_time: Instant::now(),
            seeded_rng,
        }));

        Ok(Self(state))
    }
}
