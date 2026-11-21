//! Initialization, subscriptions, and IPC/input event handlers.

mod audio;
mod bluetooth;
mod disk;
mod drun;
mod events;
mod init;
mod ipc;
mod mpris;
mod notifications;
mod quick_menu;
mod subscriptions;
mod update;

pub use init::init_state;
pub use subscriptions::build_subscriptions;
pub use update::update_state;
