//! Client for the shell control socket.
//!
//! The request/response shapes live in [`aether_protocol::control`] so the
//! compositor and this client cannot drift apart.

pub use aether_protocol::control::Client as ControlClient;
