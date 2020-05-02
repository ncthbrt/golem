use crate::isolate_core::{IsolateCore, StartupData};

pub struct GolemIsolate {
    isolate_core: Box<IsolateCore>
}

impl GolemIsolate {
    pub fn new() -> Self {
        let mut isolate = Self {
            isolate_core: IsolateCore::new(StartupData::None, false),
        };

        isolate
    }
}