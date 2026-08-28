use core::mem::{ManuallyDrop, MaybeUninit};
use dali_metadata::{
    BinaryDelegationBodyStreamParser, BinaryEnvelopeStreamParser, BinaryRevocationBodyStreamParser,
    BinaryBundleBodyStreamParser, BinaryRoleBodyParser, BinaryRootBodyStreamParser,
    BinarySnapshotBodyStreamParser,
    BinaryTimestampBodyStreamParser, DecodeError, MetadataRole, RoleDefinition, RoleKey,
    RootMetadata, Sha256Digest, StreamedEnvelope, StreamingRoleVerifier,
};

use super::{RepositoryLoadRequest, amrn, streaming};
use crate::storage::repository::{
    RepositoryDocument, RepositoryCartridgeDigest, RepositoryStreamStorage,
};

/// Maximum number of repository cartridges handed to the bounded execution pipeline.
pub const MAX_BINARY_REPOSITORY_CARTRIDGES: usize = 4;
/// Maximum revocation records retained by the bounded kernel chain pass.
pub const MAX_BINARY_REVOCATION_RECORDS: usize = 8;
/// Maximum delegation namespaces retained for one executable cartridge.
pub const MAX_BINARY_CARTRIDGE_DELEGATION_NAMESPACES: usize = 1;
/// Maximum delegation target profiles retained for one executable cartridge.
pub const MAX_BINARY_CARTRIDGE_DELEGATION_TARGETS: usize = 1;
/// Maximum delegation ABI versions retained for one executable cartridge.
pub const MAX_BINARY_CARTRIDGE_DELEGATION_ABIS: usize = 1;

/// Authentication result retained for one streamed metadata document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StreamedRoleInfo {
    /// Envelope authentication metadata collected during the first pass.
    pub envelope: StreamedEnvelope,
    /// Exact serialized document length reported by storage.
    pub length: u32,
    /// SHA-256 digest of the complete serialized document.
    pub digest: Sha256Digest,
}

/// Errors returned by bounded metadata streaming.
#[derive(Debug)]
pub(crate) enum StreamedRoleError<E> {
    /// The repository adapter failed while producing a chunk.
    Storage(E),
    /// The envelope or typed body was malformed.
    Decode,
    /// The storage-reported length did not match delivered bytes.
    LengthMismatch,
    /// The role signature threshold was not met.
    Signature,
    /// No target-provisioned anchor was declared by Root metadata.
    UnknownTrustAnchor,
}

/// Caller-owned buffers for one Binary v2 repository chain pass.
pub struct BinaryRepositoryBuffers {
    /// Shared bounded metadata and cartridge transport chunk.
    chunk: [u8; streaming::STREAMING_METADATA_CHUNK_BYTES],
    /// Fixed AMRN header and signature trailer retained between passes.
    amrn: amrn::AmrnStreamBuffers,
    /// Root policy retained after target-provisioned anchor validation.
    pub(crate) root: MaybeUninit<RootMetadata>,
    /// Snapshot/delegation metadata shared by sequential chain phases.
    pub(crate) metadata: RepositoryMetadataScratch,
    /// Revocation metadata retained for cartridge authorization checks.
    pub(crate) revocations: MaybeUninit<dali_metadata::RevocationMetadata>,
    /// Delegation references selected from the authenticated Snapshot.
    pub(crate) delegation_references:
        [Option<dali_metadata::DelegationReference>; MAX_BINARY_REPOSITORY_CARTRIDGES],
    /// Mutually exclusive target-verifier and delegation scratch storage.
    pub(crate) scratch: RepositoryScratch,
}

/// Scratch storage for the repository role verifier.
pub(crate) union RepositoryScratch {
    /// Ed25519 state used during Targets replay.
    pub(crate) target_verifier: ManuallyDrop<streaming::TargetVerifierWorkspace>,
    /// Root parser state retained outside the loader stack frame.
    pub(crate) root_parser: ManuallyDrop<MaybeUninit<BinaryRootBodyStreamParser>>,
}

/// Metadata storage shared after Snapshot delegation references are copied.
pub(crate) union RepositoryMetadataScratch {
    /// Snapshot retained through target and revocation reference checks.
    pub(crate) snapshot: ManuallyDrop<MaybeUninit<dali_metadata::SnapshotMetadata>>,
    /// Bundle summary retained during generation admission.
    pub(crate) bundle: ManuallyDrop<MaybeUninit<BinaryBundleBodyStreamParser>>,
    /// Delegation metadata retained while its cartridge is authenticated.
    pub(crate) delegation: ManuallyDrop<MaybeUninit<dali_metadata::DelegationMetadata>>,
}

impl RepositoryScratch {
/// Internal helper for `root_parser`.
    fn root_parser(&mut self) -> &mut BinaryRootBodyStreamParser {
        // SAFETY: the loader activates one parser variant at a time and all
        // variants are ManuallyDrop because the union storage is reused.
        unsafe { self.root_parser.assume_init_mut() }
    }

/// Internal helper for `target_verifier`.
    unsafe fn target_verifier(&mut self) -> &mut MaybeUninit<StreamingRoleVerifier> {
        // SAFETY: the target verifier variant is active for role replay.
        unsafe { &mut *core::ptr::addr_of_mut!(self.target_verifier) }
    }
}

impl RepositoryMetadataScratch {
/// Internal helper for `snapshot`.
    unsafe fn snapshot(&mut self) -> &mut MaybeUninit<dali_metadata::SnapshotMetadata> {
        // SAFETY: the snapshot variant is active during timestamp/snapshot
        // verification and reference checks.
        unsafe { &mut *core::ptr::addr_of_mut!(self.snapshot) }
    }

/// Internal helper for `delegation`.
    unsafe fn delegation(&mut self) -> &mut MaybeUninit<dali_metadata::DelegationMetadata> {
        // SAFETY: the delegation variant is active during one cartridge pass.
        unsafe { &mut *core::ptr::addr_of_mut!(self.delegation) }
    }

/// Internal helper for `bundle_parser`.
    unsafe fn bundle_parser(&mut self) -> &mut BinaryBundleBodyStreamParser {
        // SAFETY: the bundle parser variant is active during bundle verification.
        unsafe { self.bundle.assume_init_mut() }
    }
}

/// Internal helper for `reset_parser`.
fn reset_parser<P>(slot: &mut P, parser: P) -> &mut P {
    // SAFETY: parser slots are ManuallyDrop union storage and are initialized
    // before each sequential role pass.
    unsafe { core::ptr::write(slot, parser) };
    slot
}

impl BinaryRepositoryBuffers {
    /// Creates zeroed storage for the streaming chain.
    pub const fn new() -> Self {
        Self {
            chunk: [0; streaming::STREAMING_METADATA_CHUNK_BYTES],
            amrn: amrn::AmrnStreamBuffers::new(),
            root: MaybeUninit::uninit(),
            metadata: RepositoryMetadataScratch {
                snapshot: ManuallyDrop::new(MaybeUninit::uninit()),
            },
            revocations: MaybeUninit::uninit(),
            delegation_references: [None; MAX_BINARY_REPOSITORY_CARTRIDGES],
            scratch: RepositoryScratch {
                root_parser: ManuallyDrop::new(MaybeUninit::uninit()),
            },
        }
    }
}

impl Default for BinaryRepositoryBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a complete metadata-to-AMRN streamed authorization pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryRepositoryAuthorization {
    /// Target cartridge record accepted by Targets metadata.
    pub target: dali_metadata::TargetCartridge,
    /// Developer key authorized by the validated delegation metadata.
    pub developer_public_key: [u8; dali_metadata::PUBLIC_KEY_LENGTH],
}

/// Bounded authorization set for one repository verification pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryRepositoryAuthorizations {
/// Internal field `entries`.
    entries: [Option<BinaryRepositoryAuthorization>; MAX_BINARY_REPOSITORY_CARTRIDGES],
/// Internal field `length`.
    length: usize,
    /// Durable generation selected by boot recovery.
    pub committed_generation: Option<dali_metadata::TrustStoreRecord>,
    /// Security state reconstructed from the verified Root and Revocation roles.
    pub security_state: Option<dali_metadata::TrustStoreSecurityState>,
    /// Signed bundle generation admitted for this load.
    pub bundle_generation: Option<u64>,
}

impl BinaryRepositoryAuthorizations {
    /// Creates an empty authorization set.
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_BINARY_REPOSITORY_CARTRIDGES],
            length: 0,
            committed_generation: None,
            security_state: None,
            bundle_generation: None,
        }
    }

    /// Returns the number of authorized cartridges.
    pub const fn len(&self) -> usize {
        self.length
    }

    /// Returns whether no cartridge was authorized.
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Iterates over authorized cartridges in Targets document order.
    pub fn iter(&self) -> impl Iterator<Item = BinaryRepositoryAuthorization> + '_ {
        self.entries[..self.length]
            .iter()
            .filter_map(Option::as_ref)
            .copied()
    }

/// Internal helper for `push`.
    fn push(&mut self, authorization: BinaryRepositoryAuthorization) -> Result<(), ()> {
        let Some(entry) = self.entries.get_mut(self.length) else {
            return Err(());
        };
        *entry = Some(authorization);
        self.length += 1;
        Ok(())
    }
}

impl Default for BinaryRepositoryAuthorizations {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors returned by the complete streamed repository chain.
#[derive(Debug)]
pub enum BinaryRepositoryError<E> {
    /// Storage failed while producing one document or cartridge.
    Storage(E),
    /// A role document failed bounded parsing or signature verification.
    RoleDecode,
    /// A role stream could not satisfy its signature policy.
    RoleSignature,
    /// A role stream failed in the storage adapter.
    RoleStorage(E),
    /// Bundle-manifest streaming failed in the storage adapter.
    BundleRoleStorage(E),
    /// Bundle-manifest parsing failed.
    BundleRoleDecode,
    /// Bundle-manifest signature policy failed.
    BundleRoleSignature,
    /// Bundle manifest targets another profile.
    BundleTargetMismatch,
    /// Bundle generation is not admissible for the requested load context.
    BundleGenerationRollback,
    /// Active boot found a bundle newer than the committed generation.
    BundleGenerationAhead,
    /// The root document did not contain a target-provisioned trust anchor.
    UnknownTrustAnchor,
    /// A required role definition or record was absent.
    MissingRecord,
    /// Targets stream parsing failed after the envelope was read.
    TargetsParse(streaming::StreamingTargetsError),
    /// Snapshot's Targets reference did not match the streamed Targets file.
    TargetsReferenceMismatch,
    /// Timestamp's Snapshot reference did not match the streamed Snapshot file.
    SnapshotReferenceMismatch,
    /// Snapshot's Revocations reference did not match the streamed Revocations file.
    RevocationReferenceMismatch,
    /// Snapshot's Delegation reference did not match the streamed Delegation file.
    DelegationReferenceMismatch,
    /// The selected delegation did not authorize the selected cartridge.
    DelegationMismatch,
    /// The selected developer key was revoked.
    Revoked,
    /// Verified root/revocation state could not be reconstructed.
    SecurityState,
    /// Cartridge storage failed during streamed validation.
    CartridgeStorage(E),
    /// The cartridge length did not match its signed header.
    CartridgeLengthMismatch,
    /// The selected target record could not resolve to a target memory contract.
    CartridgeContract,
    /// The cartridge header or DSIG envelope was malformed.
    CartridgeInvalidHeader(dali_amrn::v5::Error),
    /// The cartridge digest did not match its Targets record.
    CartridgeDigestMismatch,
    /// The cartridge Ed25519 signature did not verify.
    CartridgeSignature,
    /// The cartridge CRC did not verify.
    CartridgeCrc,
}
