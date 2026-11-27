//! Initialization, subscriptions, and IPC/input event handlers.

mod annotate;
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
mod settings;
mod subscriptions;
mod tools;
mod update;
mod zoom;

pub use init::init_state;
pub use subscriptions::build_subscriptions;
pub use update::update_state;
