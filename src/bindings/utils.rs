use rusty_v8 as v8;
use rusty_v8::FunctionCallback;
use rusty_v8::MapFnTo;

pub fn add_function<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    context: &v8::Local<'sc, v8::Context>,
    obj: &v8::Local<'sc, v8::Object>,
    name: &str,
    function: impl MapFnTo<FunctionCallback>,
) {
    let mut function_template = v8::FunctionTemplate::new(scope, function);
    let function_val = function_template.get_function(scope, *context).unwrap();
    let function_name = v8::String::new(scope, name).unwrap().into();
    obj.set(*context, function_name, function_val.into());
}
