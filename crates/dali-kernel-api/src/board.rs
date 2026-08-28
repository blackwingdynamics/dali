//! Hardware-neutral board backend contracts.

use crate::storage::{BlockReader, StorageLifecycleControl};
use dali_targets::WatchdogProfile;
use dali_targets::{CapabilitiesProfile, IsolationSlot, MemoryProfile, TargetProfile};

/// A bounded memory region supplied by a selected board profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRegion {
    /// First address in the region.
    pub origin: u32,
    /// Region length in bytes.
    pub length: u32,
}

impl MemoryRegion {
    /// Creates a region after validating that its range does not overflow.
    pub const fn new(origin: u32, length: u32) -> Option<Self> {
        if origin.checked_add(length).is_none() {
            return None;
        }
        Some(Self { origin, length })
    }

    /// Returns whether an address range is fully contained in this region.
    pub const fn contains(self, origin: u32, length: u32) -> bool {
        let Some(end) = origin.checked_add(length) else {
            return false;
        };
        origin >= self.origin && end <= self.origin.saturating_add(self.length)
    }
}

/// Errors returned when a board backend cannot provide a requested service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoardError {
    /// The selected target profile is not supported by this backend.
    UnsupportedTarget,
    /// The requested hardware capability is not available.
    UnsupportedCapability,
    /// A board resource has already been transferred to its owner.
    ResourceUnavailable,
    /// The backend rejected a malformed or inconsistent profile.
    InvalidProfile,
}

/// Reset source reported by a board watchdog backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResetCause {
    /// Power was applied or the reset state was otherwise cleared.
    PowerOn,
    /// An external reset input caused the restart.
    External,
    /// Software explicitly requested the reset.
    Software,
    /// The hardware watchdog expired.
    Watchdog,
    /// The board could not classify the reset source.
    Unknown,
}

/// Hardware watchdog operations implemented by a board backend.
pub trait WatchdogBackend {
    /// Backend-specific register or HAL error.
    type Error;

    /// Arms the hardware using validated target metadata.
    fn arm(&mut self, profile: WatchdogProfile) -> Result<(), Self::Error>;

    /// Feeds the already-armed hardware watchdog.
    fn feed(&mut self) -> Result<(), Self::Error>;

    /// Reads the reset source before normal bootstrap clears it.
    fn reset_cause(&self) -> ResetCause;

    /// Clears the latched reset source after it has been recorded.
    fn clear_reset_cause(&mut self);
}

/// Read-only identity and capability metadata exposed by a board backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoardInfo {
    /// Stable backend identifier.
    pub backend: &'static str,
    /// Target metadata consumed by hardware-neutral kernel policy.
    pub target: &'static TargetProfile,
    /// Capabilities provided by this backend.
    pub capabilities: CapabilitiesProfile,
    /// Memory contract provided by this backend.
    pub memory: MemoryProfile,
}

/// Hardware-neutral memory-protection operations supplied by a backend.
#[derive(Clone, Copy)]
pub struct MemoryProtectionOperations {
    /// Programs the initial privileged protection map.
    pub configure: fn(MemoryProfile),
    /// Activates user permissions for one validated application slot.
    pub activate_application_regions: fn(IsolationSlot) -> bool,
}

/// Common board lifecycle contract consumed by kernel orchestration.
pub trait BoardBackend {
    /// CPU architecture implementation selected by the firmware composition.
    type Architecture: crate::architecture::ArchitectureBackend;
    /// Board-owned status LED resource.
    type StatusLed;
    /// Board-owned user input resource.
    type UserKey;
    /// Board-owned watchdog backend.
    type Watchdog;
    /// Board-owned storage reader and lifecycle controller.
    type StorageReader: BlockReader + StorageLifecycleControl;

    /// Returns memory-protection operations when the backend provides them.
    fn memory_protection_operations() -> Option<MemoryProtectionOperations> {
        None
    }

    /// Returns immutable board metadata without touching hardware.
    fn info() -> BoardInfo;

    /// Returns the board clock used by the bootstrap diagnostics.
    fn system_clock_mhz() -> u32;

    /// Initializes board clocks and owned peripherals.
    fn initialize() -> Result<Self, BoardError>
    where
        Self: Sized;

    /// Sets the logical status indicator.
    fn set_status_led(&mut self, on: bool) -> Result<(), BoardError>;

    /// Delays for a bounded, profile-defined duration.
    fn delay_ms(&mut self, milliseconds: u32) -> Result<(), BoardError>;

    /// Polls backend-owned user input without blocking kernel policy.
    fn poll_user_key(&mut self);

    /// Transfers the board watchdog to the kernel watchdog owner.
    fn take_watchdog(&mut self) -> Result<Self::Watchdog, BoardError>;

    /// Transfers the initialized storage reader to the kernel owner.
    fn take_storage_reader(&mut self) -> Result<Self::StorageReader, BoardError>;

    /// Returns the reset source captured during early initialization.
    fn reset_cause(&self) -> ResetCause;

    /// Reports whether native application execution is supported.
    fn application_execution_supported() -> bool;
}

/// Board-provided lifecycle services consumed by the kernel bootstrap.
///
/// This contract intentionally contains no vendor type, register, interrupt,
/// linker, or board-specific identifier. A backend crate implements it beside
/// its hardware ownership code.
pub trait BoardServices {
    /// Returns immutable metadata for the selected backend.
    fn info(&self) -> BoardInfo;

    /// Sets the logical status indicator.
    fn set_status_led(&mut self, on: bool) -> Result<(), BoardError>;

    /// Delays for a bounded duration supplied by kernel policy.
    fn delay_ms(&mut self, milliseconds: u32) -> Result<(), BoardError>;
}
