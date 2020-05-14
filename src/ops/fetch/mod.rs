use crate::dispatch_json::{Deserialize, JsonOp, Value};
use super::io::{StreamResource, StreamResourceHolder};

use crate::op_error::OpError;
use crate::state::State;
use deno_core::CoreIsolate;
use deno_core::ZeroCopyBuf;
use http::header::HeaderName;
use http::header::HeaderValue;
use http::Method;
use std::convert::From;
use std::borrow::Borrow;
use futures::FutureExt;
use crate::ops::fetch::http_util::{create_http_client, HttpBody};

pub mod http_util;

pub fn init(i: &mut CoreIsolate, s: &State) {
    i.register_op("op_fetch", s.stateful_json_op2(op_fetch));
}

#[derive(Deserialize)]
struct FetchArgs {
    method: Option<String>,
    url: String,
    headers: Vec<(String, String)>,
}

pub fn op_fetch(
    isolate: &mut CoreIsolate,
    state: &State,
    args: Value,
    data: Option<ZeroCopyBuf>,
) -> Result<JsonOp, OpError> {
    let args: FetchArgs = serde_json::from_value(args)?;
    let url = args.url;

    let client =
        create_http_client(None)?;

    let method = match args.method {
        Some(method_str) => Method::from_bytes(method_str.as_bytes())
            .map_err(|e| OpError::other(e.to_string()))?,
        None => Method::GET,
    };

    let url_ = url::Url::parse(&url).map_err(OpError::from)?;

    // Check scheme before asking for net permission
    let scheme = url_.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(OpError::type_error(format!(
            "scheme '{}' not supported",
            scheme
        )));
    }


    let mut request = client.request(method, url_);

    if let Some(buf) = data {
        request = request.body(Vec::from(&*buf));
    }

    for (key, value) in args.headers {
        let name = HeaderName::from_bytes(key.as_bytes()).unwrap();
        let v = HeaderValue::from_str(&value).unwrap();
        request = request.header(name, v);
    }
    debug!("Before fetch {}", url);

    let resource_table = isolate.resource_table.clone();
    let future = async move {
        let res = request.send().await?;
        debug!("Fetch response {}", url);
        let status = res.status();
        let mut res_headers = Vec::new();
        for (key, val) in res.headers().iter() {
            res_headers.push((key.to_string(), val.to_str().unwrap().to_owned()));
        }

        let body = HttpBody::from(res);
        let mut resource_table = resource_table.borrow_mut();
        let rid = resource_table.add(
            "httpBody",
            Box::new(StreamResourceHolder::new(StreamResource::HttpBody(
                Box::new(body),
            ))),
        );

        let json_res = json!({
      "bodyRid": rid,
      "status": status.as_u16(),
      "statusText": status.canonical_reason().unwrap_or(""),
      "headers": res_headers
    });

        Ok(json_res)
    };

    Ok(JsonOp::Async(future.boxed_local()))
}
