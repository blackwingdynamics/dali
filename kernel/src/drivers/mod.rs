//! Hardware-neutral driver contracts and adapters.

use core::cell::RefCell;

use crate::storage::{Block, BlockAddress, BlockReader, StorageError};
use embedded_sdmmc::{Block as FilesystemBlock, BlockCount, BlockDevice, BlockIdx};

/// Adapts a bounded mutable block reader to the filesystem block-device API.
///
/// The adapter owns the reader and borrows it for one bounded block operation
/// at a time. Hardware drivers therefore implement only the Dali storage
/// contract and never need to depend on filesystem policy.
pub(crate) struct BlockDeviceAdapter<R> {
    reader: RefCell<R>,
}

impl<R> BlockDeviceAdapter<R> {
    /// Wraps an initialized block reader for read-only filesystem use.
    pub(crate) const fn new(reader: R) -> Self {
        Self {
            reader: RefCell::new(reader),
        }
    }
}

impl<R> BlockDevice for BlockDeviceAdapter<R>
where
    R: BlockReader,
{
    type Error = StorageError;

    fn read(
        &self,
        blocks: &mut [FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        let mut reader = self.reader.borrow_mut();
        for (offset, block) in blocks.iter_mut().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| StorageError::InvalidBlockAddress)?;
            let address = start_block_idx
                .0
                .checked_add(offset)
                .ok_or(StorageError::InvalidBlockAddress)?;
            let mut buffer: Block = [0; crate::storage::BLOCK_SIZE];
            reader.read_block(BlockAddress::new(address), &mut buffer)?;
            block.contents.copy_from_slice(&buffer);
        }
        Ok(())
    }

    fn write(
        &self,
        _blocks: &[FilesystemBlock],
        _start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        Err(StorageError::Unsupported)
    }

    fn num_blocks(&self) -> Result<BlockCount, Self::Error> {
        self.reader.borrow().block_count().map(BlockCount)
    }
}
