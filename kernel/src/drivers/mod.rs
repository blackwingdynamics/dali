//! Hardware-neutral driver contracts and adapters.

use core::cell::RefCell;

pub mod block;
pub mod sdio;

#[cfg(feature = "storage-write")]
pub use block::BlockWriter;
pub use block::{BLOCK_SIZE, Block, BlockAddress, BlockReader, StorageError};
use embedded_sdmmc::{
    Block as FilesystemBlock, BlockCount, BlockDevice as FilesystemBlockDevice, BlockIdx,
};
pub use sdio::{SdioBlockReader, SdioTransport};

/// Adapts a bounded mutable block reader to the filesystem block-device API.
///
/// The adapter owns the reader and borrows it for one bounded block operation
/// at a time. Hardware drivers therefore implement only the Dali storage
/// contract and never need to depend on filesystem policy.
#[cfg(not(feature = "storage-write"))]
pub struct BlockDeviceAdapter<R> {
    reader: RefCell<R>,
}

#[cfg(feature = "storage-write")]
/// Adapts an explicitly read/write block transport to the filesystem API.
pub struct WritableBlockDeviceAdapter<R> {
    transport: RefCell<R>,
}

/// Borrows an existing block device for repeated read-only filesystem passes.
#[cfg(feature = "abi-current")]
pub struct BlockDeviceRef<'a, D> {
    device: &'a D,
}

#[cfg(feature = "abi-current")]
impl<D> Copy for BlockDeviceRef<'_, D> {}

#[cfg(feature = "abi-current")]
impl<D> Clone for BlockDeviceRef<'_, D> {
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(feature = "abi-current")]
impl<'a, D> BlockDeviceRef<'a, D> {
    /// Creates a read-only borrowed view of an initialized block device.
    pub const fn new(device: &'a D) -> Self {
        Self { device }
    }
}

#[cfg(feature = "abi-current")]
impl<D> FilesystemBlockDevice for BlockDeviceRef<'_, D>
where
    D: FilesystemBlockDevice,
{
    type Error = D::Error;

    fn read(
        &self,
        blocks: &mut [FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        self.device.read(blocks, start_block_idx)
    }

    fn write(
        &self,
        blocks: &[FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        self.device.write(blocks, start_block_idx)
    }

    fn num_blocks(&self) -> Result<BlockCount, Self::Error> {
        self.device.num_blocks()
    }
}

#[cfg(not(feature = "storage-write"))]
impl<R> BlockDeviceAdapter<R> {
    /// Wraps an initialized block reader for read-only filesystem use.
    pub const fn new(reader: R) -> Self {
        Self {
            reader: RefCell::new(reader),
        }
    }
}

#[cfg(not(feature = "storage-write"))]
impl<R> FilesystemBlockDevice for BlockDeviceAdapter<R>
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
            let mut buffer: Block = [0; BLOCK_SIZE];
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

#[cfg(feature = "storage-write")]
impl<R> WritableBlockDeviceAdapter<R> {
    /// Wraps an initialized transport that explicitly supports block writes.
    pub const fn new(transport: R) -> Self {
        Self {
            transport: RefCell::new(transport),
        }
    }
}

#[cfg(feature = "storage-write")]
impl<R> FilesystemBlockDevice for WritableBlockDeviceAdapter<R>
where
    R: BlockReader + BlockWriter,
{
    type Error = StorageError;

    fn read(
        &self,
        blocks: &mut [FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        let mut transport = self.transport.borrow_mut();
        for (offset, block) in blocks.iter_mut().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| StorageError::InvalidBlockAddress)?;
            let address = start_block_idx
                .0
                .checked_add(offset)
                .ok_or(StorageError::InvalidBlockAddress)?;
            transport.read_block(BlockAddress::new(address), &mut block.contents)?;
        }
        Ok(())
    }

    fn write(
        &self,
        blocks: &[FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        let mut transport = self.transport.borrow_mut();
        for (offset, block) in blocks.iter().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| StorageError::InvalidBlockAddress)?;
            let address = start_block_idx
                .0
                .checked_add(offset)
                .ok_or(StorageError::InvalidBlockAddress)?;
            transport.write_block(BlockAddress::new(address), &block.contents)?;
        }
        Ok(())
    }

    fn num_blocks(&self) -> Result<BlockCount, Self::Error> {
        self.transport.borrow().block_count().map(BlockCount)
    }
}

#[cfg(feature = "storage-write")]
impl<R> FilesystemBlockDevice for &WritableBlockDeviceAdapter<R>
where
    R: BlockReader + BlockWriter,
{
    type Error = StorageError;

    fn read(
        &self,
        blocks: &mut [FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        (**self).read(blocks, start_block_idx)
    }

    fn write(
        &self,
        blocks: &[FilesystemBlock],
        start_block_idx: BlockIdx,
    ) -> Result<(), Self::Error> {
        (**self).write(blocks, start_block_idx)
    }

    fn num_blocks(&self) -> Result<BlockCount, Self::Error> {
        (**self).num_blocks()
    }
}

#[cfg(test)]
mod tests {
    struct MockReader {
        blocks: [super::Block; 2],
    }

    impl super::BlockReader for MockReader {
        fn read_block(
            &mut self,
            address: super::BlockAddress,
            buffer: &mut super::Block,
        ) -> Result<(), super::StorageError> {
            let source = self
                .blocks
                .get(address.value() as usize)
                .ok_or(super::StorageError::InvalidBlockAddress)?;
            buffer.copy_from_slice(source);
            Ok(())
        }

        fn block_count(&self) -> Result<u32, super::StorageError> {
            Ok(self.blocks.len() as u32)
        }
    }

    #[test]
    fn adapts_bounded_reads_without_hardware_dependencies() {
        let reader = MockReader {
            blocks: [[0x11; super::BLOCK_SIZE], [0x22; super::BLOCK_SIZE]],
        };
        let device = super::BlockDeviceAdapter::new(reader);
        let mut blocks: [embedded_sdmmc::Block; 2] =
            core::array::from_fn(|_| embedded_sdmmc::Block {
                contents: [0; super::BLOCK_SIZE],
            });

        <super::BlockDeviceAdapter<MockReader> as embedded_sdmmc::BlockDevice>::read(
            &device,
            &mut blocks,
            embedded_sdmmc::BlockIdx(0),
        )
        .expect("mock read succeeds");

        assert_eq!(blocks[0].contents, [0x11; super::BLOCK_SIZE]);
        assert_eq!(blocks[1].contents, [0x22; super::BLOCK_SIZE]);
        assert_eq!(
            <super::BlockDeviceAdapter<MockReader> as embedded_sdmmc::BlockDevice>::num_blocks(
                &device,
            )
            .expect("count succeeds")
            .0,
            2
        );
    }

    #[test]
    fn keeps_the_read_only_contract() {
        let reader = MockReader {
            blocks: [[0; super::BLOCK_SIZE]; 2],
        };
        let device = super::BlockDeviceAdapter::new(reader);
        let blocks = [embedded_sdmmc::Block {
            contents: [0; super::BLOCK_SIZE],
        }];

        assert_eq!(
            <super::BlockDeviceAdapter<MockReader> as embedded_sdmmc::BlockDevice>::write(
                &device,
                &blocks,
                embedded_sdmmc::BlockIdx(0),
            ),
            Err(super::StorageError::Unsupported)
        );
    }
}
