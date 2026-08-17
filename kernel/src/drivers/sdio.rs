//! Hardware-neutral SDIO block-reader contract.

use super::{Block, BlockAddress, BlockReader, StorageError};

/// Hardware-facing SDIO transport implemented by a platform backend.
pub trait SdioTransport {
    /// Initializes the card and returns its addressable block count.
    fn initialize(&mut self) -> Result<u32, StorageError>;

    /// Reads one complete block from the initialized card.
    fn read_block(&mut self, address: BlockAddress, block: &mut Block) -> Result<(), StorageError>;
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

#[cfg(test)]
mod tests {
    use super::{SdioBlockReader, SdioTransport};
    use crate::drivers::{BLOCK_SIZE, Block, BlockAddress, BlockReader, StorageError};

    const MOCK_BLOCK_COUNT: u32 = 4;
    const READ_BLOCK: u32 = 2;

    struct MockTransport {
        initialized: bool,
    }

    impl SdioTransport for MockTransport {
        fn initialize(&mut self) -> Result<u32, StorageError> {
            self.initialized = true;
            Ok(MOCK_BLOCK_COUNT)
        }

        fn read_block(
            &mut self,
            address: BlockAddress,
            block: &mut Block,
        ) -> Result<(), StorageError> {
            if !self.initialized {
                return Err(StorageError::NotReady);
            }
            if address.value() >= MOCK_BLOCK_COUNT {
                return Err(StorageError::InvalidBlockAddress);
            }
            block.fill(address.value() as u8);
            Ok(())
        }
    }

    #[test]
    fn exposes_capacity_only_after_initialization() {
        let mut reader = SdioBlockReader::new(MockTransport { initialized: false });
        assert_eq!(reader.block_count(), Err(StorageError::NotReady));

        reader.initialize().expect("mock SDIO initializes");

        assert_eq!(reader.block_count(), Ok(MOCK_BLOCK_COUNT));
    }

    #[test]
    fn delegates_bounded_reads_to_the_transport() {
        let mut reader = SdioBlockReader::new(MockTransport { initialized: false });
        let mut block = [0; BLOCK_SIZE];

        reader.initialize().expect("mock SDIO initializes");
        reader
            .read_block(BlockAddress::new(READ_BLOCK), &mut block)
            .expect("mock block read succeeds");

        assert_eq!(block, [READ_BLOCK as u8; BLOCK_SIZE]);
        assert_eq!(
            reader.read_block(BlockAddress::new(MOCK_BLOCK_COUNT), &mut block),
            Err(StorageError::InvalidBlockAddress)
        );
    }
}
