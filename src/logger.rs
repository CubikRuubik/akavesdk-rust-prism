use std::fmt::Arguments;

#[cfg(target_arch = "wasm32")]
use web_sys::console;

#[cfg(not(target_arch = "wasm32"))]
use log::{info, error, warn, debug};


pub struct Logger;

impl Logger {
    pub fn info(args: Arguments) {
        #[cfg(target_arch = "wasm32")]
        {
            console::log_1(&wasm_bindgen::JsValue::from_str(&format!("{}", args)));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            info!("{}", args);
        }
    }

    pub fn error(args: Arguments) {
        #[cfg(target_arch = "wasm32")]
        {
            console::error_1(&wasm_bindgen::JsValue::from_str(&format!("{}", args)));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            error!("{}", args);
        }
    }

    pub fn warn(args: Arguments) {
        #[cfg(target_arch = "wasm32")]
        {
            console::warn_1(&wasm_bindgen::JsValue::from_str(&format!("{}", args)));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            warn!("{}", args);
        }
    }

    pub fn debug(args: Arguments) {
        #[cfg(target_arch = "wasm32")]
        {
            console::debug_1(&wasm_bindgen::JsValue::from_str(&format!("{}", args)));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            debug!("{}", args);
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        crate::logger::Logger::info(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        crate::logger::Logger::error(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        crate::logger::Logger::warn(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        crate::logger::Logger::debug(format_args!($($arg)*))
    };
} 