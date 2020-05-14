use crate::golem_isolate::GolemIsolate;
use rusty_v8 as v8;
use std::convert::TryInto;
use deno_core::Op;

struct Logging {}

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

pub fn init(isolate: &mut GolemIsolate) {
    &isolate.register_op("op_console", |isolate, datam, buff| {
        Op::Sync(Box::from([]))
    });
}
