//! Compositor-owned surfaces layered above client windows.
//!
//! Each overlay owns its geometry, key handling and damage marking. `topbar`
//! is different from the rest: it is passive chrome, while the others are
//! modal surfaces that capture input.

pub(crate) mod audio;
pub(crate) mod chat;
pub(crate) mod explorer;
pub(crate) mod palette;
pub(crate) mod shortcuts;
pub(crate) mod task_manager;
pub(crate) mod topbar;
