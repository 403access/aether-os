//! Virtualised explorer state.

use crate::design;

/// Virtualised file explorer. `matches` caches filtered file indices so
/// only the visible slice enters the render plan.
#[derive(Clone, Debug)]
pub struct ExplorerState {
    pub(crate) query: String,
    pub(crate) selected: usize,
    pub(crate) scroll: usize,
    pub(crate) visible: usize,
    pub(crate) matches: Vec<usize>,
    pub(crate) dirty: bool,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected: 0,
            scroll: 0,
            visible: design::EXPLORER_VISIBLE_ROWS,
            matches: Vec::new(),
            dirty: true,
        }
    }
}
