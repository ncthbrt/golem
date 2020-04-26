use rusty_v8 as v8;
mod console;
mod fetch;
mod utils;

pub fn inject_bindings<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    context: &v8::Local<'sc, v8::Context>,
) {
    let global = context.global(scope);
    console::inject_console(scope, context, &global);
    fetch::inject_fetch(scope, context, &global);
}
