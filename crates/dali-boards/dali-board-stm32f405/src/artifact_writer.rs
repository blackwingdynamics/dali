//! Board-owned single-slot Flash artifact replacement.

use dali_kernel_api::storage::StorageError;
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
    /// The HAL rejected an erase or program operation.
    Flash(stm32f4xx_hal::flash::Error),
    /// The caller's normal AMRN validation rejected the written candidate.
    Validation(E),
}

/// Board-owned writer for the F405 single physical artifact slot.
pub struct FlashArtifactWriter {
    flash: LockedFlash,
}

impl FlashArtifactWriter {
    /// Takes exclusive ownership of the board Flash peripheral.
    pub fn new(flash: pac::FLASH) -> Self {
        Self {
            flash: LockedFlash::new(flash),
        }
    }

    /// Replaces the slot and publishes it only after caller-owned validation.
    ///
    /// The callback must run the normal AMRN loader validation against the
    /// written Flash reader. Erase happens before writing, so interruption can
    /// leave the slot empty and does not preserve the previous cartridge.
    pub fn replace<F, E>(
        &mut self,
        artifact: &[u8],
        validate: F,
    ) -> Result<(), FlashArtifactWriteError<E>>
    where
        F: FnOnce(&mut FlashArtifactReader) -> Result<(), E>,
    {
        let (region, capacity, artifact_length) = artifact_bounds::<E>(artifact.len())?;
        let flash_base = self.flash.address();
        let region_offset = region.origin as usize - flash_base;
        let sector = self
            .flash
            .sector(region_offset)
            .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
        if sector.offset != region_offset || sector.size != region.length as usize {
            return Err(FlashArtifactWriteError::Storage(StorageError::InvalidRange));
        }

        let mut flash = self.flash.unlocked();
        flash
            .erase(sector.number)
            .map_err(FlashArtifactWriteError::Flash)?;
        flash
            .program(region_offset, artifact.iter())
            .map_err(FlashArtifactWriteError::Flash)?;
        drop(flash);

        validate(&mut FlashArtifactReader::new()).map_err(FlashArtifactWriteError::Validation)?;
        self.publish::<E>(region, capacity, artifact_length)
    }

    fn publish<E>(
        &mut self,
        region: dali_targets::TargetMemoryRegion,
        capacity: u32,
        artifact_length: u32,
    ) -> Result<(), FlashArtifactWriteError<E>> {
        let marker_offset = region.origin as usize - self.flash.address() + capacity as usize;
        let marker = publication_marker(artifact_length);
        let mut flash = self.flash.unlocked();
        flash
            .program(marker_offset, marker.iter())
            .map_err(FlashArtifactWriteError::Flash)
    }
}

fn artifact_bounds<E>(
    length: usize,
) -> Result<(dali_targets::TargetMemoryRegion, u32, u32), FlashArtifactWriteError<E>> {
    let region = dali_targets::TARGET_F405
        .memory
        .artifact
        .ok_or(FlashArtifactWriteError::Storage(StorageError::Unsupported))?;
    let capacity = dali_targets::TARGET_F405
        .memory
        .artifact_capacity
        .ok_or(FlashArtifactWriteError::Storage(StorageError::Unsupported))?;
    let length = u32::try_from(length)
        .map_err(|_| FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
    let marker_end = capacity
        .checked_add(PUBLICATION_MARKER_SIZE)
        .ok_or(FlashArtifactWriteError::Storage(StorageError::InvalidRange))?;
    if length == 0 || length > capacity || marker_end > region.length {
        return Err(FlashArtifactWriteError::Storage(StorageError::InvalidRange));
    }
    Ok((region, capacity, length))
}

fn publication_marker(length: u32) -> [u8; PUBLICATION_MARKER_SIZE as usize] {
    let mut marker = [0xFF; PUBLICATION_MARKER_SIZE as usize];
    marker[..PUBLICATION_MAGIC.len()].copy_from_slice(&PUBLICATION_MAGIC);
    marker[8..12].copy_from_slice(&length.to_le_bytes());
    marker[12..16].copy_from_slice(&(!length).to_le_bytes());
    marker
}
