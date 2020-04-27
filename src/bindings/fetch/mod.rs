use crate::bindings::utils;
use http::Method;
use rusty_v8 as v8;
use v8::PromiseResolver;
use std::future::Future;
use actix_web::client::Client;
use reqwest::Url;
use std::str::FromStr;
use std::thread;
use rusty_v8::{Promise, Local, HandleScope};
use std::sync::mpsc;
use std::rc::Rc;
use v8::ExternalReferences;

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

    let mut hs = v8::EscapableHandleScope::new(root_scope);

    let scope = hs.enter();

    let context = v8::Context::new(scope);

    let resolver = PromiseResolver::new(scope, context).unwrap();
    let promise = resolver.get_promise(scope).into();

    let promise = scope.escape(promise);
    rv.set(promise);
    let args = decoders::decode_arguments(scope, context, args);

    unsafe {
        let mut scope = HandleScope::new(scope);
        let resolver = *resolver;

        thread::spawn(move || {
            let mut scope = scope.enter();
            let context = v8::Context::new(scope);
            match args {
                Result::Err(msg) => {
                    println!("{}", "Parsing args has errors");
                    let error_value = v8::String::new(scope, &msg).unwrap().into();
                    resolver.reject(context, error_value);
                }
                Result::Ok(args) => {
                    let client = reqwest::blocking::Client::new();
                    let request = client.request(args.method, &args.url);
                    let result = match request.send() {
                        Result::Ok(_) => Result::Ok(String::from("Something good")),
                        Result::Err(_) => Result::Err(String::from("Something bad"))
                    };
                    match result {
                        Result::Ok(result) => resolver.resolve(context, v8::String::new(scope, &result).unwrap().into()),
                        Result::Err(err) => resolver.reject(context, v8::String::new(scope, &err).unwrap().into()),
                    };
                }
            };
        });
    }
}


pub fn inject_fetch<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    context: &v8::Local<'sc, v8::Context>,
    global: &v8::Local<'sc, v8::Object>,
) {
    utils::add_function(scope, context, &global, "fetch", fetch);
}
