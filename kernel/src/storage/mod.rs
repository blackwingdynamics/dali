//! Transport-independent block-storage boundary.

pub mod filesystem;

/// Re-export the generic block contract for storage policy modules.
pub use crate::drivers::{BLOCK_SIZE, Block, BlockAddress, BlockReader, StorageError};
