//! Hardware-neutral SDIO block-reader contract.

use super::BlockTransportFlush;
use super::{
    Block, BlockAddress, BlockReader, StorageError,
    lifecycle::{StorageLifecycle, StorageLifecycleEvent, StorageLifecycleState},
};
use crate::storage::durable::BlockDevice;

pub use dali_kernel_api::storage::{SdioTransport, StorageLifecycleControl};

/// Adapts any SDIO transport to the generic bounded block-reader contract.
pub struct SdioBlockReader<T> {
    /// Stores the transport associated with this bounded state.
    transport: T,
    /// Stores the block count associated with this bounded state.
    block_count: Option<u32>,
    /// Stores the lifecycle associated with this bounded state.
    lifecycle: StorageLifecycle,
}

impl<T> SdioBlockReader<T> {
    /// Creates an uninitialized reader around a platform SDIO transport.
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            block_count: None,
            lifecycle: StorageLifecycle::new(),
        }
    }
}

impl<T> SdioBlockReader<T>
where
    T: SdioTransport,
{
    /// Initializes the card and records its bounded capacity.
    pub fn initialize(&mut self) -> Result<(), StorageError> {
        self.lifecycle
            .apply(StorageLifecycleEvent::InitializationStarted);
        self.block_count = None;
        match self.transport.initialize() {
            Ok(block_count) => {
                self.lifecycle
                    .apply(StorageLifecycleEvent::InitializationSucceeded);
                self.block_count = Some(block_count);
                Ok(())
            }
            Err(error) => {
                self.record_error(error);
                Err(error)
            }
        }
    }

    /// Returns the current lifecycle state without probing hardware.
    pub const fn lifecycle_state(&self) -> StorageLifecycleState {
        self.lifecycle.state()
    }

    /// Records the `record error` operation for this subsystem.
    ///
    /// Arguments select the bounded state, buffer, or hardware operation described by the signature.
    fn record_error(&mut self, error: StorageError) {
        let event = if matches!(error, StorageError::CardRemoved) {
            StorageLifecycleEvent::CardRemoved
        } else {
            StorageLifecycleEvent::OperationFailed
        };
        self.lifecycle.apply(event);
    }
}

impl<T> StorageLifecycleControl for SdioBlockReader<T>
where
    T: SdioTransport,
{
    fn initialize(&mut self) -> Result<(), StorageError> {
        Self::initialize(self)
    }

    fn reinitialize(&mut self) -> Result<(), StorageError> {
        Self::initialize(self)
    }
}

impl<T> BlockReader for SdioBlockReader<T>
where
    T: SdioTransport,
{
    fn read_block(
        &mut self,
        address: BlockAddress,
        buffer: &mut Block,
    ) -> Result<(), StorageError> {
        if self.lifecycle_state() != StorageLifecycleState::Ready {
            return Err(StorageError::NotReady);
        }
        match self.transport.read_block(address, buffer) {
            Ok(()) => {
                self.lifecycle
                    .apply(StorageLifecycleEvent::OperationSucceeded);
                Ok(())
            }
            Err(error) => {
                self.record_error(error);
                Err(error)
            }
        }
    }

    fn block_count(&self) -> Result<u32, StorageError> {
        self.block_count.ok_or(StorageError::NotReady)
    }
}

impl<T> BlockDevice for SdioBlockReader<T>
where
    T: SdioTransport,
{
    type Error = StorageError;

    fn read_blocks(
        &mut self,
        start: BlockAddress,
        blocks: &mut [Block],
    ) -> Result<(), Self::Error> {
        for (offset, block) in blocks.iter_mut().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| StorageError::InvalidBlockAddress)?;
            let address = start
                .value()
                .checked_add(offset)
                .ok_or(StorageError::InvalidBlockAddress)?;
            self.read_block(BlockAddress::new(address), block)?;
        }
        Ok(())
    }

    fn write_blocks(&mut self, start: BlockAddress, blocks: &[Block]) -> Result<(), Self::Error> {
        #[cfg(feature = "storage-write")]
        {
            for (offset, block) in blocks.iter().enumerate() {
                let offset =
                    u32::try_from(offset).map_err(|_| StorageError::InvalidBlockAddress)?;
                let address = start
                    .value()
                    .checked_add(offset)
                    .ok_or(StorageError::InvalidBlockAddress)?;
                <Self as super::BlockWriter>::write_block(self, BlockAddress::new(address), block)?;
            }
            Ok(())
        }
        #[cfg(not(feature = "storage-write"))]
        {
            let _ = (start, blocks);
            Err(StorageError::Unsupported)
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        #[cfg(feature = "storage-write")]
        {
            <Self as BlockTransportFlush>::flush(self)
        }
        #[cfg(not(feature = "storage-write"))]
        {
            Err(StorageError::Unsupported)
        }
    }

    fn block_count(&self) -> Result<u32, Self::Error> {
        <Self as BlockReader>::block_count(self)
    }
}

impl<T> BlockTransportFlush for SdioBlockReader<T>
where
    T: SdioTransport,
{
    type Error = StorageError;

    fn flush(&mut self) -> Result<(), Self::Error> {
        if self.lifecycle_state() != StorageLifecycleState::Ready {
            return Err(StorageError::NotReady);
        }
        match self.transport.flush() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.record_error(error);
                Err(error)
            }
        }
    }
}

impl<T> super::BlockWriter for SdioBlockReader<T>
where
    T: SdioTransport,
{
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError> {
        if self.lifecycle_state() != StorageLifecycleState::Ready {
            return Err(StorageError::NotReady);
        }
        match self.transport.write_block(address, block) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.record_error(error);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests;
