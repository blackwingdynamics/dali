use super::{SdioTransport, StorageLifecycleControl};

const MOCK_BLOCK_COUNT: u32 = 4;
const READ_BLOCK: u32 = 2;

struct MockTransport {
    initialized: bool,
    init_failures: u8,
    read_error: Option<crate::drivers::StorageError>,
}

impl SdioTransport for MockTransport {
    fn initialize(&mut self) -> Result<u32, crate::drivers::StorageError> {
        if self.init_failures != 0 {
            self.init_failures -= 1;
            self.initialized = false;
            return Err(crate::drivers::StorageError::CardRemoved);
        }
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
        if let Some(error) = self.read_error {
            return Err(error);
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

    #[cfg(feature = "storage-write")]
    fn flush(&mut self) -> Result<(), crate::drivers::StorageError> {
        if self.initialized {
            Ok(())
        } else {
            Err(crate::drivers::StorageError::NotReady)
        }
    }
}

#[test]
fn exposes_capacity_only_after_initialization() {
    let mut reader = super::SdioBlockReader::new(MockTransport {
        initialized: false,
        init_failures: 0,
        read_error: None,
    });
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
    let mut reader = super::SdioBlockReader::new(MockTransport {
        initialized: false,
        init_failures: 0,
        read_error: None,
    });
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

#[test]
fn removal_moves_reader_to_recovery_and_reinitialization_restores_ready() {
    let mut reader = super::SdioBlockReader::new(MockTransport {
        initialized: false,
        init_failures: 1,
        read_error: None,
    });

    assert_eq!(
        reader.initialize(),
        Err(crate::drivers::StorageError::CardRemoved)
    );
    assert_eq!(
        reader.lifecycle_state(),
        crate::drivers::lifecycle::StorageLifecycleState::Removed
    );
    reader
        .reinitialize()
        .expect("reinitialization succeeds after reinsertion");
    assert_eq!(
        reader.lifecycle_state(),
        crate::drivers::lifecycle::StorageLifecycleState::Ready
    );
}

#[test]
fn read_removal_is_recorded_before_recovery() {
    let mut reader = super::SdioBlockReader::new(MockTransport {
        initialized: false,
        init_failures: 0,
        read_error: Some(crate::drivers::StorageError::CardRemoved),
    });
    let mut block = [0; crate::drivers::BLOCK_SIZE];

    reader.initialize().expect("mock SDIO initializes");
    assert_eq!(
        <super::SdioBlockReader<MockTransport> as crate::drivers::BlockReader>::read_block(
            &mut reader,
            crate::drivers::BlockAddress::new(READ_BLOCK),
            &mut block,
        ),
        Err(crate::drivers::StorageError::CardRemoved)
    );
    assert_eq!(
        reader.lifecycle_state(),
        crate::drivers::lifecycle::StorageLifecycleState::Removed
    );
}
