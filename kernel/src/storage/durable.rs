//! Board-agnostic durable trust-store storage contracts.

use crate::drivers::{Block, BlockAddress};

pub mod coordinator;
pub mod journal;

pub use journal::{JournalError, RecoveryDecision, recover as recover_commit_journal};

/// Hardware-neutral fixed-block boundary below filesystem adapters.
///
/// Board support packages implement this boundary. The trait deliberately
/// carries no SDIO, FAT, MCU, pin, or controller-specific types.
pub trait BlockDevice {
    /// Transport-specific failure type owned by the adapter.
    type Error;

    /// Reads complete blocks into caller-owned storage.
    fn read_blocks(&mut self, start: BlockAddress, blocks: &mut [Block])
    -> Result<(), Self::Error>;

    /// Writes complete blocks from caller-owned storage.
    fn write_blocks(&mut self, start: BlockAddress, blocks: &[Block]) -> Result<(), Self::Error>;

    /// Makes previously accepted writes durable.
    fn flush(&mut self) -> Result<(), Self::Error>;

    /// Returns the number of addressable blocks.
    fn block_count(&self) -> Result<u32, Self::Error>;
}

/// One logical durable artifact owned by the kernel installer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurableArtifact {
    /// First trust-store payload slot.
    SlotA,
    /// Second trust-store payload slot.
    SlotB,
    /// Commit journal selecting the durable active generation.
    CommitJournal,
}

/// Commit journal state encoded in the bounded journal record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CommitJournalState {
    /// Candidate data is durable but not yet selected as active.
    Prepared = 1,
    /// The referenced slot is the active trust-store generation.
    Committed = 2,
}

/// Size of one DALI-CMT.BIN journal record.
pub const COMMIT_JOURNAL_RECORD_BYTES: usize = 104;
/// Size reserved for two journal records used for torn-write recovery.
pub const COMMIT_JOURNAL_BYTES: usize = COMMIT_JOURNAL_RECORD_BYTES * 2;

/// Logical storage adapter consumed by the trust-store persistence coordinator.
pub trait DurableStorageAdapter {
    /// Adapter-specific storage failure type.
    type Error;

    /// Reads one logical artifact into a bounded caller-owned buffer.
    fn read_artifact(
        &mut self,
        artifact: DurableArtifact,
        output: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Replaces one logical artifact with the supplied bounded bytes.
    fn write_artifact(
        &mut self,
        artifact: DurableArtifact,
        contents: &[u8],
    ) -> Result<(), Self::Error>;

    /// Flushes the artifact and its metadata before the next state transition.
    fn flush(&mut self) -> Result<(), Self::Error>;
}
