//! Kernel loader facade for AMRN cartridges and repository applications.

use crate::{drivers::StorageError, storage::filesystem::AmrnFile};

mod cartridge;
mod pipeline;
#[cfg(all(feature = "abi-current", feature = "repository-loader"))]
mod repository_boot;

#[cfg(feature = "repository-loader")]
pub mod repository;

#[cfg(all(feature = "abi-current", not(feature = "repository-loader")))]
pub(crate) use cartridge::load_current_abi;
#[cfg(not(feature = "abi-current"))]
pub use cartridge::start_application;
pub use cartridge::{load_amrn_file, validate_amrn_file};
#[cfg(all(feature = "abi-current", feature = "repository-loader"))]
pub(crate) use repository_boot::load_repository_cartridge;

/// Errors reported while validating or loading a cartridge.
#[derive(Debug)]
pub enum LoaderError {
    /// The read-only filesystem could not provide the cartridge stream.
    Filesystem(embedded_sdmmc::Error<StorageError>),
    /// The cartridge header or payload failed AMRN validation.
    Cartridge(dali_amrn::ParseError),
    /// The ABI v3 cartridge failed target or segment validation.
    #[cfg(feature = "abi-current")]
    CurrentAbiCartridge(dali_amrn::v2::Error),
    /// The selected target does not declare an ABI v3 memory contract.
    #[cfg(feature = "abi-current")]
    UnsupportedCurrentAbiTarget,
    /// The kernel could not reserve the cartridge's manifest-declared slot.
    #[cfg(feature = "abi-current")]
    SlotManager(crate::runtime::memory::slots::SlotManagerError),
    /// The application lifecycle could not record the loaded slot ownership.
    #[cfg(feature = "abi-current")]
    Lifecycle(crate::runtime::application::lifecycle::LifecycleError),
    /// The cartridge failed the identity and slot catalog contract.
    #[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
    CartridgeCatalog(crate::loader_contract::CatalogError),
    /// The relocatable ABI v3 cartridge failed format validation or patching.
    #[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
    V3RelocationCartridge(dali_amrn::v3::Error),
    /// The identity-aware ABI v3 cartridge failed format validation or patching.
    #[cfg(all(feature = "abi-relocation", not(feature = "repository-loader")))]
    V4IdentityCartridge(dali_amrn::v4::Error),
    /// The signed v5 cartridge failed format validation or authentication.
    #[cfg(feature = "abi-authentication")]
    V5SignedCartridge(dali_amrn::v5::Error),
    /// The signed cartridge selected no provisioned target trust anchor.
    #[cfg(feature = "abi-authentication")]
    UnknownTrustAnchor,
    /// The signed cartridge failed cryptographic verification.
    #[cfg(feature = "abi-authentication")]
    SignatureVerification(dali_crypto::VerificationError),
    /// The cartridge requested a service not exposed by the current kernel.
    #[cfg(feature = "abi-relocation")]
    UnsupportedServices(u32),
}

/// Sequential reader consumed by format-specific AMRN pipelines.
#[cfg(feature = "abi-authentication")]
pub(crate) trait CartridgeReader {
    /// Returns the immutable cartridge length.
    fn length(&self) -> u32;

    /// Reads the next bounded portion of the cartridge.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LoaderError>;

    /// Rewinds the reader to the cartridge beginning.
    fn rewind(&mut self) -> Result<(), LoaderError>;
}

#[cfg(feature = "abi-authentication")]
impl<D> CartridgeReader for AmrnFile<'_, D>
where
    D: embedded_sdmmc::BlockDevice<Error = StorageError>,
{
    fn length(&self) -> u32 {
        AmrnFile::length(self)
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LoaderError> {
        AmrnFile::read(self, buffer).map_err(LoaderError::Filesystem)
    }

    fn rewind(&mut self) -> Result<(), LoaderError> {
        AmrnFile::rewind(self).map_err(LoaderError::Filesystem)
    }
}

/// Reads exactly one complete buffer from a sequential cartridge reader.
#[cfg(feature = "abi-authentication")]
pub(crate) fn read_cartridge_exact<R>(reader: &mut R, buffer: &mut [u8]) -> Result<(), LoaderError>
where
    R: CartridgeReader,
{
    let mut offset = 0;
    while offset < buffer.len() {
        let read = reader.read(&mut buffer[offset..])?;
        if read == 0 {
            return Err(LoaderError::Filesystem(embedded_sdmmc::Error::EndOfFile));
        }
        offset += read;
    }
    Ok(())
}

/// Performs the `read_exact` operation for this subsystem.
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
