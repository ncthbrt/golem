use crate::bindings::utils;
use rusty_v8 as v8;
use std::convert::TryInto;

fn to_console_string(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
) -> std::string::String {
    let arg_len = args.length().try_into().unwrap();
    let capacity: usize = args.length().try_into().unwrap();
    let mut str_vec = Vec::with_capacity(capacity);
    let context = v8::Context::new(scope);

    for i in 0..arg_len {
        let obj = args.get(i);

        if obj.is_object() || obj.is_array() {
            let json = v8::json::stringify(context, obj)
                .unwrap_or_else(|| v8::String::new(scope, "").unwrap());
            str_vec.push(json.to_rust_string_lossy(scope));
        } else {
            let string_value = obj
                .to_string(scope)
                .unwrap_or_else(|| v8::String::new(scope, "").unwrap());
            str_vec.push(string_value.to_rust_string_lossy(scope));
        }
    }

    str_vec.join(" ")
}

fn console_log(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) -> () {
    println!("{}", to_console_string(scope, args));
}

fn console_error(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) -> () {
    eprint!("{}", to_console_string(scope, args));
}

pub fn inject_console<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    context: &v8::Local<'sc, v8::Context>,
    global: &v8::Local<'sc, v8::Object>,
) {
    let console = v8::Object::new(scope);

    utils::add_function(scope, context, &console, "log", console_log);
    utils::add_function(scope, context, &console, "error", console_error);

    global.set(
        *context,
        v8::String::new(scope, "console").unwrap().into(),
        console.into(),
    );
}
