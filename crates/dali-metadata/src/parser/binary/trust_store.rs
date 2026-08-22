//! Binary Metadata v2 durable trust-store payload codec.

use super::{bundle_kind_from_number, bundle_kind_number, read_header, read_text, text};
use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    BundleFile, DecodeError, EncodeError, MetadataRole, Sha256Digest, TrustStorePayload,
    validate_trust_store_payload,
};

/// Encodes the package-free Binary Metadata v2 recovery body.
pub fn encode_binary_trust_store_body(
    metadata: TrustStorePayload,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_trust_store_payload(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    super::header(&mut writer, metadata.header)?;
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

/// Parses the package-free Binary Metadata v2 recovery body.
pub fn parse_binary_trust_store_body(bytes: &[u8]) -> Result<TrustStorePayload, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Recovery)?;
    let target_profile = read_text(&mut reader)?;
    let file_count = reader.u16()?;
    if usize::from(file_count) > crate::MAX_TRUST_STORE_FILES {
        return Err(DecodeError::TooManyRecords);
    }
    let mut files = [BundleFile::default(); crate::MAX_TRUST_STORE_FILES];
    for file in files.iter_mut().take(usize::from(file_count)) {
        file.kind = bundle_kind_from_number(reader.u8()?).ok_or(DecodeError::InvalidValue)?;
        file.id = read_text(&mut reader)?;
        file.length = reader.u32()?;
        file.sha256 = Sha256Digest(reader.array()?);
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = TrustStorePayload {
        header,
        target_profile,
        files,
        file_count,
    };
    validate_trust_store_payload(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundedText, BundleFileKind, MetadataHeader};

    fn text<const N: usize>(value: &str) -> BoundedText<N> {
        BoundedText::new(value).expect("test text fits")
    }

    fn digest() -> Sha256Digest {
        Sha256Digest([7; crate::SHA256_LENGTH])
    }

    fn header() -> MetadataHeader {
        MetadataHeader {
            role: MetadataRole::Recovery,
            version: 1,
            expires: 0,
        }
    }

    fn payload() -> TrustStorePayload {
        let kinds = [
            (BundleFileKind::Root, "root"),
            (BundleFileKind::Timestamp, "timestamp"),
            (BundleFileKind::Snapshot, "snapshot"),
            (BundleFileKind::Targets, "targets"),
            (BundleFileKind::Revocation, "revocation"),
            (BundleFileKind::Delegation, "developer-one"),
        ];
        let mut files = [BundleFile::default(); crate::MAX_TRUST_STORE_FILES];
        for (file, (kind, id)) in files.iter_mut().zip(kinds) {
            *file = BundleFile {
                kind,
                id: text(id),
                length: 64,
                sha256: digest(),
            };
        }
        TrustStorePayload {
            header: header(),
            target_profile: text("f405"),
            files,
            file_count: kinds.len() as u16,
        }
    }

    #[test]
    fn round_trips_without_package_references() {
        let payload = payload();
        let mut buffer = [0; crate::MAX_TRUST_STORE_BYTES];
        let length = encode_binary_trust_store_body(payload, &mut buffer)
            .expect("trust store payload encodes");
        let parsed =
            parse_binary_trust_store_body(&buffer[..length]).expect("trust store payload parses");
        assert_eq!(parsed, payload);
    }

    #[test]
    fn rejects_package_reference() {
        let mut payload = payload();
        payload.files[5].kind = BundleFileKind::Package;
        let mut buffer = [0; crate::MAX_TRUST_STORE_BYTES];
        assert_eq!(
            encode_binary_trust_store_body(payload, &mut buffer),
            Err(EncodeError::InvalidValue)
        );
    }
}
