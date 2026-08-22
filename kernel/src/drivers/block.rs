//! Hardware-neutral block transport contracts.

/// The fixed sector size used by block storage transports.
pub const BLOCK_SIZE: usize = 512;

/// A single fixed-size storage block.
pub type Block = [u8; BLOCK_SIZE];

/// A zero-based address of a storage block.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BlockAddress(u32);

impl BlockAddress {
    /// Creates a block address from a zero-based card sector number.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the zero-based card sector number.
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Errors that can occur at the block transport boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageError {
    /// The storage transport has not completed initialization.
    NotReady,
    /// The requested block is outside the detected media capacity.
    InvalidBlockAddress,
    /// The card did not complete an operation before the bounded deadline.
    Timeout,
    /// The card or transport reported a data-integrity failure.
    DataCorruption,
    /// The selected board transport cannot provide the requested operation.
    Unsupported,
    /// The underlying hardware transport reported an unspecified failure.
    Transport,
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
        })
    }
}

impl core::error::Error for StorageError {}

/// Reads fixed-size blocks from a storage medium.
pub trait BlockReader {
    /// Reads one complete block into the caller-provided fixed-size buffer.
    ///
    /// The implementation must not partially expose a block as successful. On
    /// error, the buffer contents are unspecified and must not be parsed.
    fn read_block(&mut self, address: BlockAddress, buffer: &mut Block)
    -> Result<(), StorageError>;

    /// Returns the number of addressable blocks after initialization.
    fn block_count(&self) -> Result<u32, StorageError>;
}

/// Flushes completed writes at a block-transport boundary.
pub trait FlushableBlockDevice {
    /// Transport-specific failure type.
    type Error;

    /// Waits until previously accepted writes are ready on durable media.
    fn flush(&self) -> Result<(), Self::Error>;
}

/// Mutable flush operation exposed by a concrete block transport.
#[cfg(feature = "storage-write")]
pub trait BlockTransportFlush {
    /// Transport-specific failure type.
    type Error;

    /// Waits until previously accepted writes are ready on durable media.
    fn flush(&mut self) -> Result<(), Self::Error>;
}

/// Writes complete fixed-size blocks to a storage medium.
#[cfg(feature = "storage-write")]
pub trait BlockWriter {
    /// Writes one complete block to the selected sector.
    fn write_block(&mut self, address: BlockAddress, block: &Block) -> Result<(), StorageError>;
}
