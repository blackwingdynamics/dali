//! Hardware-neutral block transport contracts.

pub use dali_kernel_api::storage::{
    BLOCK_SIZE, Block, BlockAddress, BlockReader, BlockTransportFlush, BlockWriter,
    FlushableBlockDevice, StorageError,
};
