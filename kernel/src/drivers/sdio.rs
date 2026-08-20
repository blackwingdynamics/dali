//! Hardware-neutral SDIO block-reader contract.

use super::{Block, BlockAddress, BlockReader, StorageError};
use crate::storage::durable::BlockDevice;

/// Hardware-facing SDIO transport implemented by a platform backend.
pub trait SdioTransport {
    /// Initializes the card and returns its addressable block count.
    fn initialize(&mut self) -> Result<u32, StorageError>;

    /// Reads one complete block from the initialized card.
    fn read_block(&mut self, address: BlockAddress, block: &mut Block) -> Result<(), StorageError>;

    /// Writes one complete block to the initialized card.
    #[cfg(feature = "storage-write")]
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError>;
}

/// Adapts any SDIO transport to the generic bounded block-reader contract.
pub struct SdioBlockReader<T> {
    transport: T,
    block_count: Option<u32>,
}

impl<T> SdioBlockReader<T> {
    /// Creates an uninitialized reader around a platform SDIO transport.
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            block_count: None,
        }
    }
}

impl<T> SdioBlockReader<T>
where
    T: SdioTransport,
{
    /// Initializes the card and records its bounded capacity.
    pub fn initialize(&mut self) -> Result<(), StorageError> {
        self.block_count = Some(self.transport.initialize()?);
        Ok(())
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
        self.transport.read_block(address, buffer)
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
        Ok(())
    }

    fn block_count(&self) -> Result<u32, Self::Error> {
        <Self as BlockReader>::block_count(self)
    }
}

#[cfg(feature = "storage-write")]
impl<T> super::BlockWriter for SdioBlockReader<T>
where
    T: SdioTransport,
{
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError> {
        self.transport.write_block(address, block)
    }
}

#[cfg(test)]
mod tests {
    use super::SdioTransport;

    const MOCK_BLOCK_COUNT: u32 = 4;
    const READ_BLOCK: u32 = 2;

    struct MockTransport {
        initialized: bool,
    }

    impl SdioTransport for MockTransport {
        fn initialize(&mut self) -> Result<u32, crate::drivers::StorageError> {
            self.initialized = true;
            Ok(MOCK_BLOCK_COUNT)
        }

        fn read_block(
            &mut self,
            address: crate::drivers::BlockAddress,
            block: &mut crate::drivers::Block,
        ) -> Result<(), crate::drivers::StorageError> {
            if !self.initialized {
                return Err(crate::drivers::StorageError::NotReady);
            }
            if address.value() >= MOCK_BLOCK_COUNT {
                return Err(crate::drivers::StorageError::InvalidBlockAddress);
            }
            block.fill(address.value() as u8);
            Ok(())
        }

        #[cfg(feature = "storage-write")]
        fn write_block(
            &mut self,
            _address: crate::drivers::BlockAddress,
            _block: &crate::drivers::Block,
        ) -> Result<(), crate::drivers::StorageError> {
            if self.initialized {
                Ok(())
            } else {
                Err(crate::drivers::StorageError::NotReady)
            }
        }
    }

    #[test]
    fn exposes_capacity_only_after_initialization() {
        let mut reader = super::SdioBlockReader::new(MockTransport { initialized: false });
        assert_eq!(
            <super::SdioBlockReader<MockTransport> as crate::drivers::BlockReader>::block_count(
                &reader,
            ),
            Err(crate::drivers::StorageError::NotReady)
        );

        reader.initialize().expect("mock SDIO initializes");

        assert_eq!(
            <super::SdioBlockReader<MockTransport> as crate::drivers::BlockReader>::block_count(
                &reader,
            ),
            Ok(MOCK_BLOCK_COUNT)
        );
    }

    #[test]
    fn delegates_bounded_reads_to_the_transport() {
        let mut reader = super::SdioBlockReader::new(MockTransport { initialized: false });
        let mut block = [0; crate::drivers::BLOCK_SIZE];

        reader.initialize().expect("mock SDIO initializes");
        <super::SdioBlockReader<MockTransport> as crate::drivers::BlockReader>::read_block(
            &mut reader,
            crate::drivers::BlockAddress::new(READ_BLOCK),
            &mut block,
        )
        .expect("mock block read succeeds");

        assert_eq!(block, [READ_BLOCK as u8; crate::drivers::BLOCK_SIZE]);
        assert_eq!(
            <super::SdioBlockReader<MockTransport> as crate::drivers::BlockReader>::read_block(
                &mut reader,
                crate::drivers::BlockAddress::new(MOCK_BLOCK_COUNT),
                &mut block,
            ),
            Err(crate::drivers::StorageError::InvalidBlockAddress)
        );
    }
}
