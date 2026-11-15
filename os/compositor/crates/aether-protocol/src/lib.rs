//! Wire protocols shared by the Aether shell and its local services.
//!
//! Every local service speaks newline-delimited JSON over a Unix socket. The
//! message shapes used to live next to both the producer and the consumer,
//! which let them drift apart. They are defined once here instead:
//!
//! - [`chat`] — prompts and streamed tool events for the chat drawer.
//! - [`index`] — file-index queries and ranked hits.
//! - [`control`] — JSON-RPC-shaped shell control calls.
//! - [`line`] — framing helpers shared by all of the above.
//!
//! The crate is deliberately transport-only: it owns message types and small
//! socket helpers, not policy, retries or UI state.

#![forbid(unsafe_code)]

pub mod chat;
pub mod control;
pub mod index;
pub mod line;

pub use line::{read_json_line, write_json_line};
