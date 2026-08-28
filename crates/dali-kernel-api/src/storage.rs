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
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError>;
}

/// Hardware-facing SDIO transport implemented by a board backend.
pub trait SdioTransport {
    /// Initializes the medium and returns its block count.
    fn initialize(&mut self) -> Result<u32, StorageError>;

    /// Reads one complete block from the initialized medium.
    fn read_block(&mut self, address: BlockAddress, block: &mut Block) -> Result<(), StorageError>;

    /// Writes one complete block to the initialized medium.
    #[cfg(feature = "storage-write")]
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError>;

    /// Flushes previously accepted writes to durable media.
    #[cfg(feature = "storage-write")]
    fn flush(&mut self) -> Result<(), StorageError>;
}

/// Lifecycle controls exposed by a reinitializable board transport.
pub trait StorageLifecycleControl {
    /// Initializes or reinitializes the storage medium.
    fn initialize(&mut self) -> Result<(), StorageError>;

    /// Reinitializes the medium after removal or transport failure.
    fn reinitialize(&mut self) -> Result<(), StorageError>;
}
