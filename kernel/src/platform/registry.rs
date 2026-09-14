//! Target metadata registration and capability validation.

#[cfg(any(
    feature = "abi-current",
    feature = "abi-mpu",
    feature = "abi-context-switch"
))]
use dali_kernel_api::BoardInfo;
use dali_kernel_api::{BoardBackend, BoardError};

#[cfg(any(
    feature = "abi-current",
    feature = "abi-mpu",
    feature = "abi-context-switch"
))]
/// Backend metadata registered during kernel bootstrap.
static BOARD_INFO: critical_section::Mutex<core::cell::RefCell<Option<BoardInfo>>> =
    critical_section::Mutex::new(core::cell::RefCell::new(None));

/// Registers backend metadata and validates enabled capabilities.
pub(super) fn register<B: BoardBackend>() -> Result<(), BoardError> {
    #[cfg(any(
        feature = "sdio",
        feature = "usb-cdc",
        feature = "abi-mpu",
        feature = "abi-relocation"
    ))]
    validate_feature_capabilities(B::info().capabilities)?;

    #[cfg(any(
        feature = "abi-current",
        feature = "abi-mpu",
        feature = "abi-context-switch"
    ))]
    critical_section::with(|cs| {
        let mut registered = BOARD_INFO.borrow(cs).borrow_mut();
        if registered.is_none() {
            *registered = Some(B::info());
        }
    });
    Ok(())
}

/// Returns metadata registered by the selected firmware composition.
#[cfg(any(
    feature = "abi-current",
    feature = "abi-mpu",
    feature = "abi-context-switch"
))]
pub(crate) fn board_info() -> Option<BoardInfo> {
    critical_section::with(|cs| *BOARD_INFO.borrow(cs).borrow())
}

/// Returns the target profile registered by the selected firmware composition.
#[cfg(any(
    feature = "abi-authentication",
    feature = "repository-loader",
    feature = "abi-context-switch"
))]
pub(crate) fn target_profile() -> Option<&'static dali_targets::TargetProfile> {
    board_info().map(|info| info.target)
}

/// Returns the memory profile registered by the selected firmware composition.
#[cfg(any(feature = "abi-current", feature = "abi-mpu"))]
pub(crate) fn memory_profile() -> Option<dali_targets::MemoryProfile> {
    board_info().map(|info| info.memory)
}

/// Returns scheduler metadata registered by the selected firmware composition.
#[cfg(feature = "abi-context-switch")]
pub(crate) fn scheduler_profile() -> Option<dali_targets::SchedulerProfile> {
    target_profile().and_then(|target| target.scheduler)
}

/// Returns trust anchors selected by the registered target and build policy.
#[cfg(feature = "abi-authentication")]
pub(crate) fn trust_anchors() -> &'static [dali_targets::TrustAnchorProfile] {
    let Some(target) = target_profile() else {
        return &[];
    };
    #[cfg(feature = "abi-test-fixtures")]
    {
        target.authentication.development_trust_anchors
    }
    #[cfg(not(feature = "abi-test-fixtures"))]
    {
        target.authentication.release_trust_anchors
    }
}

/// Rejects feature selections that the manifest does not declare.
#[cfg(any(
    feature = "sdio",
    feature = "usb-cdc",
    feature = "abi-mpu",
    feature = "abi-relocation"
))]
fn validate_feature_capabilities(
    capabilities: dali_targets::CapabilitiesProfile,
) -> Result<(), BoardError> {
    #[cfg(feature = "sdio")]
    if !capabilities.storage {
        return Err(BoardError::InvalidProfile);
    }
    #[cfg(feature = "usb-cdc")]
    if !capabilities.usb_console {
        return Err(BoardError::InvalidProfile);
    }
    #[cfg(feature = "abi-mpu")]
    if !capabilities.mpu {
        return Err(BoardError::InvalidProfile);
    }
    #[cfg(feature = "abi-relocation")]
    if !capabilities.relocation {
        return Err(BoardError::InvalidProfile);
    }
    Ok(())
}
