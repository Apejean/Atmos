pub mod api;
mod frb_generated;

pub mod audio;
pub mod common;
pub mod core;
pub mod osc;

#[macro_export]
macro_rules! log_print {
    ($($arg:tt)*) => {
        crate::core::state::GLOBAL_STATE.log(format!($($arg)*))
    };
}
