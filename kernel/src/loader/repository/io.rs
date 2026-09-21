//! Bounded bridges from streaming storage into legacy loader buffers.

use super::{RepositoryLoaderError, streaming::STREAMING_METADATA_CHUNK_BYTES};
use crate::storage::repository::{
    RepositoryCartridgeDigest, RepositoryDocument, RepositoryStreamStorage,
};

/// Internal helper for `read_metadata`.
pub(super) fn read_metadata<S>(
    storage: &mut S,
    document: RepositoryDocument<'_>,
    output: &mut [u8],
    chunk: &mut [u8; STREAMING_METADATA_CHUNK_BYTES],
) -> Result<usize, RepositoryLoaderError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    if output.is_empty() {
        return Err(RepositoryLoaderError::ArtifactTooLarge);
    }
    let mut offset: usize = 0;
    let mut failure = None;
    let streamed = storage
        .stream_metadata(document, chunk, |part| {
            if failure.is_some() {
                return Ok(());
            }
            let end = match offset.checked_add(part.len()) {
                Some(end) => end,
                None => {
                    failure = Some(RepositoryLoaderError::ArtifactTooLarge);
                    return Ok(());
                }
            };
            if end > output.len() {
                failure = Some(RepositoryLoaderError::ArtifactTooLarge);
                return Ok(());
            }
            output[offset..end].copy_from_slice(part);
            offset = end;
            Ok(())
        })
        .map_err(RepositoryLoaderError::Storage)?;
    validate_stream_length(streamed, offset, output.len())?;
    failure.map_or(Ok(offset), Err)
}

/// Internal helper for `read_cartridge`.
pub(super) fn read_cartridge<S>(
    storage: &mut S,
    digest: RepositoryCartridgeDigest,
    output: &mut [u8],
    chunk: &mut [u8; STREAMING_METADATA_CHUNK_BYTES],
) -> Result<usize, RepositoryLoaderError<S::Error>>
where
    S: RepositoryStreamStorage,
{
    if output.is_empty() {
        return Err(RepositoryLoaderError::ArtifactTooLarge);
    }
    let mut offset: usize = 0;
    let mut failure = None;
    let streamed = storage
        .stream_cartridge(digest, chunk, |part| {
            if failure.is_some() {
                return Ok(());
            }
            let end = match offset.checked_add(part.len()) {
                Some(end) => end,
                None => {
                    failure = Some(RepositoryLoaderError::ArtifactTooLarge);
                    return Ok(());
                }
            };
            if end > output.len() {
                failure = Some(RepositoryLoaderError::ArtifactTooLarge);
                return Ok(());
            }
            output[offset..end].copy_from_slice(part);
            offset = end;
            Ok(())
        })
        .map_err(RepositoryLoaderError::Storage)?;
    validate_stream_length(streamed, offset, output.len())?;
    failure.map_or(Ok(offset), Err)
}

/// Internal helper for `validate_stream_length`.
fn validate_stream_length<E>(
    streamed: u32,
    delivered: usize,
    capacity: usize,
) -> Result<(), RepositoryLoaderError<E>> {
    let streamed =
        usize::try_from(streamed).map_err(|_| RepositoryLoaderError::ArtifactTooLarge)?;
    if streamed > capacity {
        return Err(RepositoryLoaderError::ArtifactTooLarge);
    }
    if streamed != delivered {
        return Err(RepositoryLoaderError::StorageLengthMismatch);
    }
    Ok(())
}
