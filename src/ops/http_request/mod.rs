use crate::isolate_core::{IsolateCore, ZeroCopyBuf};
use crate::ops;
use crate::ops::dispatch_json::{json_op, JsonOp};
use serde_json::Value;
use crate::ops::Op;
use futures::FutureExt;
use crate::ops::op_error::OpError;

struct HttpRequestArgs {}

// Fn(&mut IsolateCore, Value, Option<ZeroCopyBuf>) -> Result<JsonOp, OpError>,
fn http_request(isolate: &mut IsolateCore, args: Value, data: Option<ZeroCopyBuf>) -> Result<JsonOp, OpError> {
    let future = async move {
        println!("{}", "Calling future");
        let json_res = json!({
        "body": "hello world",
        "status": 200,
        "statusText": "OK",
        });
        println!("{}", "Json Res");
        Ok(json_res)

    };
    Result::Ok(JsonOp::Async(future.boxed_local()))
}

// Fn(&mut IsolateCore, &[u8], Option<ZeroCopyBuf>) -> Op
pub fn init(isolate: &mut IsolateCore) {
    isolate.register_op("op_http_request", json_op(http_request));
}