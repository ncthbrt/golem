use rusty_v8 as v8;
use std::cell::Cell;
use std::convert::TryFrom;
use crate::isolate_core::{ZeroCopyBuf, IsolateCore};
use v8::MapFnTo;

mod console;
mod utils;

lazy_static! {
  pub static ref EXTERNAL_REFERENCES: v8::ExternalReferences =
    v8::ExternalReferences::new(&[
      v8::ExternalReference {
        function: recv.map_fn_to()
      },
      v8::ExternalReference {
        function: send.map_fn_to()
      },
      v8::ExternalReference {
        function: encode.map_fn_to()
      },
      v8::ExternalReference {
        function: decode.map_fn_to()
      }
    ]);
}


pub fn script_origin<'a>(
    s: &mut impl v8::ToLocal<'a>,
    resource_name: v8::Local<'a, v8::String>,
) -> v8::ScriptOrigin<'a> {
    let resource_line_offset = v8::Integer::new(s, 0);
    let resource_column_offset = v8::Integer::new(s, 0);
    let resource_is_shared_cross_origin = v8::Boolean::new(s, false);
    let script_id = v8::Integer::new(s, 123);
    let source_map_url = v8::String::new(s, "source_map_url").unwrap();
    let resource_is_opaque = v8::Boolean::new(s, true);
    let is_wasm = v8::Boolean::new(s, false);
    let is_module = v8::Boolean::new(s, false);
    v8::ScriptOrigin::new(
        resource_name.into(),
        resource_line_offset,
        resource_column_offset,
        resource_is_shared_cross_origin,
        script_id,
        source_map_url.into(),
        resource_is_opaque,
        is_wasm,
        is_module,
    )
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
        let msg = v8::String::new(scope, "Golem.core.recv already called.").unwrap();
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


pub extern "C" fn promise_reject_callback(message: v8::PromiseRejectMessage) {
    let mut cbs = v8::CallbackScope::new(&message);
    let mut hs = v8::HandleScope::new(cbs.enter());
    let scope = hs.enter();

    let core_isolate: &mut IsolateCore =
        unsafe { &mut *(scope.isolate().get_data(0) as *mut IsolateCore) };

    let context = core_isolate.global_context.get(scope).unwrap();
    let mut cs = v8::ContextScope::new(scope, context);
    let scope = cs.enter();

    let promise = message.get_promise();
    let promise_id = promise.get_identity_hash();

    match message.get_event() {
        v8::PromiseRejectEvent::PromiseRejectWithNoHandler => {
            let error = message.get_value();
            let mut error_global = v8::Global::<v8::Value>::new();
            error_global.set(scope, error);
            core_isolate
                .pending_promise_exceptions
                .insert(promise_id, error_global);
        }
        v8::PromiseRejectEvent::PromiseHandlerAddedAfterReject => {
            if let Some(mut handle) =
            core_isolate.pending_promise_exceptions.remove(&promise_id)
            {
                handle.reset(scope);
            }
        }
        v8::PromiseRejectEvent::PromiseRejectAfterResolved => {}
        v8::PromiseRejectEvent::PromiseResolveAfterResolved => {
            // Should not warn. See #1272
        }
    };
}


fn shared_getter(
    scope: v8::PropertyCallbackScope,
    _name: v8::Local<v8::Name>,
    _args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let isolate_core: &mut IsolateCore =
        unsafe { &mut *(scope.isolate().get_data(0) as *mut IsolateCore) };

    // Lazily initialize the persistent external ArrayBuffer.
    if isolate_core.shared_ab.is_empty() {
        let ab = v8::SharedArrayBuffer::with_backing_store(
            scope,
            isolate_core.shared.get_backing_store(),
        );
        isolate_core.shared_ab.set(scope, ab);
    }

    let shared_ab = isolate_core.shared_ab.get(scope).unwrap();
    rv.set(shared_ab.into());
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

    console::inject_console(scope, &context, &global);


    core_val.set_accessor(
        context,
        v8::String::new(scope, "shared").unwrap().into(),
        shared_getter,
    );

    scope.escape(context)
}
