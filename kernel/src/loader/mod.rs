//! Kernel loader facade for AMRN packages and repository applications.

use crate::{drivers::StorageError, storage::filesystem::AmrnFile};

mod package;
mod pipeline;
#[cfg(all(feature = "abi-current", feature = "repository-loader"))]
mod repository_boot;

#[cfg(feature = "repository-loader")]
pub mod repository;

#[cfg(all(feature = "abi-current", not(feature = "repository-loader")))]
pub(crate) use package::load_current_abi;
pub use package::{load_amrn_file, start_application, validate_amrn_file};
#[cfg(all(feature = "abi-current", feature = "repository-loader"))]
pub(crate) use repository_boot::load_repository_package;

/// Errors reported while validating or loading a package.
#[derive(Debug)]
pub enum LoaderError {
    /// The read-only filesystem could not provide the package stream.
    Filesystem(embedded_sdmmc::Error<StorageError>),
    /// The package header or payload failed AMRN validation.
    Package(dali_amrn::ParseError),
    /// The ABI v3 package failed target or segment validation.
    #[cfg(feature = "abi-current")]
    CurrentAbiPackage(dali_amrn::v2::Error),
    /// The selected target does not declare an ABI v3 memory contract.
    #[cfg(feature = "abi-current")]
    UnsupportedCurrentAbiTarget,
    /// The kernel could not reserve the package's manifest-declared slot.
    #[cfg(feature = "abi-current")]
    SlotManager(crate::runtime::memory::slots::SlotManagerError),
    /// The application lifecycle could not record the loaded slot ownership.
    #[cfg(feature = "abi-current")]
    Lifecycle(crate::runtime::application::lifecycle::LifecycleError),
    /// The package failed the identity and slot catalog contract.
    #[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
    PackageCatalog(crate::loader_contract::CatalogError),
    /// The relocatable ABI v3 package failed format validation or patching.
    #[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
    V3RelocationPackage(dali_amrn::v3::Error),
    /// The identity-aware ABI v3 package failed format validation or patching.
    #[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
    V4IdentityPackage(dali_amrn::v4::Error),
    /// The signed v5 package failed format validation or authentication.
    #[cfg(feature = "abi-authentication")]
    V5SignedPackage(dali_amrn::v5::Error),
    /// The signed package selected no provisioned target trust anchor.
    #[cfg(feature = "abi-authentication")]
    UnknownTrustAnchor,
    /// The signed package failed cryptographic verification.
    #[cfg(feature = "abi-authentication")]
    SignatureVerification(dali_crypto::VerificationError),
    /// The package requested a service not exposed by the current kernel.
    #[cfg(feature = "abi-relocation")]
    UnsupportedServices(u32),
}

pub(crate) fn read_exact<D>(
    file: &AmrnFile<'_, D>,
    buffer: &mut [u8],
) -> Result<(), embedded_sdmmc::Error<StorageError>>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    let mut offset = 0;
    while offset < buffer.len() {
        let read = file.read(&mut buffer[offset..])?;
        if read == 0 {
            return Err(embedded_sdmmc::Error::EndOfFile);
        }
        offset += read;
    }
    Ok(())
}
