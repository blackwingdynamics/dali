//! Board-owned single-slot Flash artifact replacement.

use dali_kernel_api::{
    installation::SingleSlotReplacement,
    storage::{ArtifactReader, StorageError},
};
use stm32f4xx_hal::{
    flash::{FlashExt, LockedFlash},
    pac,
};

use crate::artifact::{FlashArtifactReader, PUBLICATION_MAGIC, PUBLICATION_MARKER_SIZE};

/// Errors returned while replacing the F405 Flash artifact.
#[derive(Debug)]
pub enum FlashArtifactWriteError<E> {
    /// The target memory profile does not provide a valid artifact region.
    Storage(StorageError),
    /// The replacement operation is not valid in its current lifecycle state.
    Replacement(dali_kernel_api::installation::SingleSlotReplacementError),
    /// The HAL rejected an erase or program operation.
    Flash(stm32f4xx_hal::flash::Error),
    /// The caller's AMRN validation rejected the written candidate.
    Validation(E),
}

/// Board-owned streaming writer for the F405 single physical artifact slot.
pub struct FlashArtifactWriter {
    flash: LockedFlash,
    replacement: SingleSlotReplacement,
    capacity: u32,
    region: dali_targets::TargetMemoryRegion,
    candidate_length: Option<u32>,
}

impl FlashArtifactWriter {
    /// Takes exclusive ownership of the board Flash peripheral.
    pub fn new(flash: pac::FLASH) -> Result<Self, StorageError> {
        let region = dali_targets::TARGET_F405
            .memory
            .artifact
            .ok_or(StorageError::Unsupported)?;
        let capacity = dali_targets::TARGET_F405
            .memory
            .artifact_capacity
            .ok_or(StorageError::Unsupported)?;
        if capacity == 0 || capacity > region.length {
            return Err(StorageError::InvalidRange);
        }
        Ok(Self {
            flash: LockedFlash::new(flash),
            replacement: SingleSlotReplacement::new(capacity),
            capacity,
            region,
            candidate_length: None,
        })
    }

    /// Erases the single physical artifact sector and starts a bounded write.
    pub fn begin(
        &mut self,
        length: u32,
    ) -> Result<(), FlashArtifactWriteError<core::convert::Infallible>> {
        self.candidate_length = None;
        self.replacement
            .begin(length)
            .map_err(FlashArtifactWriteError::Replacement)?;
        let region_offset = self.region_offset()?;
        let sector = self
            .flash
            .sector(region_offset)
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        if sector.offset != region_offset || sector.size != self.region.length as usize {
            return Err(FlashArtifactWriteError::Storage(StorageError::InvalidRange));
        }
        let mut flash = self.flash.unlocked();
        if let Err(error) = flash.erase(sector.number) {
            self.replacement.recover_empty();
            return Err(FlashArtifactWriteError::Flash(error));
        }
        self.candidate_length = Some(length);
        Ok(())
    }

    /// Programs one contiguous candidate chunk without buffering the artifact.
    pub fn write(
        &mut self,
        offset: u32,
        bytes: &[u8],
    ) -> Result<(), FlashArtifactWriteError<core::convert::Infallible>> {
        self.replacement
            .record_write(
                offset,
                u32::try_from(bytes.len())
                    .map_err(|_| FlashArtifactWriteError::Storage(StorageError::InvalidRange))?,
            )
            .map_err(FlashArtifactWriteError::Replacement)?;
        let absolute = self
            .region
            .origin
            .checked_add(offset)
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        let offset = usize::try_from(absolute)
            .map_err(|_| FlashArtifactWriteError::Storage(StorageError::InvalidRange))?
            .checked_sub(self.flash.address())
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        let mut flash = self.flash.unlocked();
        if let Err(error) = flash.program(offset, bytes.iter()) {
            self.replacement.recover_empty();
            self.candidate_length = None;
            return Err(FlashArtifactWriteError::Flash(error));
        }
        Ok(())
    }

    /// Publishes the candidate after external AMRN validation succeeds.
    pub fn publish(&mut self) -> Result<(), FlashArtifactWriteError<core::convert::Infallible>> {
        self.replacement
            .mark_validated()
            .map_err(FlashArtifactWriteError::Replacement)?;
        let artifact_length = self
            .replacement
            .validated_length()
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        let marker_offset = self
            .region
            .origin
            .checked_add(self.capacity)
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        let marker_offset = usize::try_from(marker_offset)
            .map_err(|_| FlashArtifactWriteError::Storage(StorageError::InvalidRange))?
            .checked_sub(self.flash.address())
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        let marker = publication_marker(artifact_length);
        let mut flash = self.flash.unlocked();
        flash
            .program(marker_offset, marker.iter())
            .map_err(FlashArtifactWriteError::Flash)?;
        match FlashArtifactReader::new().artifact_present() {
            Ok(true) => {}
            Ok(false) => {
                self.replacement.recover_empty();
                self.candidate_length = None;
                return Err(FlashArtifactWriteError::Storage(
                    StorageError::DataCorruption,
                ));
            }
            Err(error) => {
                self.replacement.recover_empty();
                self.candidate_length = None;
                return Err(FlashArtifactWriteError::Storage(error));
            }
        }
        self.replacement
            .publish()
            .map_err(FlashArtifactWriteError::Replacement)
    }

    /// Validates the complete candidate and publishes its marker.
    pub fn validate_and_publish<F, E>(
        &mut self,
        validate: F,
    ) -> Result<(), FlashArtifactWriteError<E>>
    where
        F: FnOnce(&mut FlashArtifactReader) -> Result<(), E>,
    {
        let length = self
            .candidate_length
            .ok_or(FlashArtifactWriteError::Storage(StorageError::NotReady))?;
        let mut reader =
            FlashArtifactReader::candidate(length).map_err(FlashArtifactWriteError::Storage)?;
        validate(&mut reader).map_err(|error| {
            self.replacement.recover_empty();
            self.candidate_length = None;
            FlashArtifactWriteError::Validation(error)
        })?;
        self.publish().map_err(map_publish_error)
    }

    /// Invalidates the candidate without publishing a marker.
    pub fn abort(&mut self) {
        self.replacement.recover_empty();
        self.candidate_length = None;
    }

    fn region_offset(&self) -> Result<usize, FlashArtifactWriteError<core::convert::Infallible>> {
        let origin = usize::try_from(self.region.origin)
            .map_err(|_| FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        origin
            .checked_sub(self.flash.address())
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))
    }
}

impl dali_kernel_api::installation::ArtifactInstallationBackend for FlashArtifactWriter {
    type Error = FlashArtifactWriteError<core::convert::Infallible>;

    fn begin(&mut self, length: u32) -> Result<(), Self::Error> {
        Self::begin(self, length)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        Self::write(self, offset, bytes)
    }

    fn publish(&mut self) -> Result<(), Self::Error> {
        Self::publish(self)
    }

    fn abort(&mut self) {
        Self::abort(self);
    }
}

fn publication_marker(length: u32) -> [u8; PUBLICATION_MARKER_SIZE as usize] {
    let mut marker = [0xFF; PUBLICATION_MARKER_SIZE as usize];
    marker[..PUBLICATION_MAGIC.len()].copy_from_slice(&PUBLICATION_MAGIC);
    marker[8..12].copy_from_slice(&length.to_le_bytes());
    marker[12..16].copy_from_slice(&(!length).to_le_bytes());
    marker
}

fn map_publish_error<E>(
    error: FlashArtifactWriteError<core::convert::Infallible>,
) -> FlashArtifactWriteError<E> {
    match error {
        FlashArtifactWriteError::Storage(error) => FlashArtifactWriteError::Storage(error),
        FlashArtifactWriteError::Replacement(error) => FlashArtifactWriteError::Replacement(error),
        FlashArtifactWriteError::Flash(error) => FlashArtifactWriteError::Flash(error),
        FlashArtifactWriteError::Validation(error) => match error {},
    }
}

#[cfg(test)]
mod tests {
    use super::{PUBLICATION_MAGIC, PUBLICATION_MARKER_SIZE, publication_marker};

    #[test]
    fn publication_marker_contains_length_and_inverse() {
        let marker = publication_marker(0x1234_5678);
        assert_eq!(&marker[..PUBLICATION_MAGIC.len()], &PUBLICATION_MAGIC);
        assert_eq!(&marker[8..12], &0x1234_5678_u32.to_le_bytes());
        assert_eq!(&marker[12..16], &(!0x1234_5678_u32).to_le_bytes());
        assert_eq!(marker.len(), PUBLICATION_MARKER_SIZE as usize);
    }
}
