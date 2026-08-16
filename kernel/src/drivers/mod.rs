//! Hardware-neutral driver contracts and adapters.

use core::cell::RefCell;

use crate::storage::{Block, BlockAddress, BlockReader, StorageError};
use embedded_sdmmc::{Block as FilesystemBlock, BlockCount, BlockDevice, BlockIdx};

/// Adapts a bounded mutable block reader to the filesystem block-device API.
///
/// The adapter owns the reader and borrows it for one bounded block operation
/// at a time. Hardware drivers therefore implement only the Dali storage
/// contract and never need to depend on filesystem policy.
pub struct BlockDeviceAdapter<R> {
    reader: RefCell<R>,
}

impl<R> BlockDeviceAdapter<R> {
    /// Wraps an initialized block reader for read-only filesystem use.
    pub const fn new(reader: R) -> Self {
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

#[cfg(test)]
mod tests {
    use super::BlockDeviceAdapter;
    use crate::storage::{Block, BlockAddress, BlockReader, StorageError};
    use embedded_sdmmc::{Block as FilesystemBlock, BlockDevice, BlockIdx};

    struct MockReader {
        blocks: [Block; 2],
    }

    impl BlockReader for MockReader {
        fn read_block(
            &mut self,
            address: BlockAddress,
            buffer: &mut Block,
        ) -> Result<(), StorageError> {
            let source = self
                .blocks
                .get(address.value() as usize)
                .ok_or(StorageError::InvalidBlockAddress)?;
            buffer.copy_from_slice(source);
            Ok(())
        }

        fn block_count(&self) -> Result<u32, StorageError> {
            Ok(self.blocks.len() as u32)
        }
    }

    #[test]
    fn adapts_bounded_reads_without_hardware_dependencies() {
        let reader = MockReader {
            blocks: [
                [0x11; crate::storage::BLOCK_SIZE],
                [0x22; crate::storage::BLOCK_SIZE],
            ],
        };
        let device = BlockDeviceAdapter::new(reader);
        let mut blocks: [FilesystemBlock; 2] = core::array::from_fn(|_| FilesystemBlock {
            contents: [0; crate::storage::BLOCK_SIZE],
        });

        device
            .read(&mut blocks, BlockIdx(0))
            .expect("mock read succeeds");

        assert_eq!(blocks[0].contents, [0x11; crate::storage::BLOCK_SIZE]);
        assert_eq!(blocks[1].contents, [0x22; crate::storage::BLOCK_SIZE]);
        assert_eq!(device.num_blocks().expect("count succeeds").0, 2);
    }

    #[test]
    fn keeps_the_read_only_contract() {
        let reader = MockReader {
            blocks: [[0; crate::storage::BLOCK_SIZE]; 2],
        };
        let device = BlockDeviceAdapter::new(reader);
        let blocks = [FilesystemBlock {
            contents: [0; crate::storage::BLOCK_SIZE],
        }];

        assert_eq!(
            device.write(&blocks, BlockIdx(0)),
            Err(StorageError::Unsupported)
        );
    }
}
