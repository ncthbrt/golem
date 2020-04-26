use crate::bindings::utils;
use reqwest;
use rusty_v8 as v8;
use rusty_v8::scope::Entered;
use rusty_v8::FunctionCallbackInfo;
use v8::PromiseResolver;

#[derive(Debug)]
enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

#[derive(Debug)]
struct FetchArgs {
    method: HttpMethod,
    url: String,
}

fn get_url<'s>(
    scope: &mut Entered<'s, FunctionCallbackInfo>,
    url: v8::Local<v8::Value>,
) -> Result<String, String> {
    if url.is_string() {
        Result::Ok(url.to_string(scope).unwrap().to_rust_string_lossy(scope))
    } else {
        Result::Err(String::from("URL required as first argument"))
    }
}

fn decode_argarray<'a>(
    rest: FetchArgs,
    scope: v8::FunctionCallbackScope,
    context: v8::Local<'a, v8::Context>,
    arg: v8::Local<v8::Value>,
) -> Result<FetchArgs, String> {
    if arg.is_object() {
        let obj = arg.to_object(scope).unwrap();
        let method = v8::String::new(scope, "method").unwrap().into();
        let method = (*obj)
            .get(scope, context, method)
            .and_then(|x| x.to_string(scope))
            .map(|x| x.to_rust_string_lossy(scope));
        let method = match method {
            Option::Some(s) if s == "GET" => HttpMethod::Get,
            Option::Some(s) if s == "HEAD" => HttpMethod::Head,
            Option::Some(s) if s == "POST" => HttpMethod::Post,
            Option::Some(s) if s == "PUT" => HttpMethod::Put,
            Option::Some(s) if s == "DELETE" => HttpMethod::Delete,
            Option::Some(s) if s == "CONNECT" => HttpMethod::Connect,
            Option::Some(s) if s == "OPTIONS" => HttpMethod::Options,
            Option::Some(s) if s == "TRACE" => HttpMethod::Trace,
            Option::Some(s) if s == "PATCH" => HttpMethod::Patch,
            _ => HttpMethod::Get,
        };
        Result::Ok(FetchArgs { method, ..rest })
    } else {
        Result::Err(String::from("Expected Object"))
    }
}

fn decode_arguments(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
) -> Result<FetchArgs, String> {
    let context = v8::Context::new(scope);

    let args_count = args.length();
    assert!(args_count >= 0);
    if args_count == 0 {
        Result::Err(String::from("No args found"))
    } else {
        let url = get_url(scope, args.get(0))?;
        let default_args: FetchArgs = FetchArgs {
            method: HttpMethod::Get,
            url,
        };
        if args_count > 1 {
            let second_arg = args.get(1);
            decode_argarray(default_args, scope, context, second_arg)
        } else {
            Result::Ok(default_args)
        }
    }
}

pub fn fetch_impl(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    // resolver: v8::Local<PromiseResolver>,
) -> Result<String, String> {
    let arguments = decode_arguments(scope, args)?;
    println!("{:?}", arguments);
    Result::Ok(String::from("ok"))
}

pub fn fetch<'a, 'b, 'c>(
    scope: v8::FunctionCallbackScope<'a>,
    args: v8::FunctionCallbackArguments<'b>,
    mut _rv: v8::ReturnValue<'c>,
) {
    // let context = v8::Context::new(scope);
    // let promise_resolver = PromiseResolver::new(scope, context).unwrap();
    let result = fetch_impl(scope, args);
    println!("{:?}", result);
    // let promise = promise_resolver.get_promise(scope).into();
    // rv.set(promise);
}

pub fn inject_fetch<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    context: &v8::Local<'sc, v8::Context>,
    global: &v8::Local<'sc, v8::Object>,
) {
    utils::add_function(scope, context, &global, "fetch", fetch);
}
