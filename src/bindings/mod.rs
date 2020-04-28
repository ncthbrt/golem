use rusty_v8 as v8;
use std::cell::Cell;
use std::convert::TryFrom;
use crate::isolate_core::{ZeroCopyBuf, IsolateCore};
// mod console;
// mod fetch;
// mod utils;

pub fn inject_bindings<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    context: &v8::Local<'sc, v8::Context>,
) {
    let global = context.global(scope);
    // console::inject_console(scope, context, &global);
    // fetch::inject_fetch(scope, context, &global);
}


pub(crate) unsafe fn get_backing_store_slice(
    backing_store: &v8::SharedRef<v8::BackingStore>,
    byte_offset: usize,
    byte_length: usize,
) -> &[u8] {
    let cells: *const [Cell<u8>] =
        &backing_store[byte_offset..byte_offset + byte_length];
    let bytes = cells as *const [u8];
    &*bytes
}


#[allow(clippy::mut_from_ref)]
pub(crate) unsafe fn get_backing_store_slice_mut(
    backing_store: &v8::SharedRef<v8::BackingStore>,
    byte_offset: usize,
    byte_length: usize,
) -> &mut [u8] {
    let cells: *const [Cell<u8>] =
        &backing_store[byte_offset..byte_offset + byte_length];
    let bytes = cells as *const _ as *mut [u8];
    &mut *bytes
}

fn encode(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let text = match v8::Local::<v8::String>::try_from(args.get(0)) {
        Ok(s) => s,
        Err(_) => {
            let msg = v8::String::new(scope, "Invalid argument").unwrap();
            let exception = v8::Exception::type_error(scope, msg);
            scope.isolate().throw_exception(exception);
            return;
        }
    };
    let text_str = text.to_rust_string_lossy(scope);
    let text_bytes = text_str.as_bytes().to_vec().into_boxed_slice();

    let buf = if text_bytes.is_empty() {
        let ab = v8::ArrayBuffer::new(scope, 0);
        v8::Uint8Array::new(ab, 0, 0).expect("Failed to create UintArray8")
    } else {
        let buf_len = text_bytes.len();
        let backing_store =
            v8::ArrayBuffer::new_backing_store_from_boxed_slice(text_bytes);
        let backing_store_shared = backing_store.make_shared();
        let ab = v8::ArrayBuffer::with_backing_store(scope, &backing_store_shared);
        v8::Uint8Array::new(ab, 0, buf_len).expect("Failed to create UintArray8")
    };

    rv.set(buf.into())
}

fn decode(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let view = match v8::Local::<v8::ArrayBufferView>::try_from(args.get(0)) {
        Ok(view) => view,
        Err(_) => {
            let msg = v8::String::new(scope, "Invalid argument").unwrap();
            let exception = v8::Exception::type_error(scope, msg);
            scope.isolate().throw_exception(exception);
            return;
        }
    };

    let backing_store = view.buffer().unwrap().get_backing_store();
    let buf = unsafe {
        get_backing_store_slice(
            &backing_store,
            view.byte_offset(),
            view.byte_length(),
        )
    };

    let text_str =
        v8::String::new_from_utf8(scope, &buf, v8::NewStringType::Normal).unwrap();
    rv.set(text_str.into())
}


fn recv(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let core_isolate: &mut IsolateCore =
        unsafe { &mut *(scope.isolate().get_data(0) as *mut IsolateCore) };

    if !core_isolate.js_recv_cb.is_empty() {
        let msg = v8::String::new(scope, "Deno.core.recv already called.").unwrap();
        scope.isolate().throw_exception(msg.into());
        return;
    }

    let recv_fn = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
    core_isolate.js_recv_cb.set(scope, recv_fn);
}

fn send(
    scope: v8::FunctionCallbackScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let core_isolate: &mut IsolateCore =
        unsafe { &mut *(scope.isolate().get_data(0) as *mut IsolateCore) };
    assert!(!core_isolate.global_context.is_empty());

    let op_id = match v8::Local::<v8::Uint32>::try_from(args.get(0)) {
        Ok(op_id) => op_id.value() as u32,
        Err(err) => {
            let msg = format!("invalid op id: {}", err);
            let msg = v8::String::new(scope, &msg).unwrap();
            scope.isolate().throw_exception(msg.into());
            return;
        }
    };

    let control_backing_store: v8::SharedRef<v8::BackingStore>;
    let control = match v8::Local::<v8::ArrayBufferView>::try_from(args.get(1)) {
        Ok(view) => unsafe {
            control_backing_store = view.buffer().unwrap().get_backing_store();
            get_backing_store_slice(
                &control_backing_store,
                view.byte_offset(),
                view.byte_length(),
            )
        },
        Err(_) => &[],
    };

    let zero_copy: Option<ZeroCopyBuf> =
        v8::Local::<v8::ArrayBufferView>::try_from(args.get(2))
            .map(ZeroCopyBuf::new)
            .ok();

    // If response is empty then it's either async op or exception was thrown
    let maybe_response =
        core_isolate.dispatch_op(scope, op_id, control, zero_copy);

    if let Some(response) = maybe_response {
        // Synchronous response.
        // Note op_id is not passed back in the case of synchronous response.
        let (_op_id, buf) = response;

        if !buf.is_empty() {
            let ui8 = boxed_slice_to_uint8array(scope, buf);
            rv.set(ui8.into())
        }
    }
}


pub fn boxed_slice_to_uint8array<'sc>(
    scope: &mut impl v8::ToLocal<'sc>,
    buf: Box<[u8]>,
) -> v8::Local<'sc, v8::Uint8Array> {
    assert!(!buf.is_empty());
    let buf_len = buf.len();
    let backing_store = v8::ArrayBuffer::new_backing_store_from_boxed_slice(buf);
    let backing_store_shared = backing_store.make_shared();
    let ab = v8::ArrayBuffer::with_backing_store(scope, &backing_store_shared);
    v8::Uint8Array::new(ab, 0, buf_len).expect("Failed to create UintArray8")
}

pub fn initialize_context<'s>(
    scope: &mut impl v8::ToLocal<'s>,
) -> v8::Local<'s, v8::Context> {
    let mut hs = v8::EscapableHandleScope::new(scope);
    let scope = hs.enter();

    let context = v8::Context::new(scope);
    let global = context.global(scope);

    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let golem_val = v8::Object::new(scope);
    global.set(
        context,
        v8::String::new(scope, "Golem").unwrap().into(),
        golem_val.into(),
    );

    let mut core_val = v8::Object::new(scope);
    golem_val.set(
        context,
        v8::String::new(scope, "core").unwrap().into(),
        core_val.into(),
    );


    let mut recv_tmpl = v8::FunctionTemplate::new(scope, recv);
    let recv_val = recv_tmpl.get_function(scope, context).unwrap();
    core_val.set(
        context,
        v8::String::new(scope, "recv").unwrap().into(),
        recv_val.into(),
    );

    let mut send_tmpl = v8::FunctionTemplate::new(scope, send);
    let send_val = send_tmpl.get_function(scope, context).unwrap();
    core_val.set(
        context,
        v8::String::new(scope, "send").unwrap().into(),
        send_val.into(),
    );

    let mut encode_tmpl = v8::FunctionTemplate::new(scope, encode);
    let encode_val = encode_tmpl.get_function(scope, context).unwrap();
    core_val.set(
        context,
        v8::String::new(scope, "encode").unwrap().into(),
        encode_val.into(),
    );

    let mut decode_tmpl = v8::FunctionTemplate::new(scope, decode);
    let decode_val = decode_tmpl.get_function(scope, context).unwrap();
    core_val.set(
        context,
        v8::String::new(scope, "decode").unwrap().into(),
        decode_val.into(),
    );


    scope.escape(context)
}
