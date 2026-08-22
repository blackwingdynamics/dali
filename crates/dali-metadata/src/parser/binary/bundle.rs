//! Binary Metadata v2 offline bundle body codec.

use super::{bundle_kind_from_number, bundle_kind_number, header, read_header, read_text, text};
use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    BundleFile, BundleMetadata, DecodeError, EncodeError, MetadataRole, Sha256Digest,
    validate_bundle_metadata,
};

/// Encodes the canonical Binary Metadata v2 bundle body.
pub fn encode_binary_bundle_body(
    metadata: BundleMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_bundle_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    text(&mut writer, metadata.target_profile)?;
    writer.u16(metadata.file_count)?;
    for file in metadata.files.iter().take(usize::from(metadata.file_count)) {
        writer.u8(bundle_kind_number(file.kind))?;
        text(&mut writer, file.id)?;
        writer.u32(file.length)?;
        writer.bytes(&file.sha256.0)?;
    }
    Ok(writer.position())
}

/// Parses the canonical Binary Metadata v2 bundle body.
pub fn parse_binary_bundle_body(bytes: &[u8]) -> Result<BundleMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Bundle)?;
    let target_profile = read_text(&mut reader)?;
    let file_count = reader.u16()?;
    if usize::from(file_count) > crate::MAX_BUNDLE_FILES {
        return Err(DecodeError::TooManyRecords);
    }
    let mut files = [BundleFile::default(); crate::MAX_BUNDLE_FILES];
    for file in files.iter_mut().take(usize::from(file_count)) {
        file.kind = bundle_kind_from_number(reader.u8()?).ok_or(DecodeError::InvalidValue)?;
        file.id = read_text(&mut reader)?;
        file.length = reader.u32()?;
        file.sha256 = Sha256Digest(reader.array()?);
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = BundleMetadata {
        header,
        target_profile,
        files,
        file_count,
    };
    validate_bundle_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}
