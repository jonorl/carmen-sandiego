//! The device's modules the game uses, with nothing behind them but memory.

#[path = "../../../../src/device/progress.rs"]
pub mod progress;
#[path = "../../../../src/device/tunes.rs"]
pub mod tunes;

pub mod sound {
    include!(concat!(env!("OUT_DIR"), "/effects.rs"));
}

pub mod storage {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static SAVED: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    }

    pub fn read(name: &str) -> Option<String> {
        SAVED.with(|s| s.borrow().get(name).cloned())
    }

    pub fn write(name: &str, text: &str) {
        SAVED.with(|s| s.borrow_mut().insert(name.to_string(), text.to_string()));
    }
}

/// No server: arrests stay on the "device".
pub mod net {
    pub type Answer = Result<String, String>;

    pub struct Net;

    impl Net {
        pub fn get(&mut self, _url: &str) -> u32 {
            0
        }

        pub fn put(&mut self, _url: &str, _body: String) -> u32 {
            0
        }

        pub fn poll(&self) -> Vec<(u32, Answer)> {
            Vec::new()
        }
    }

    pub fn server() -> Option<&'static str> {
        None
    }

    pub fn escape(name: &str) -> String {
        name.to_string()
    }
}
