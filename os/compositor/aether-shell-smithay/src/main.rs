//! Linux-only Smithay backend for Aether.
//!
//!   aether-shell-smithay            client-capable Wayland compositor window
//!   aether-shell-smithay --demo     solid FramePlan preview (no clients)
//!   aether-shell-smithay --frames N exit after N iterations

mod agents;
mod client;
mod damage;
mod demo;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().any(|arg| arg == "--demo") {
        demo::run()
    } else {
        client::run()
    }
}
