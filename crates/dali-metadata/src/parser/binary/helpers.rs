use crate::codec::binary::{BodyReader, BodyWriter};
use crate::{
    BoundedText, BundleFileKind, DecodeError, EncodeError, MetadataHeader, MetadataRole,
    RevocationReference, Sha256Digest, TargetsReference,
};

pub(crate) const fn bundle_kind_number(value: BundleFileKind) -> u8 {
    match value {
        BundleFileKind::Root => 1,
        BundleFileKind::Timestamp => 2,
        BundleFileKind::Snapshot => 3,
        BundleFileKind::Targets => 4,
        BundleFileKind::Delegation => 5,
        BundleFileKind::Revocation => 6,
        BundleFileKind::Package => 7,
    }
}

pub(crate) const fn bundle_kind_from_number(value: u8) -> Option<BundleFileKind> {
    match value {
        1 => Some(BundleFileKind::Root),
        2 => Some(BundleFileKind::Timestamp),
        3 => Some(BundleFileKind::Snapshot),
        4 => Some(BundleFileKind::Targets),
        5 => Some(BundleFileKind::Delegation),
        6 => Some(BundleFileKind::Revocation),
        7 => Some(BundleFileKind::Package),
        _ => None,
    }
}

pub(crate) fn header(
    writer: &mut BodyWriter<'_>,
    value: MetadataHeader,
) -> Result<(), EncodeError> {
    writer.u64(value.version)?;
    writer.u64(value.expires)
}

pub(crate) fn read_header(
    reader: &mut BodyReader<'_>,
    role: MetadataRole,
) -> Result<MetadataHeader, DecodeError> {
    Ok(MetadataHeader {
        role,
        version: reader.u64()?,
        expires: reader.u64()?,
    })
}

pub(crate) fn text<const N: usize>(
    writer: &mut BodyWriter<'_>,
    value: BoundedText<N>,
) -> Result<(), EncodeError> {
    let value = value.as_str().ok_or(EncodeError::InvalidValue)?;
    let length = u16::try_from(value.len()).map_err(|_| EncodeError::InvalidValue)?;
    writer.u16(length)?;
    writer.bytes(value.as_bytes())
}

pub(crate) fn read_text<const N: usize>(
    reader: &mut BodyReader<'_>,
) -> Result<BoundedText<N>, DecodeError> {
    let length = usize::from(reader.u16()?);
    let bytes = reader.array_slice(length)?;
    let value = core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidString)?;
    BoundedText::new(value).map_err(|_| DecodeError::InvalidValue)
}

pub(crate) trait MetadataReference {
    fn version(&self) -> u64;
    fn length(&self) -> u32;
    fn sha256(&self) -> Sha256Digest;
}

impl MetadataReference for TargetsReference {
    fn version(&self) -> u64 {
        self.version
    }

    fn length(&self) -> u32 {
        self.length
    }

    fn sha256(&self) -> Sha256Digest {
        self.sha256
    }
}

impl MetadataReference for RevocationReference {
    fn version(&self) -> u64 {
        self.version
    }

    fn length(&self) -> u32 {
        self.length
    }

    fn sha256(&self) -> Sha256Digest {
        self.sha256
    }
}

pub(crate) fn reference<T: MetadataReference>(
    writer: &mut BodyWriter<'_>,
    value: T,
) -> Result<(), EncodeError> {
    writer.u64(value.version())?;
    writer.u32(value.length())?;
    writer.bytes(&value.sha256().0)
}

pub(crate) fn read_reference(reader: &mut BodyReader<'_>) -> Result<TargetsReference, DecodeError> {
    Ok(TargetsReference {
        version: reader.u64()?,
        length: reader.u32()?,
        sha256: Sha256Digest(reader.array()?),
    })
}

pub(crate) fn read_revocation_reference(
    reader: &mut BodyReader<'_>,
) -> Result<RevocationReference, DecodeError> {
    Ok(RevocationReference {
        version: reader.u64()?,
        length: reader.u32()?,
        sha256: Sha256Digest(reader.array()?),
    })
}
