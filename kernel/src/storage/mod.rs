//! Transport-independent block-storage boundary.

/// The fixed sector size used by SD cards and the MVP filesystem layer.
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

/// Errors that can occur at the storage boundary.
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

/// Reads fixed-size blocks from a storage medium.
pub trait BlockReader {
    /// Reads one complete block into the caller-provided fixed-size buffer.
    ///
    /// The implementation must not partially expose a block as successful. On
    /// error, the buffer contents are unspecified and must not be parsed.
    fn read_block(&mut self, address: BlockAddress, buffer: &mut Block)
    -> Result<(), StorageError>;
}
