use std::mem;
use crate::isolate_core::{IsolateCore, StartupData};

pub struct GolemIsolate {
    isolate_core: Option<Box<IsolateCore>>,
    // main_handle:
}

impl GolemIsolate {
    pub fn try_new() -> Option<Self> {
        let isolate = Self {
            isolate_core: Some(IsolateCore::new(StartupData::None, false)),
        };

        Some(isolate)
    }
}

impl Drop for GolemIsolate {
    fn drop(&mut self) {
        let isolate = self.isolate_core.take().unwrap();
        drop(isolate);
    }
}