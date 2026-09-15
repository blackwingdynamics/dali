//! Read-only memory-mapped artifact storage for the STM32F405 target.

use dali_kernel_api::storage::{ArtifactReader, StorageError};

/// Fixed publication marker size stored after the logical artifact capacity.
pub(crate) const PUBLICATION_MARKER_SIZE: u32 = 16;
pub(crate) const PUBLICATION_MAGIC: [u8; 8] = *b"DALIART1";

/// Read-only view of the manifest-owned F405 artifact flash region.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlashArtifactReader;

impl FlashArtifactReader {
    /// Creates a reader for the target's configured artifact region.
    pub const fn new() -> Self {
        Self
    }

    fn region() -> Result<dali_targets::TargetMemoryRegion, StorageError> {
        dali_targets::TARGET_F405
            .memory
            .artifact
            .ok_or(StorageError::Unsupported)
    }

    fn capacity() -> Result<u32, StorageError> {
        dali_targets::TARGET_F405
            .memory
            .artifact_capacity
            .ok_or(StorageError::Unsupported)
    }

    fn absolute_range(offset: u32, length: usize) -> Result<(usize, usize), StorageError> {
        let region = Self::region()?;
        let capacity = Self::capacity()?;
        let length = u32::try_from(length).map_err(|_| StorageError::InvalidRange)?;
        let end = offset
            .checked_add(length)
            .ok_or(StorageError::InvalidRange)?;
        if end > capacity || capacity > region.length {
            return Err(StorageError::InvalidRange);
        }
        let address = region
            .origin
            .checked_add(offset)
            .ok_or(StorageError::InvalidRange)?;
        let address = usize::try_from(address).map_err(|_| StorageError::InvalidRange)?;
        let end = address
            .checked_add(length as usize)
            .ok_or(StorageError::InvalidRange)?;
        Ok((address, end))
    }

    fn marker_range() -> Result<(usize, usize), StorageError> {
        let region = Self::region()?;
        let capacity = Self::capacity()?;
        let marker_end = capacity
            .checked_add(PUBLICATION_MARKER_SIZE)
            .ok_or(StorageError::InvalidRange)?;
        if marker_end > region.length {
            return Err(StorageError::InvalidRange);
        }
        let start = region
            .origin
            .checked_add(capacity)
            .ok_or(StorageError::InvalidRange)?;
        let start = usize::try_from(start).map_err(|_| StorageError::InvalidRange)?;
        let end = start
            .checked_add(PUBLICATION_MARKER_SIZE as usize)
            .ok_or(StorageError::InvalidRange)?;
        Ok((start, end))
    }

    fn read_marker() -> Result<[u8; PUBLICATION_MARKER_SIZE as usize], StorageError> {
        let (start, end) = Self::marker_range()?;
        // SAFETY: marker_range proves the fixed marker lies within the
        // manifest-owned physical artifact erase unit.
        let source = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
        let mut marker = [0; PUBLICATION_MARKER_SIZE as usize];
        marker.copy_from_slice(source);
        Ok(marker)
    }

    fn marker_length(marker: &[u8; PUBLICATION_MARKER_SIZE as usize]) -> Option<u32> {
        if marker[..PUBLICATION_MAGIC.len()] != PUBLICATION_MAGIC {
            return None;
        }
        let mut length_bytes = [0; core::mem::size_of::<u32>()];
        length_bytes.copy_from_slice(&marker[8..12]);
        let mut inverse_bytes = [0; core::mem::size_of::<u32>()];
        inverse_bytes.copy_from_slice(&marker[12..16]);
        let length = u32::from_le_bytes(length_bytes);
        let inverse = u32::from_le_bytes(inverse_bytes);
        (length > 0 && length ^ inverse == u32::MAX).then_some(length)
    }
}

impl ArtifactReader for FlashArtifactReader {
    fn artifact_present(&mut self) -> Result<bool, StorageError> {
        let marker = Self::read_marker()?;
        Ok(Self::marker_length(&marker)
            .is_some_and(|length| Self::capacity().is_ok_and(|capacity| length <= capacity)))
    }

    fn artifact_length(&self) -> Result<u32, StorageError> {
        Self::capacity()
    }

    fn published_length(&mut self) -> Result<u32, StorageError> {
        let marker = Self::read_marker()?;
        let length = Self::marker_length(&marker).ok_or(StorageError::DataCorruption)?;
        if length > Self::capacity()? {
            return Err(StorageError::InvalidRange);
        }
        Ok(length)
    }

    fn read_artifact(&mut self, offset: u32, buffer: &mut [u8]) -> Result<(), StorageError> {
        let (start, end) = Self::absolute_range(offset, buffer.len())?;
        // SAFETY: the target manifest bounds the region, and absolute_range
        // proves the complete caller buffer lies inside that region.
        let source = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
        buffer.copy_from_slice(source);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_manifest_artifact_capacity() {
        assert_eq!(
            FlashArtifactReader::new().artifact_length(),
            Ok(dali_targets::TARGET_F405.memory.artifact_capacity.unwrap())
        );
    }

    #[test]
    fn rejects_ranges_outside_manifest_artifact_capacity() {
        let length = FlashArtifactReader::new().artifact_length().unwrap();
        let mut byte = [0u8; 1];
        assert_eq!(
            FlashArtifactReader::new().read_artifact(length, &mut byte),
            Err(StorageError::InvalidRange)
        );
    }

    #[test]
    fn validates_publication_marker_length() {
        let mut marker = [0xFF; PUBLICATION_MARKER_SIZE as usize];
        marker[..PUBLICATION_MAGIC.len()].copy_from_slice(&PUBLICATION_MAGIC);
        marker[8..12].copy_from_slice(&128_u32.to_le_bytes());
        marker[12..16].copy_from_slice(&(!128_u32).to_le_bytes());
        assert_eq!(FlashArtifactReader::marker_length(&marker), Some(128));
        marker[15] ^= 1;
        assert_eq!(FlashArtifactReader::marker_length(&marker), None);
    }
}
