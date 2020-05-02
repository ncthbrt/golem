use rusty_v8::{Promise, Local, Value, PromiseState, Scope, ContextScope, ToLocal};
use futures::Future;
use futures::task::{Context, Poll};
use tokio::macros::support::Pin;
use std::sync::{Arc, Mutex};
use std::ops::DerefMut;
use std::borrow::BorrowMut;

pub struct PromiseFutureWrapper<'sc> {
    promise: Local<'sc, Promise>
}

impl<'sc> PromiseFutureWrapper<'sc> {
    pub fn new(promise: Local<'sc, Promise>) -> Self {
        Self { promise }
    }
}

impl<'sc> Future for PromiseFutureWrapper<'sc> {
    type Output = Result<Local<'sc, Promise>, Local<'sc, Promise>>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut promise = self.promise;
        let state = promise.state();
        println!("{}", "Polling promise resolution");
        match state {
            PromiseState::Pending => Poll::Pending,
            PromiseState::Rejected => Poll::Ready(Result::Err(promise)),
            PromiseState::Fulfilled => Poll::Ready(Result::Ok(promise)),
        }
    }
}
