use crate::bindings::utils;
use reqwest;
use rusty_v8 as v8;
use v8::PromiseResolver;

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

struct FetchArgs {
    method: HttpMethod,
    url: String,
}

fn get_url(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
) -> Result<String, String> {
    assert!(args.length() >= 1);
    let url = args.get(0);
    if url.is_string() {
        Result::Ok(url.to_string(scope).unwrap().to_rust_string_lossy(scope))
    } else {
        Result::Err(String::from("URL required as first argument"))
    }
}

fn decode_arguments(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
) -> Result<FetchArgs, String> {
    let args_count = args.length();
    assert!(args_count >= 0);
    if args_count == 0 {
        Result::Err(String::from("No args found"))
    } else {
        let default_args: FetchArgs = FetchArgs {
            method: HttpMethod::Get,
            url: String::default(),
        };
        let url = get_url(scope, args)?;
        if args_count > 1 {
            Result::Ok(FetchArgs {
                method: HttpMethod::Get,
                url: url,
                ..default_args
            })
        } else {
            Result::Ok(FetchArgs {
                method: HttpMethod::Get,
                url: url,
                ..default_args
            })
        }
    }
}

pub fn fetch_impl(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    // resolver: v8::Local<PromiseResolver>,
) -> Result<String, String> {
    let arguments = decode_arguments(scope, args)?;

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
    let request = v8::Object::new(scope);

    utils::add_function(scope, context, &request, "fetch", fetch);

    global.set(
        *context,
        v8::String::new(scope, "request").unwrap().into(),
        request.into(),
    );
}
