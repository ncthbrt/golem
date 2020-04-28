use std::ops::{Deref, DerefMut};
use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::{Mutex, Arc, Once};
use std::cell::RefCell;
use std::rc::Rc;
use crate::resources::ResourceTable;
use futures::stream::FuturesUnordered;
use crate::bindings;
use std::ffi::c_void;
use crate::ops::OpRegistry;

static V8_INIT: Once = Once::new();

/// A ZeroCopyBuf encapsulates a slice that's been borrowed from a JavaScript
/// ArrayBuffer object. JavaScript objects can normally be garbage collected,
/// but the existence of a ZeroCopyBuf inhibits this until it is dropped. It
/// behaves much like an Arc<[u8]>, although a ZeroCopyBuf currently can't be
/// cloned.
pub struct ZeroCopyBuf {
    backing_store: v8::SharedRef<v8::BackingStore>,
    byte_offset: usize,
    byte_length: usize,
}

unsafe impl Send for ZeroCopyBuf {}


impl ZeroCopyBuf {
    pub fn new(view: v8::Local<v8::ArrayBufferView>) -> Self {
        let backing_store = view.buffer().unwrap().get_backing_store();
        let byte_offset = view.byte_offset();
        let byte_length = view.byte_length();
        Self {
            backing_store,
            byte_offset,
            byte_length,
        }
    }
}


impl Deref for ZeroCopyBuf {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe {
            bindings::get_backing_store_slice(
                &self.backing_store,
                self.byte_offset,
                self.byte_length,
            )
        }
    }
}


impl DerefMut for ZeroCopyBuf {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            bindings::get_backing_store_slice_mut(
                &self.backing_store,
                self.byte_offset,
                self.byte_length,
            )
        }
    }
}

impl AsRef<[u8]> for ZeroCopyBuf {
    fn as_ref(&self) -> &[u8] {
        &*self
    }
}

impl AsMut<[u8]> for ZeroCopyBuf {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut *self
    }
}

pub enum Snapshot {
    Static(&'static [u8]),
    JustCreated(v8::StartupData),
}


/// Stores a script used to initalize a Isolate
pub struct Script<'a> {
    pub source: &'a str,
    pub filename: &'a str,
}


// TODO(ry) It's ugly that we have both Script and OwnedScript. Ideally we
// wouldn't expose such twiddly complexity.
struct OwnedScript {
    pub source: String,
    pub filename: String,
}

impl From<Script<'_>> for OwnedScript {
    fn from(s: Script) -> OwnedScript {
        OwnedScript {
            source: s.source.to_string(),
            filename: s.filename.to_string(),
        }
    }
}


/// Represents data used to initialize isolate at startup
/// either a binary snapshot or a javascript source file
/// in the form of the StartupScript struct.
pub enum StartupData<'a> {
    Script(Script<'a>),
    Snapshot(Snapshot),
    None,
}

pub struct IsolateCore {
    pub v8_isolate: Option<v8::OwnedIsolate>,
    snapshot_creator: Option<v8::SnapshotCreator>,
    has_snapshotted: bool,
    pub resource_table: Rc<RefCell<ResourceTable>>,
    pub global_context: v8::Global<v8::Context>,
    pub(crate) shared_ab: v8::Global<v8::SharedArrayBuffer>,
    pub(crate) js_recv_cb: v8::Global<v8::Function>,
    pub(crate) js_macrotask_cb: v8::Global<v8::Function>,
    pub(crate) pending_promise_exceptions: HashMap<i32, v8::Global<v8::Value>>,
    shared_isolate_handle: Arc<Mutex<Option<*mut v8::Isolate>>>,
    pub(crate) js_error_create_fn: Box<JSErrorCreateFn>,
    needs_init: bool,
    pub(crate) shared: SharedQueue,
    pending_ops: FuturesUnordered<PendingOpFuture>,
    pending_unref_ops: FuturesUnordered<PendingOpFuture>,
    have_unpolled_ops: bool,
    startup_script: Option<OwnedScript>,
    pub op_registry: OpRegistry,
    waker: AtomicWaker,
    error_handler: Option<Box<IsolateErrorHandleFn>>,
}


#[allow(clippy::missing_safety_doc)]
pub unsafe fn v8_init() {
    let platform = v8::new_default_platform().unwrap();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    // TODO(ry) This makes WASM compile synchronously. Eventually we should
    // remove this to make it work asynchronously too. But that requires getting
    // PumpMessageLoop and RunMicrotasks setup correctly.
    // See https://github.com/denoland/deno/issues/2544
    let argv = vec![
        "".to_string(),
        "--no-wasm-async-compilation".to_string(),
        "--harmony-top-level-await".to_string(),
    ];
    v8::V8::set_flags_from_command_line(argv);
}


impl IsolateCore {
    pub fn new(startup_data: StartupData, will_snapshot: bool) -> Box<Self> {
        V8_INIT.call_once(|| {
            unsafe { v8_init() };
        });

        let (startup_script, startup_snapshot) = match startup_data {
            StartupData::Script(script) => (Some(script.into()), None),
            StartupData::Snapshot(snapshot) => (None, Some(snapshot)),
            StartupData::None => (None, None),
        };

        let mut global_context = v8::Global::<v8::Context>::new();
        let (mut isolate, maybe_snapshot_creator) = if will_snapshot {
            assert!(startup_snapshot.is_none());
            let mut creator =
                v8::SnapshotCreator::new(Some(&bindings::EXTERNAL_REFERENCES));
            let isolate = unsafe { creator.get_owned_isolate() };
            let mut isolate = IsolateCore::setup_isolate(isolate);

            let mut hs = v8::HandleScope::new(&mut isolate);
            let scope = hs.enter();

            let context = bindings::initialize_context(scope);
            global_context.set(scope, context);
            creator.set_default_context(context);

            (isolate, Some(creator))
        } else {
            let mut params = v8::Isolate::create_params()
                .external_references(&**bindings::EXTERNAL_REFERENCES);
            let snapshot_loaded = if let Some(snapshot) = startup_snapshot {
                params = match snapshot {
                    Snapshot::Static(data) => params.snapshot_blob(data),
                    Snapshot::JustCreated(data) => params.snapshot_blob(data),
                };
                true
            } else {
                false
            };

            let isolate = v8::Isolate::new(params);
            let mut isolate = IsolateCore::setup_isolate(isolate);

            let mut hs = v8::HandleScope::new(&mut isolate);
            let scope = hs.enter();

            let context = if snapshot_loaded {
                v8::Context::new(scope)
            } else {
                // If no snapshot is provided, we initialize the context with empty
                // main source code and source maps.
                bindings::initialize_context(scope)
            };
            global_context.set(scope, context);

            (isolate, None)
        };

        let shared = SharedQueue::new(RECOMMENDED_SIZE);
        let needs_init = true;

        let core_isolate = Self {
            v8_isolate: None,
            global_context,
            resource_table: Rc::new(RefCell::new(ResourceTable::default())),
            pending_promise_exceptions: HashMap::new(),
            shared_ab: v8::Global::<v8::SharedArrayBuffer>::new(),
            js_recv_cb: v8::Global::<v8::Function>::new(),
            js_macrotask_cb: v8::Global::<v8::Function>::new(),
            snapshot_creator: maybe_snapshot_creator,
            has_snapshotted: false,
            shared_isolate_handle: Arc::new(Mutex::new(None)),
            js_error_create_fn: Box::new(JSError::create),
            shared,
            needs_init,
            pending_ops: FuturesUnordered::new(),
            pending_unref_ops: FuturesUnordered::new(),
            have_unpolled_ops: false,
            startup_script,
            op_registry: OpRegistry::new(),
            waker: AtomicWaker::new(),
            error_handler: None,
        };

        let mut boxed_isolate = Box::new(core_isolate);
        {
            let core_isolate_ptr: *mut Self = Box::into_raw(boxed_isolate);
            unsafe { isolate.set_data(0, core_isolate_ptr as *mut c_void) };
            boxed_isolate = unsafe { Box::from_raw(core_isolate_ptr) };
            let shared_handle_ptr = &mut *isolate;
            *boxed_isolate.shared_isolate_handle.lock().unwrap() =
                Some(shared_handle_ptr);
            boxed_isolate.v8_isolate = Some(isolate);
        }

        boxed_isolate
    }
}