//! Binary Metadata v2 snapshot role body codec.

use super::{
    header, read_header, read_reference, read_revocation_reference, read_text, reference, text,
};
use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    DecodeError, DelegationReference, EncodeError, MetadataRole, Sha256Digest, SnapshotMetadata,
    validate_snapshot_metadata,
};

/// Encodes the canonical Binary Metadata v2 snapshot body.
pub fn encode_binary_snapshot_body(
    metadata: SnapshotMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_snapshot_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    reference(&mut writer, metadata.targets)?;
    reference(&mut writer, metadata.revocations)?;
    writer.u8(metadata.delegation_count)?;
    for item in metadata
        .delegations
        .iter()
        .take(usize::from(metadata.delegation_count))
    {
        text(&mut writer, item.id)?;
        writer.u64(item.version)?;
        writer.u32(item.length)?;
        writer.bytes(&item.sha256.0)?;
    }
    Ok(writer.position())
}

/// Parses the canonical Binary Metadata v2 snapshot body.
pub fn parse_binary_snapshot_body(bytes: &[u8]) -> Result<SnapshotMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Snapshot)?;
    let targets = read_reference(&mut reader)?;
    let revocations = read_revocation_reference(&mut reader)?;
    let delegation_count = reader.u8()?;
    if usize::from(delegation_count) > crate::MAX_SNAPSHOT_REFERENCES {
        return Err(DecodeError::TooManyRecords);
    }
    let mut delegations = [DelegationReference::default(); crate::MAX_SNAPSHOT_REFERENCES];
    for item in delegations.iter_mut().take(usize::from(delegation_count)) {
        item.id = read_text(&mut reader)?;
        item.version = reader.u64()?;
        item.length = reader.u32()?;
        item.sha256 = Sha256Digest(reader.array()?);
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = SnapshotMetadata {
        header,
        targets,
        revocations,
        delegations,
        delegation_count,
    };
    validate_snapshot_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}
