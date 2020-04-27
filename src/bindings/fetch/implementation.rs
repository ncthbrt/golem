use crate::bindings::fetch::FetchArgs;
use reqwest::{self, Client, RequestBuilder, Url};
use rusty_v8::{self as v8, Local, PromiseResolver, ToLocal, Context};
use std::str::FromStr;
use tokio;


pub async fn execute_fetch<'a>(
    args: FetchArgs,
    scope: &mut impl ToLocal<'a>,
    context: Local<'a, Context>,
    resolver: Local<'a, PromiseResolver>,
) {
    println!("{:?}", args);
    let client = Client::new();
    let request: RequestBuilder = client.request(args.method, Url::from_str(&args.url).unwrap());
    match request.send().await {
        Result::Ok(_) => resolver.resolve(context,v8::String::new(scope, "Something good").unwrap().into()),
        Result::Err(_) => resolver.reject(context, v8::String::new(scope, "Something bad").unwrap().into()),
    };
}
