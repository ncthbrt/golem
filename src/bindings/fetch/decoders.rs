use crate::bindings::fetch::FetchArgs;
use http::Method;
use rusty_v8 as v8;
use std::str::FromStr;

fn get_url<'s>(
    scope: &mut v8::FunctionCallbackScope,
    url: v8::Local<v8::Value>,
) -> Result<String, String> {
    if url.is_string() {
        Result::Ok(url.to_string(*scope).unwrap().to_rust_string_lossy(*scope))
    } else {
        Result::Err(String::from("URL required as first argument"))
    }
}

fn decode_argarray<'a>(
    rest: FetchArgs,
    scope: &mut v8::FunctionCallbackScope,
    context: v8::Local<'a, v8::Context>,
    arg: v8::Local<v8::Value>,
) -> Result<FetchArgs, String> {
    if arg.is_object() {
        let obj = arg.to_object(*scope).unwrap();
        let method = v8::String::new(*scope, "method").unwrap().into();
        let method = (*obj)
            .get(*scope, context, method)
            .and_then(|x| x.to_string(*scope))
            .map(|x| x.to_rust_string_lossy(*scope))
            .unwrap_or_else(|| String::from("GET"));

        let method = Method::from_str(&method).map_err(|_| String::from("Invalid Method"))?;
        Result::Ok(FetchArgs { method, ..rest })
    } else {
        Result::Err(String::from("Expected Object"))
    }
}

pub fn decode_arguments<'a>(
    scope: &mut v8::FunctionCallbackScope,
    context: v8::Local<v8::Context>,
    args: v8::FunctionCallbackArguments,
) -> Result<FetchArgs, String> {
    let args_count = args.length();
    assert!(args_count >= 0);
    if args_count == 0 {
        Result::Err(String::from("No args found"))
    } else {
        let url = get_url(scope, args.get(0))?;
        let default_args: FetchArgs = FetchArgs {
            method: Method::GET,
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
