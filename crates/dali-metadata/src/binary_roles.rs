//! Canonical Binary Metadata v2 role bodies.

use crate::binary::{BodyReader, BodyWriter};
use crate::{
    BoundedText, BundleFile, BundleFileKind, BundleMetadata, DecodeError, DelegationMetadata,
    DelegationReference, EncodeError, KeyId, MetadataHeader, MetadataRole, PublicKey,
    RevocationMetadata, RevocationRecord, RevocationReference, RoleDefinition, RoleKey,
    Sha256Digest, SnapshotMetadata, TargetPackage, TargetsMetadata, TargetsReference,
    validate_bundle_metadata, validate_delegation, validate_revocation_metadata,
    validate_role_references, validate_snapshot_metadata, validate_targets_metadata,
};

pub fn encode_binary_root_body(
    metadata: crate::RootMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    if metadata.header.role != MetadataRole::Root
        || metadata.key_count == 0
        || usize::from(metadata.key_count) > crate::MAX_ROOT_KEYS
        || metadata.role_count == 0
        || usize::from(metadata.role_count) > crate::MAX_ROOT_ROLES
    {
        return Err(EncodeError::InvalidValue);
    }
    validate_role_references(
        &metadata.keys[..usize::from(metadata.key_count)],
        &metadata.roles[..usize::from(metadata.role_count)],
    )
    .map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    writer.u8(metadata.key_count)?;
    for key in metadata.keys.iter().take(usize::from(metadata.key_count)) {
        writer.bytes(&key.key_id.0)?;
        writer.u8(crate::binary::role_number(key.role))?;
        writer.bytes(&key.public_key.0)?;
    }
    writer.u8(metadata.role_count)?;
    for role in metadata.roles.iter().take(usize::from(metadata.role_count)) {
        writer.u8(crate::binary::role_number(role.role))?;
        writer.u8(role.threshold)?;
        writer.u8(role.key_count)?;
        for key in role.keys.iter().take(usize::from(role.key_count)) {
            writer.bytes(&key.0)?;
        }
    }
    Ok(writer.position())
}

pub fn parse_binary_root_body(bytes: &[u8]) -> Result<crate::RootMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Root)?;
    let key_count = reader.u8()?;
    if key_count == 0 || usize::from(key_count) > crate::MAX_ROOT_KEYS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut keys = [RoleKey {
        role: MetadataRole::Root,
        key_id: KeyId([0; crate::KEY_ID_LENGTH]),
        public_key: PublicKey([0; crate::PUBLIC_KEY_LENGTH]),
    }; crate::MAX_ROOT_KEYS];
    for key in keys.iter_mut().take(usize::from(key_count)) {
        key.key_id = KeyId(reader.array()?);
        key.role =
            crate::binary::role_from_number(reader.u8()?).ok_or(DecodeError::InvalidValue)?;
        key.public_key = PublicKey(reader.array()?);
    }
    let role_count = reader.u8()?;
    if role_count == 0 || usize::from(role_count) > crate::MAX_ROOT_ROLES {
        return Err(DecodeError::TooManyRecords);
    }
    let empty_role = RoleDefinition {
        role: MetadataRole::Root,
        keys: [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS],
        key_count: 0,
        threshold: 0,
    };
    let mut roles = [empty_role; crate::MAX_ROOT_ROLES];
    for role in roles.iter_mut().take(usize::from(role_count)) {
        role.role =
            crate::binary::role_from_number(reader.u8()?).ok_or(DecodeError::InvalidValue)?;
        role.threshold = reader.u8()?;
        role.key_count = reader.u8()?;
        if role.key_count == 0 || usize::from(role.key_count) > crate::MAX_ROLE_KEYS {
            return Err(DecodeError::InvalidValue);
        }
        for key in role.keys.iter_mut().take(usize::from(role.key_count)) {
            *key = KeyId(reader.array()?);
        }
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    validate_role_references(
        &keys[..usize::from(key_count)],
        &roles[..usize::from(role_count)],
    )
    .map_err(|_| DecodeError::InvalidValue)?;
    Ok(crate::RootMetadata {
        header,
        keys,
        key_count,
        roles,
        role_count,
    })
}

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

pub fn encode_binary_delegation_body(
    metadata: DelegationMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_delegation(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    text(&mut writer, metadata.developer_id)?;
    writer.bytes(&metadata.key_id.0)?;
    writer.bytes(&metadata.public_key.0)?;
    writer.u8(metadata.namespace_count)?;
    for item in metadata
        .allowed_namespaces
        .iter()
        .take(usize::from(metadata.namespace_count))
    {
        text(&mut writer, *item)?;
    }
    writer.u8(metadata.target_count)?;
    for item in metadata
        .allowed_targets
        .iter()
        .take(usize::from(metadata.target_count))
    {
        text(&mut writer, *item)?;
    }
    writer.u8(metadata.abi_count)?;
    for item in metadata
        .allowed_abis
        .iter()
        .take(usize::from(metadata.abi_count))
    {
        writer.u16(*item)?;
    }
    writer.u64(metadata.not_before)?;
    writer.u64(metadata.not_after)?;
    Ok(writer.position())
}

pub fn parse_binary_delegation_body(bytes: &[u8]) -> Result<DelegationMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Delegation)?;
    let developer_id = read_text(&mut reader)?;
    let key_id = KeyId(reader.array()?);
    let public_key = PublicKey(reader.array()?);
    let namespace_count = reader.u8()?;
    if usize::from(namespace_count) > crate::MAX_DELEGATION_SCOPES {
        return Err(DecodeError::TooManyRecords);
    }
    let mut allowed_namespaces = [BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
    for item in allowed_namespaces
        .iter_mut()
        .take(usize::from(namespace_count))
    {
        *item = read_text(&mut reader)?;
    }
    let target_count = reader.u8()?;
    if usize::from(target_count) > crate::MAX_DELEGATION_TARGETS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut allowed_targets = [BoundedText::default(); crate::MAX_DELEGATION_TARGETS];
    for item in allowed_targets.iter_mut().take(usize::from(target_count)) {
        *item = read_text(&mut reader)?;
    }
    let abi_count = reader.u8()?;
    if usize::from(abi_count) > crate::MAX_DELEGATION_ABIS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut allowed_abis = [0; crate::MAX_DELEGATION_ABIS];
    for item in allowed_abis.iter_mut().take(usize::from(abi_count)) {
        *item = reader.u16()?;
    }
    let metadata = DelegationMetadata {
        header,
        developer_id,
        key_id,
        public_key,
        allowed_namespaces,
        namespace_count,
        allowed_targets,
        target_count,
        allowed_abis,
        abi_count,
        not_before: reader.u64()?,
        not_after: reader.u64()?,
    };
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    validate_delegation(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

pub fn encode_binary_revocation_body(
    metadata: RevocationMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_revocation_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    writer.u16(u16::from(metadata.record_count))?;
    for record in metadata
        .records
        .iter()
        .take(usize::from(metadata.record_count))
    {
        text(&mut writer, record.developer_id)?;
        writer.u64(record.effective_version)?;
        writer.bytes(&record.issuer_key_id.0)?;
        writer.bytes(&record.key_id.0)?;
        text(&mut writer, record.reason)?;
    }
    Ok(writer.position())
}

pub fn parse_binary_revocation_body(bytes: &[u8]) -> Result<RevocationMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Revocation)?;
    let record_count = reader.u16()?;
    if usize::from(record_count) > crate::MAX_REVOCATIONS {
        return Err(DecodeError::TooManyRecords);
    }
    let mut records = [RevocationRecord::default(); crate::MAX_REVOCATIONS];
    for record in records.iter_mut().take(usize::from(record_count)) {
        record.developer_id = read_text(&mut reader)?;
        record.effective_version = reader.u64()?;
        record.issuer_key_id = KeyId(reader.array()?);
        record.key_id = KeyId(reader.array()?);
        record.reason = read_text(&mut reader)?;
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = RevocationMetadata {
        header,
        records,
        record_count: u8::try_from(record_count).map_err(|_| DecodeError::TooManyRecords)?,
    };
    validate_revocation_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

pub fn encode_binary_targets_body(
    metadata: TargetsMetadata,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    validate_targets_metadata(&metadata).map_err(|_| EncodeError::InvalidValue)?;
    let mut writer = BodyWriter::new(output);
    header(&mut writer, metadata.header)?;
    writer.u16(u16::from(metadata.delegation_count))?;
    for item in metadata
        .delegations
        .iter()
        .take(usize::from(metadata.delegation_count))
    {
        text(&mut writer, *item)?;
    }
    writer.u32(u32::from(metadata.package_count))?;
    for package in metadata
        .packages
        .iter()
        .take(usize::from(metadata.package_count))
    {
        let length_position = writer.position();
        writer.u16(0)?;
        let record_start = writer.position();
        encode_target_record(&mut writer, *package)?;
        let record_length = writer.position() - record_start;
        let record_length = u16::try_from(record_length).map_err(|_| EncodeError::InvalidValue)?;
        writer.patch_u16(length_position, record_length)?;
    }
    Ok(writer.position())
}

pub fn parse_binary_targets_body(bytes: &[u8]) -> Result<TargetsMetadata, DecodeError> {
    let mut reader = BodyReader::new(bytes);
    let header = read_header(&mut reader, MetadataRole::Targets)?;
    let delegation_count = reader.u16()?;
    if usize::from(delegation_count) > crate::MAX_DELEGATION_SCOPES {
        return Err(DecodeError::TooManyRecords);
    }
    let mut delegations = [BoundedText::default(); crate::MAX_DELEGATION_SCOPES];
    for item in delegations.iter_mut().take(usize::from(delegation_count)) {
        *item = read_text(&mut reader)?;
    }
    let package_count = reader.u32()?;
    if package_count > crate::MAX_TARGET_RECORDS as u32 {
        return Err(DecodeError::TooManyRecords);
    }
    let mut packages = [TargetPackage::default(); crate::MAX_TARGET_RECORDS];
    for package in packages.iter_mut().take(package_count as usize) {
        let record_length = usize::from(reader.u16()?);
        let record = reader.array_slice(record_length)?;
        let mut record_reader = BodyReader::new(record);
        *package = parse_target_record(&mut record_reader)?;
        if !record_reader.complete() {
            return Err(DecodeError::TrailingBytes);
        }
    }
    if !reader.complete() {
        return Err(DecodeError::TrailingBytes);
    }
    let metadata = TargetsMetadata {
        header,
        delegations,
        delegation_count: u8::try_from(delegation_count)
            .map_err(|_| DecodeError::TooManyRecords)?,
        packages,
        package_count: u16::try_from(package_count).map_err(|_| DecodeError::TooManyRecords)?,
    };
    validate_targets_metadata(&metadata).map_err(|_| DecodeError::InvalidValue)?;
    Ok(metadata)
}

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

fn encode_target_record(
    writer: &mut BodyWriter<'_>,
    value: TargetPackage,
) -> Result<(), EncodeError> {
    writer.bytes(&value.package_id.0)?;
    text(writer, value.namespace)?;
    text(writer, value.developer_id)?;
    text(writer, value.delegation_id)?;
    writer.bytes(&value.developer_key_id.0)?;
    text(writer, value.target_profile)?;
    writer.u16(value.amrn_format)?;
    writer.u16(value.abi_version)?;
    text(writer, value.package_version)?;
    text(writer, value.minimum_kernel_version)?;
    writer.u32(value.length)?;
    writer.bytes(&value.sha256.0)?;
    writer.u32(value.required_services)?;
    writer.u8(value.slot_id)
}

fn parse_target_record(reader: &mut BodyReader<'_>) -> Result<TargetPackage, DecodeError> {
    Ok(TargetPackage {
        package_id: crate::PackageId(reader.array()?),
        namespace: read_text(reader)?,
        developer_id: read_text(reader)?,
        delegation_id: read_text(reader)?,
        developer_key_id: KeyId(reader.array()?),
        target_profile: read_text(reader)?,
        amrn_format: reader.u16()?,
        abi_version: reader.u16()?,
        package_version: read_text(reader)?,
        minimum_kernel_version: read_text(reader)?,
        length: reader.u32()?,
        sha256: Sha256Digest(reader.array()?),
        required_services: reader.u32()?,
        slot_id: reader.u8()?,
    })
}

const fn bundle_kind_number(value: BundleFileKind) -> u8 {
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

const fn bundle_kind_from_number(value: u8) -> Option<BundleFileKind> {
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

fn header(writer: &mut BodyWriter<'_>, value: MetadataHeader) -> Result<(), EncodeError> {
    writer.u64(value.version)?;
    writer.u64(value.expires)
}

fn read_header(
    reader: &mut BodyReader<'_>,
    role: MetadataRole,
) -> Result<MetadataHeader, DecodeError> {
    Ok(MetadataHeader {
        role,
        version: reader.u64()?,
        expires: reader.u64()?,
    })
}

fn text<const N: usize>(
    writer: &mut BodyWriter<'_>,
    value: BoundedText<N>,
) -> Result<(), EncodeError> {
    let value = value.as_str().ok_or(EncodeError::InvalidValue)?;
    let length = u16::try_from(value.len()).map_err(|_| EncodeError::InvalidValue)?;
    writer.u16(length)?;
    writer.bytes(value.as_bytes())
}

fn read_text<const N: usize>(reader: &mut BodyReader<'_>) -> Result<BoundedText<N>, DecodeError> {
    let length = usize::from(reader.u16()?);
    let bytes = reader.array_slice(length)?;
    let value = core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidString)?;
    BoundedText::new(value).map_err(|_| DecodeError::InvalidValue)
}

trait MetadataReference {
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

fn reference<T: MetadataReference>(
    writer: &mut BodyWriter<'_>,
    value: T,
) -> Result<(), EncodeError> {
    writer.u64(value.version())?;
    writer.u32(value.length())?;
    writer.bytes(&value.sha256().0)
}

fn read_reference(reader: &mut BodyReader<'_>) -> Result<TargetsReference, DecodeError> {
    Ok(TargetsReference {
        version: reader.u64()?,
        length: reader.u32()?,
        sha256: Sha256Digest(reader.array()?),
    })
}

fn read_revocation_reference(
    reader: &mut BodyReader<'_>,
) -> Result<RevocationReference, DecodeError> {
    Ok(RevocationReference {
        version: reader.u64()?,
        length: reader.u32()?,
        sha256: Sha256Digest(reader.array()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(role: MetadataRole) -> MetadataHeader {
        MetadataHeader {
            role,
            version: 1,
            expires: 0,
        }
    }

    fn text<const N: usize>(value: &str) -> BoundedText<N> {
        BoundedText::new(value).expect("test text fits")
    }

    fn digest() -> Sha256Digest {
        Sha256Digest([7; crate::SHA256_LENGTH])
    }

    #[test]
    fn round_trips_binary_role_bodies() {
        let key = KeyId([1; crate::KEY_ID_LENGTH]);
        let empty_key = RoleKey {
            role: MetadataRole::Root,
            key_id: KeyId([0; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([0; crate::PUBLIC_KEY_LENGTH]),
        };
        let mut root_keys = [empty_key; crate::MAX_ROOT_KEYS];
        root_keys[0] = RoleKey {
            role: MetadataRole::Root,
            key_id: key,
            public_key: PublicKey([2; crate::PUBLIC_KEY_LENGTH]),
        };
        let empty_role = RoleDefinition {
            role: MetadataRole::Root,
            keys: [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS],
            key_count: 0,
            threshold: 0,
        };
        let mut roles = [empty_role; crate::MAX_ROOT_ROLES];
        let mut role_keys = [KeyId([0; crate::KEY_ID_LENGTH]); crate::MAX_ROLE_KEYS];
        role_keys[0] = key;
        roles[0] = RoleDefinition {
            role: MetadataRole::Targets,
            keys: role_keys,
            key_count: 1,
            threshold: 1,
        };
        let root = crate::RootMetadata {
            header: header(MetadataRole::Root),
            keys: root_keys,
            key_count: 1,
            roles,
            role_count: 1,
        };
        let mut buffer = [0; crate::MAX_ROOT_BYTES];
        let length = encode_binary_root_body(root, &mut buffer).expect("root encodes");
        let parsed = parse_binary_root_body(&buffer[..length]).expect("root parses");
        assert_eq!(parsed.header, root.header);
        assert_eq!(parsed.keys[..1], root.keys[..1]);
        assert_eq!(parsed.roles[..1], root.roles[..1]);

        let reference = TargetsReference {
            version: 1,
            length: 64,
            sha256: digest(),
        };
        let snapshot = SnapshotMetadata {
            header: header(MetadataRole::Snapshot),
            targets: reference,
            revocations: RevocationReference {
                version: 1,
                length: 64,
                sha256: digest(),
            },
            delegations: [DelegationReference {
                id: text("developer"),
                version: 1,
                length: 64,
                sha256: digest(),
            }; crate::MAX_SNAPSHOT_REFERENCES],
            delegation_count: 1,
        };
        let mut buffer = [0; crate::MAX_SNAPSHOT_BYTES];
        let length = encode_binary_snapshot_body(snapshot, &mut buffer).expect("snapshot encodes");
        let parsed = parse_binary_snapshot_body(&buffer[..length]).expect("snapshot parses");
        assert_eq!(parsed.header, snapshot.header);
        assert_eq!(parsed.targets, snapshot.targets);
        assert_eq!(parsed.revocations, snapshot.revocations);
        assert_eq!(parsed.delegations[..1], snapshot.delegations[..1]);

        let delegation = DelegationMetadata {
            header: header(MetadataRole::Delegation),
            developer_id: text("developer"),
            key_id: KeyId([2; crate::KEY_ID_LENGTH]),
            public_key: PublicKey([3; crate::PUBLIC_KEY_LENGTH]),
            allowed_namespaces: [text("developer/app"); crate::MAX_DELEGATION_SCOPES],
            namespace_count: 1,
            allowed_targets: [text("f405"); crate::MAX_DELEGATION_TARGETS],
            target_count: 1,
            allowed_abis: {
                let mut values = [0; crate::MAX_DELEGATION_ABIS];
                values[0] = 3;
                values
            },
            abi_count: 1,
            not_before: 0,
            not_after: 0,
        };
        let mut buffer = [0; crate::MAX_DELEGATION_BYTES];
        let length =
            encode_binary_delegation_body(delegation, &mut buffer).expect("delegation encodes");
        let parsed = parse_binary_delegation_body(&buffer[..length]).expect("delegation parses");
        assert_eq!(parsed.header, delegation.header);
        assert_eq!(parsed.developer_id, delegation.developer_id);
        assert_eq!(parsed.key_id, delegation.key_id);
        assert_eq!(parsed.public_key, delegation.public_key);
        assert_eq!(
            parsed.allowed_namespaces[..1],
            delegation.allowed_namespaces[..1]
        );
        assert_eq!(parsed.allowed_targets[..1], delegation.allowed_targets[..1]);
        assert_eq!(parsed.allowed_abis[..1], delegation.allowed_abis[..1]);
    }

    #[test]
    fn target_record_length_rejects_truncation() {
        let mut buffer = [0; crate::MAX_TARGETS_BYTES];
        let metadata = TargetsMetadata {
            header: header(MetadataRole::Targets),
            delegations: [text("developer"); crate::MAX_DELEGATION_SCOPES],
            delegation_count: 1,
            packages: [TargetPackage {
                package_id: crate::PackageId([1; crate::KEY_ID_LENGTH]),
                namespace: text("developer/app"),
                developer_id: text("developer"),
                delegation_id: text("developer"),
                developer_key_id: KeyId([2; crate::KEY_ID_LENGTH]),
                target_profile: text("f405"),
                amrn_format: 5,
                abi_version: 3,
                package_version: text("0.1.0"),
                minimum_kernel_version: text("0.1.0"),
                length: 64,
                sha256: digest(),
                required_services: 1,
                slot_id: 0,
            }; crate::MAX_TARGET_RECORDS],
            package_count: 1,
        };
        let length = encode_binary_targets_body(metadata, &mut buffer).expect("targets encode");
        assert!(parse_binary_targets_body(&buffer[..length - 1]).is_err());
    }
}
