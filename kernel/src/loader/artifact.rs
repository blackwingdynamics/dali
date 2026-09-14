//! Sequential cursor over a board-owned read-only artifact source.

use dali_kernel_api::storage::{ArtifactReader, StorageError};

use super::{CartridgeReader, LoaderError};

/// Streams one manifest-owned artifact through the signed cartridge pipeline.
pub(crate) struct FlashCartridgeReader<R> {
    /// Board-owned bounded artifact source.
    reader: R,
    /// Immutable artifact capacity.
    length: u32,
    /// Current sequential read offset.
    offset: u32,
}

impl<R> FlashCartridgeReader<R>
where
    R: ArtifactReader,
{
    /// Creates a cursor after reading the immutable artifact capacity.
    pub(crate) fn new(reader: R) -> Result<Self, LoaderError> {
        let length = reader.artifact_length().map_err(storage_error_to_loader)?;
        Ok(Self {
            reader,
            length,
            offset: 0,
        })
    }
}

impl<R> CartridgeReader for FlashCartridgeReader<R>
where
    R: ArtifactReader,
{
    fn length(&self) -> u32 {
        self.length
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LoaderError> {
        let requested = u32::try_from(buffer.len()).map_err(|_| invalid_range())?;
        let remaining = self.length.saturating_sub(self.offset);
        let count = requested.min(remaining);
        if count == 0 {
            return Ok(0);
        }
        let count = usize::try_from(count).map_err(|_| invalid_range())?;
        self.reader
            .read_artifact(self.offset, &mut buffer[..count])
            .map_err(storage_error_to_loader)?;
        self.offset = self
            .offset
            .checked_add(u32::try_from(count).map_err(|_| invalid_range())?)
            .ok_or_else(invalid_range)?;
        Ok(count)
    }

    fn rewind(&mut self) -> Result<(), LoaderError> {
        self.offset = 0;
        Ok(())
    }
}

/// Creates the stable loader error for an invalid cursor range.
fn invalid_range() -> LoaderError {
    storage_error_to_loader(StorageError::InvalidRange)
}

/// Maps a hardware-neutral artifact failure to the loader error boundary.
fn storage_error_to_loader(error: StorageError) -> LoaderError {
    LoaderError::Filesystem(embedded_sdmmc::Error::DeviceError(error))
}
