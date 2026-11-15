//! Application state, formatting models, messages, and event handlers.

pub mod format;
pub mod handlers;
pub mod msg;
pub mod scroller;
pub mod state;
pub mod tray_cache;
pub mod types;

pub use msg::Msg;
pub use state::State;
