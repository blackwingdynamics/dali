//! Hardware-neutral driver contracts and adapters.

use core::cell::RefCell;

pub mod block;
pub mod lifecycle;
pub mod sdio;

#[cfg(feature = "storage-write")]
pub use block::BlockTransportFlush;
#[cfg(feature = "storage-write")]
pub use block::BlockWriter;
pub use block::{BLOCK_SIZE, Block, BlockAddress, BlockReader, FlushableBlockDevice, StorageError};
use embedded_sdmmc::{
    Block as FilesystemBlock, BlockCount, BlockDevice as FilesystemBlockDevice, BlockIdx,
};
pub use sdio::{SdioBlockReader, SdioTransport, StorageLifecycleControl};

#[cfg(all(test, not(feature = "storage-write")))]
#[path = "tests/adapters.rs"]
mod adapter_tests;

/// Adapts a bounded mutable block reader to the filesystem block-device API.
///
/// The adapter owns the reader and borrows it for one bounded block operation
/// at a time. Hardware drivers therefore implement only the Dali storage
/// contract and never need to depend on filesystem policy.
#[cfg(not(feature = "storage-write"))]
pub struct BlockDeviceAdapter<R> {
    /// Stores the reader associated with this bounded state.
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

#[cfg(feature = "abi-current")]
impl<D> FlushableBlockDevice for BlockDeviceRef<'_, D>
where
    D: FlushableBlockDevice,
{
    type Error = D::Error;

    fn flush(&self) -> Result<(), Self::Error> {
        self.device.flush()
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
impl<R> FlushableBlockDevice for WritableBlockDeviceAdapter<R>
where
    R: BlockTransportFlush,
{
    type Error = R::Error;

    fn flush(&self) -> Result<(), Self::Error> {
        self.transport.borrow_mut().flush()
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

#[cfg(feature = "storage-write")]
impl<R> FlushableBlockDevice for &WritableBlockDeviceAdapter<R>
where
    R: BlockTransportFlush,
{
    type Error = R::Error;

    fn flush(&self) -> Result<(), Self::Error> {
        (**self).flush()
    }
}
