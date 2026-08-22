//! Hardware-neutral storage lifecycle transitions.

mod state;
mod transition;

pub mod policy;
pub use state::{StorageLifecycleEvent, StorageLifecycleState};
pub use transition::StorageLifecycle;
