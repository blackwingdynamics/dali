//! Read-only memory-mapped artifact storage for the STM32F405 target.

use dali_kernel_api::storage::{ArtifactReader, StorageError};

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

    fn absolute_range(offset: u32, length: usize) -> Result<(usize, usize), StorageError> {
        let region = Self::region()?;
        let length = u32::try_from(length).map_err(|_| StorageError::InvalidRange)?;
        let end = offset
            .checked_add(length)
            .ok_or(StorageError::InvalidRange)?;
        if end > region.length {
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
}

impl ArtifactReader for FlashArtifactReader {
    fn artifact_length(&self) -> Result<u32, StorageError> {
        Ok(Self::region()?.length)
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
            Ok(dali_targets::TARGET_F405.memory.artifact.unwrap().length)
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
}
