use super::*;
use crate::{
    BundleFile, BundleFileKind, BundleMetadata, MetadataHeader, MetadataRole, Sha256Digest,
};

#[test]
fn parses_a_fragmented_manifest_without_retaining_file_records() {
    let kinds = [
        (BundleFileKind::Root, "root"),
        (BundleFileKind::Timestamp, "timestamp"),
        (BundleFileKind::Snapshot, "snapshot"),
        (BundleFileKind::Targets, "targets"),
        (BundleFileKind::Revocation, "revocation"),
        (BundleFileKind::Delegation, "developer"),
        (BundleFileKind::Cartridge, "cartridge"),
    ];
    let mut files = [BundleFile::default(); crate::MAX_BUNDLE_FILES];
    for (file, (kind, id)) in files.iter_mut().zip(kinds) {
        *file = BundleFile {
            kind,
            id: crate::BoundedText::new(id).expect("test id fits"),
            length: 1,
            sha256: Sha256Digest([7; crate::SHA256_LENGTH]),
        };
    }
    let metadata = BundleMetadata {
        header: MetadataHeader {
            role: MetadataRole::Bundle,
            version: 9,
            expires: 0,
        },
        target_profile: crate::BoundedText::new("f405").expect("profile fits"),
        files,
        file_count: kinds.len() as u16,
    };
    let mut body = [0; 1024];
    let length = crate::encode_binary_bundle_body(metadata, &mut body).expect("body fits");
    let mut parser = BinaryBundleBodyStreamParser::new();
    for chunk in body[..length].chunks(3) {
        parser.feed(chunk).expect("fragment should parse");
    }
    assert_eq!(
        parser.finish(),
        Ok(BundleManifestSummary {
            header: metadata.header,
            target_profile: metadata.target_profile,
            file_count: metadata.file_count,
        })
    );
}
