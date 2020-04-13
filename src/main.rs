use actix_web::{get, web, App, HttpServer, Responder};
use rusty_v8 as v8;

#[get("/{id}/{name}/index.html")]
async fn index(info: web::Path<(u32, String)>) -> impl Responder {
    format!("Hello {}! id:{}", info.1, info.0)
}

fn run_v8() {
    let platform = v8::new_default_platform();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let mut create_params = v8::Isolate::create_params();
    create_params.set_array_buffer_allocator(v8::new_default_allocator());
    let mut isolate = v8::Isolate::new(create_params);

    let mut handle_scope = v8::HandleScope::new(&mut isolate);
    let scope = handle_scope.enter();

    let context = v8::Context::new(scope);
    let mut context_scope = v8::ContextScope::new(scope, context);
    let scope = context_scope.enter();

    let code = v8::String::new(scope, "'Hello' + ' World!'").unwrap();
    println!("javascript code: {}", code.to_rust_string_lossy(scope));

    let mut script = v8::Script::compile(scope, context, code, None).unwrap();
    let result = script.run(scope, context).unwrap();
    let result = result.to_string(scope).unwrap();
    println!("result: {}", result.to_rust_string_lossy(scope));
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    run_v8();
    HttpServer::new(|| App::new().service(index))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
