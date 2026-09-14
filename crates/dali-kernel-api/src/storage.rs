//! Hardware-neutral block storage contracts implemented by board backends.

/// Fixed sector size used by Dali block transports.
pub const BLOCK_SIZE: usize = 512;

/// One fixed-size storage block.
pub type Block = [u8; BLOCK_SIZE];

/// Zero-based address of a storage block.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BlockAddress(u32);

impl BlockAddress {
    /// Creates an address from a zero-based sector number.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the sector number.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Errors returned by a block transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageError {
    /// The transport has not completed initialization.
    NotReady,
    /// The requested block is outside media capacity.
    InvalidBlockAddress,
    /// The requested byte range is outside artifact capacity.
    InvalidRange,
    /// The operation exceeded its bounded deadline.
    Timeout,
    /// The transport reported data corruption.
    DataCorruption,
    /// The backend does not provide the requested operation.
    Unsupported,
    /// The transport reported an unspecified failure.
    Transport,
    /// The medium stopped responding and requires reinitialization.
    CardRemoved,
}

impl core::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::NotReady => "storage is not ready",
            Self::InvalidBlockAddress => "invalid block address",
            Self::InvalidRange => "invalid storage range",
            Self::Timeout => "storage operation timed out",
            Self::DataCorruption => "storage data is corrupted",
            Self::Unsupported => "storage operation is unsupported",
            Self::Transport => "storage transport failure",
            Self::CardRemoved => "storage card was removed",
        })
    }
}

impl core::error::Error for StorageError {}

/// Reads fixed-size blocks from a storage medium.
pub trait BlockReader {
    /// Reads one complete block into caller-owned storage.
    fn read_block(&mut self, address: BlockAddress, buffer: &mut Block)
    -> Result<(), StorageError>;

    /// Returns the number of addressable blocks.
    fn block_count(&self) -> Result<u32, StorageError>;
}

/// Reads a bounded persistent artifact from a selected storage medium.
pub trait ArtifactReader {
    /// Returns the addressable artifact capacity in bytes.
    fn artifact_length(&self) -> Result<u32, StorageError>;

    /// Reads one complete caller-owned byte range from the artifact.
    ///
    /// Implementations must reject an offset or range that exceeds the
    /// declared artifact capacity. They must not return a partial successful
    /// read, so the loader can validate complete AMRN fields deterministically.
    fn read_artifact(&mut self, offset: u32, buffer: &mut [u8]) -> Result<(), StorageError>;
}

/// Flushes previously accepted writes.
pub trait FlushableBlockDevice {
    /// Transport-specific failure type.
    type Error;

    /// Waits until writes are ready on durable media.
    fn flush(&self) -> Result<(), Self::Error>;
}

/// Flushes writes through a mutable board transport.
pub trait BlockTransportFlush {
    /// Transport-specific failure type.
    type Error;

    /// Waits until writes are ready on durable media.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Writes complete fixed-size blocks to a storage medium.
pub trait BlockWriter {
    /// Writes one complete block to the selected sector.
    fn write_block(&mut self, _address: BlockAddress, _block: &Block) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }
}

/// Hardware-facing SDIO transport implemented by a board backend.
pub trait SdioTransport {
    /// Initializes the medium and returns its block count.
    fn initialize(&mut self) -> Result<u32, StorageError>;

    /// Reads one complete block from the initialized medium.
    fn read_block(&mut self, address: BlockAddress, block: &mut Block) -> Result<(), StorageError>;

    /// Writes one complete block to the initialized medium.
    fn write_block(&mut self, _address: BlockAddress, _block: &Block) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }

    /// Flushes previously accepted writes to durable media.
    fn flush(&mut self) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }
}

/// Lifecycle controls exposed by a reinitializable board transport.
pub trait StorageLifecycleControl {
    /// Initializes or reinitializes the storage medium.
    fn initialize(&mut self) -> Result<(), StorageError>;

    /// Reinitializes the medium after removal or transport failure.
    fn reinitialize(&mut self) -> Result<(), StorageError>;
}

/// Observable states for a bounded storage medium lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageLifecycleState {
    /// No storage presence has been established.
    Unavailable,
    /// A storage medium is being probed or has been detected.
    Present,
    /// The medium completed initialization and can serve operations.
    Ready,
    /// The medium stopped responding during an operation.
    Removed,
    /// The medium or transport reported a non-removal failure.
    Fault,
}

/// Lifecycle-aware generic adapter for an SDIO transport.
pub struct SdioBlockReader<T> {
    transport: T,
    block_count: Option<u32>,
    lifecycle: StorageLifecycleState,
}

impl<T> SdioBlockReader<T> {
    /// Creates an uninitialized reader around a board-provided transport.
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            block_count: None,
            lifecycle: StorageLifecycleState::Unavailable,
        }
    }
}

impl<T> SdioBlockReader<T>
where
    T: SdioTransport,
{
    /// Initializes the card and records its bounded capacity.
    pub fn initialize(&mut self) -> Result<(), StorageError> {
        self.lifecycle = StorageLifecycleState::Present;
        self.block_count = None;
        match self.transport.initialize() {
            Ok(block_count) => {
                self.lifecycle = StorageLifecycleState::Ready;
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
        self.lifecycle
    }

    fn record_error(&mut self, error: StorageError) {
        self.lifecycle = if matches!(error, StorageError::CardRemoved) {
            StorageLifecycleState::Removed
        } else {
            StorageLifecycleState::Fault
        };
    }

    fn is_ready(&self) -> bool {
        self.lifecycle == StorageLifecycleState::Ready
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
    fn read_block(&mut self, address: BlockAddress, block: &mut Block) -> Result<(), StorageError> {
        if !self.is_ready() {
            return Err(StorageError::NotReady);
        }
        match self.transport.read_block(address, block) {
            Ok(()) => Ok(()),
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

impl<T> BlockWriter for SdioBlockReader<T>
where
    T: SdioTransport,
{
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError> {
        if !self.is_ready() {
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

impl<T> BlockTransportFlush for SdioBlockReader<T>
where
    T: SdioTransport,
{
    type Error = StorageError;

    fn flush(&mut self) -> Result<(), Self::Error> {
        if !self.is_ready() {
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
