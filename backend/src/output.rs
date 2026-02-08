/// Cross-platform println macro.
/// On native: routes to `println!()`.
/// On WASM: routes to `web_sys::console::log_1()`.
#[cfg(not(target_arch = "wasm32"))]
macro_rules! engine_println {
    ($($arg:tt)*) => { println!($($arg)*) }
}

#[cfg(target_arch = "wasm32")]
macro_rules! engine_println {
    ($($arg:tt)*) => {
        web_sys::console::log_1(&format!($($arg)*).into())
    }
}

pub(crate) use engine_println;
