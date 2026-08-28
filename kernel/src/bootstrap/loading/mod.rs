//! Package validation and application launch handoff.

mod package;

#[cfg(feature = "sdio")]
pub(super) use package::load;
