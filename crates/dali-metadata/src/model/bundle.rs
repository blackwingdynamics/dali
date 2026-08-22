use crate::{BoundedText, MAX_TARGET_PROFILE_BYTES, MetadataHeader, Sha256Digest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleFileKind {
    Root,
    Timestamp,
    Snapshot,
    Targets,
    Delegation,
    Revocation,
    Package,
}
impl BundleFileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Timestamp => "timestamp",
            Self::Snapshot => "snapshot",
            Self::Targets => "targets",
            Self::Delegation => "delegation",
            Self::Revocation => "revocation",
            Self::Package => "package",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleFile {
    pub kind: BundleFileKind,
    pub id: BoundedText<{ crate::MAX_BUNDLE_ID_BYTES }>,
    pub length: u32,
    pub sha256: Sha256Digest,
}
impl Default for BundleFile {
    fn default() -> Self {
        Self {
            kind: BundleFileKind::Root,
            id: BoundedText::default(),
            length: 0,
            sha256: Sha256Digest::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleMetadata {
    pub header: MetadataHeader,
    pub target_profile: BoundedText<MAX_TARGET_PROFILE_BYTES>,
    pub files: [BundleFile; crate::MAX_BUNDLE_FILES],
    pub file_count: u16,
}
