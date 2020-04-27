use crate::bindings::utils;
use http::Method;
use rusty_v8 as v8;
use v8::PromiseResolver;
use std::future::Future;
use tokio;
use actix_web::client::Client;
use reqwest::Url;
use std::str::FromStr;
use std::thread;
use rusty_v8::{Promise, Local};

mod decoders;
mod encoders;


mod implementation;


#[derive(Debug)]
pub struct FetchArgs {
    method: Method,
    url: String,
}

pub fn fetch(
    mut root_scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    println!("{}", "Calling fetch");
    let context = v8::Context::new(root_scope);
    let args = decoders::decode_arguments(&mut root_scope, context, args);

    let mut hs = v8::EscapableHandleScope::new(root_scope);
    let scope = hs.enter();
    let context = v8::Context::new(scope);

    let promise_resolver = PromiseResolver::new(scope, context).unwrap();
    let promise = promise_resolver.get_promise(scope).into();
    let escaped_promise = scope.escape(promise);
    rv.set(escaped_promise);
    match args {
        Result::Err(msg) => {
            println!("{}", "Parsing args has errors");
            let error_value = v8::String::new(scope, &msg).unwrap().into();
            promise_resolver.reject(context, error_value);
        }
        Result::Ok(args) => {
            let mut resolver_handle = v8::Global::new();
            resolver_handle.set(scope, promise_resolver);

            let mut resolver_handle = v8::Global::new();
            resolver_handle.set(scope, promise_resolver);
            tokio::task::spawn(move || {
                let resolver: Local<PromiseResolver> = resolver_handle.get(scope).unwrap().into();
                println!("{}", "Parsing args is ok");
                let client = reqwest::blocking::Client::new();
                let request = client.request(args.method, &args.url);
                match request.send() {
                    Result::Ok(_) => resolver.resolve(context, v8::String::new(scope, "Something good").unwrap().into()),
                    Result::Err(_) => resolver.reject(context, v8::String::new(scope, "Something bad").unwrap().into()),
                };
            });
        }
    };
}

pub fn inject_fetch<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    context: &v8::Local<'sc, v8::Context>,
    global: &v8::Local<'sc, v8::Object>,
) {
    utils::add_function(scope, context, &global, "fetch", fetch);
}
