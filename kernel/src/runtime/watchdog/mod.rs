//! Hardware-neutral watchdog ownership and reset-cause contracts.

use dali_targets::WatchdogProfile;

/// Reset source reported by a platform watchdog backend.
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
    /// The platform could not classify the reset source.
    Unknown,
}

/// Kernel-owned source allowed to feed the watchdog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeedOwner {
    /// The bounded kernel heartbeat loop owns the feed operation.
    KernelHeartbeat,
}

/// Errors raised while validating or advancing watchdog ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchdogError {
    /// The target profile contains an unusable watchdog controller name.
    MissingController,
    /// A zero timeout cannot provide a recovery window.
    ZeroTimeout,
    /// A zero feed interval cannot represent a bounded heartbeat cadence.
    ZeroFeedInterval,
    /// Feeding at or after the timeout cannot guarantee recovery.
    FeedIntervalExceedsTimeout,
    /// The watchdog was already armed.
    AlreadyArmed,
    /// The watchdog is not armed.
    NotArmed,
    /// The caller does not own the active feed lease.
    UnauthorizedOwner,
}

/// Hardware-neutral state machine for one kernel watchdog lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchdogState {
    /// Hardware must remain disabled until an owner is established.
    Disabled,
    /// The declared owner is responsible for bounded feed operations.
    Armed(FeedOwner),
}

/// Validates watchdog metadata and tracks its single kernel feed owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WatchdogContract {
    profile: WatchdogProfile,
    state: WatchdogState,
}

impl WatchdogContract {
    /// Creates a disabled contract from target-declared watchdog facts.
    pub const fn new(profile: WatchdogProfile) -> Result<Self, WatchdogError> {
        if profile.controller.is_empty() {
            return Err(WatchdogError::MissingController);
        }
        if profile.timeout_ms == 0 {
            return Err(WatchdogError::ZeroTimeout);
        }
        if profile.feed_interval_ms == 0 {
            return Err(WatchdogError::ZeroFeedInterval);
        }
        if profile.feed_interval_ms >= profile.timeout_ms {
            return Err(WatchdogError::FeedIntervalExceedsTimeout);
        }
        Ok(Self {
            profile,
            state: WatchdogState::Disabled,
        })
    }

    /// Returns the target-declared hardware facts.
    pub const fn profile(self) -> WatchdogProfile {
        self.profile
    }

    /// Returns the current ownership state.
    pub const fn state(self) -> WatchdogState {
        self.state
    }

    /// Establishes the sole feed owner before hardware is armed.
    pub fn arm(&mut self, owner: FeedOwner) -> Result<(), WatchdogError> {
        if self.state != WatchdogState::Disabled {
            return Err(WatchdogError::AlreadyArmed);
        }
        self.state = WatchdogState::Armed(owner);
        Ok(())
    }

    /// Accepts one feed only from the established owner.
    pub fn feed(&self, owner: FeedOwner) -> Result<(), WatchdogError> {
        match self.state {
            WatchdogState::Disabled => Err(WatchdogError::NotArmed),
            WatchdogState::Armed(active) if active == owner => Ok(()),
            WatchdogState::Armed(_) => Err(WatchdogError::UnauthorizedOwner),
        }
    }
}

/// Platform adapter implemented by each hardware watchdog backend.
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

/// Errors returned by the runtime wrapper around a platform backend.
#[derive(Debug, Eq, PartialEq)]
pub enum WatchdogRuntimeError<E> {
    /// The ownership contract rejected the requested operation.
    Contract(WatchdogError),
    /// The hardware backend rejected or failed the operation.
    Backend(E),
}

/// Couples one platform watchdog backend to the kernel ownership contract.
pub struct WatchdogRuntime<B> {
    backend: B,
    contract: WatchdogContract,
}

impl<B> WatchdogRuntime<B>
where
    B: WatchdogBackend,
{
    /// Creates a disabled runtime wrapper after validating target metadata.
    pub fn new(backend: B, profile: WatchdogProfile) -> Result<Self, WatchdogError> {
        let contract = WatchdogContract::new(profile)?;
        Ok(Self { backend, contract })
    }

    /// Arms hardware only after establishing the requested feed owner.
    pub fn arm(&mut self, owner: FeedOwner) -> Result<(), WatchdogRuntimeError<B::Error>> {
        if self.contract.state() != WatchdogState::Disabled {
            return Err(WatchdogRuntimeError::Contract(WatchdogError::AlreadyArmed));
        }
        self.backend
            .arm(self.contract.profile())
            .map_err(WatchdogRuntimeError::Backend)?;
        self.contract
            .arm(owner)
            .map_err(WatchdogRuntimeError::Contract)
    }

    /// Feeds hardware only through the established ownership contract.
    pub fn feed(&mut self, owner: FeedOwner) -> Result<(), WatchdogRuntimeError<B::Error>> {
        self.contract
            .feed(owner)
            .map_err(WatchdogRuntimeError::Contract)?;
        self.backend.feed().map_err(WatchdogRuntimeError::Backend)
    }

    /// Returns the reset cause captured by the platform backend.
    pub fn reset_cause(&self) -> ResetCause {
        self.backend.reset_cause()
    }
}

#[cfg(test)]
mod tests {
    use dali_targets::WatchdogProfile;

    const PROFILE: WatchdogProfile = WatchdogProfile {
        controller: "test-controller",
        timeout_ms: 2_000,
        feed_interval_ms: 500,
        reset_cause_supported: true,
    };

    #[test]
    fn validates_profile_and_requires_explicit_owner() {
        let contract = super::WatchdogContract::new(PROFILE).expect("profile is valid");
        assert_eq!(contract.state(), super::WatchdogState::Disabled);
    }

    #[test]
    fn accepts_feed_only_after_kernel_heartbeat_arms_it() {
        let mut contract = super::WatchdogContract::new(PROFILE).expect("profile is valid");
        assert_eq!(
            contract.feed(super::FeedOwner::KernelHeartbeat),
            Err(super::WatchdogError::NotArmed)
        );
        contract
            .arm(super::FeedOwner::KernelHeartbeat)
            .expect("owner can arm once");
        assert_eq!(contract.feed(super::FeedOwner::KernelHeartbeat), Ok(()));
    }

    #[test]
    fn rejects_invalid_timing_metadata() {
        let invalid = WatchdogProfile {
            feed_interval_ms: PROFILE.timeout_ms,
            ..PROFILE
        };
        assert_eq!(
            super::WatchdogContract::new(invalid),
            Err(super::WatchdogError::FeedIntervalExceedsTimeout)
        );
    }

    #[test]
    fn rejects_second_arm_attempt() {
        let mut contract = super::WatchdogContract::new(PROFILE).expect("profile is valid");
        contract
            .arm(super::FeedOwner::KernelHeartbeat)
            .expect("owner can arm once");
        assert_eq!(
            contract.arm(super::FeedOwner::KernelHeartbeat),
            Err(super::WatchdogError::AlreadyArmed)
        );
    }
}
