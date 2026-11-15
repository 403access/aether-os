//! Domain types shared by the shell state and its adapters.
//!
//! These modules hold the data and algorithms that are independent of the
//! compositor backend: chat transcripts, explorer state, frame plans, output
//! descriptors, palette providers and window descriptors.

pub(crate) mod chat;
pub(crate) mod explorer;
pub(crate) mod frame;
pub(crate) mod output;
pub(crate) mod palette;
pub(crate) mod window;
