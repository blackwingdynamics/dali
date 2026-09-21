//! Main-context dispatcher for bounded artifact installation.

use crate::installation::ArtifactInstallationBackend;
use dali_usb::installation::{Event as ProtocolEvent, Frame, Session, SessionError};

/// Actions returned to the kernel after one accepted installation frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchEvent {
    /// The backend erased its slot and is ready for data.
    Begun {
        /// Complete candidate length declared by the transport.
        total_length: u32,
    },
    /// The backend accepted one contiguous data chunk.
    Data {
        /// Candidate offset accepted by the backend.
        offset: u32,
        /// Number of payload bytes accepted by the backend.
        payload_length: u16,
    },
    /// The kernel must validate the complete candidate before accepting commit.
    ValidationRequired,
    /// The backend published the validated candidate.
    Published,
    /// The candidate was invalidated.
    Aborted,
}

/// Errors returned while dispatching protocol frames to an installation backend.
#[derive(Debug, Eq, PartialEq)]
pub enum DispatchError<E> {
    /// The frame violates the protocol lifecycle.
    Protocol(SessionError),
    /// The backend rejected the requested operation.
    Backend(E),
}

/// Connects the transport-neutral protocol to one installation capability.
pub struct InstallationDispatcher<B> {
    session: Session,
    backend: B,
}

impl<B> InstallationDispatcher<B>
where
    B: ArtifactInstallationBackend,
{
    /// Creates a dispatcher for a caller-owned installation backend.
    pub const fn new(backend: B) -> Self {
        Self {
            session: Session::new(),
            backend,
        }
    }

    /// Returns the protocol lifecycle state.
    pub const fn state(&self) -> dali_usb::installation::State {
        self.session.state()
    }

    /// Processes one frame in the caller's main context.
    pub fn accept(&mut self, frame: Frame<'_>) -> Result<DispatchEvent, DispatchError<B::Error>> {
        let event = self
            .session
            .accept(frame)
            .map_err(DispatchError::Protocol)?;
        match event {
            ProtocolEvent::Begin { total_length } => {
                self.backend
                    .begin(total_length)
                    .map_err(|error| self.fail(error))?;
                Ok(DispatchEvent::Begun { total_length })
            }
            ProtocolEvent::Data {
                offset,
                payload_length,
            } => {
                self.backend
                    .write(offset, frame.payload)
                    .map_err(|error| self.fail(error))?;
                Ok(DispatchEvent::Data {
                    offset,
                    payload_length,
                })
            }
            ProtocolEvent::Validate => Ok(DispatchEvent::ValidationRequired),
            ProtocolEvent::Commit => {
                self.backend.publish().map_err(|error| self.fail(error))?;
                Ok(DispatchEvent::Published)
            }
            ProtocolEvent::Abort => {
                self.backend.abort();
                Ok(DispatchEvent::Aborted)
            }
        }
    }

    /// Marks the candidate valid after the kernel AMRN validation succeeds.
    pub fn mark_validated(&mut self) -> Result<(), DispatchError<B::Error>> {
        self.session
            .mark_validated()
            .map_err(DispatchError::Protocol)
    }

    fn fail(&mut self, error: B::Error) -> DispatchError<B::Error> {
        self.backend.abort();
        self.session.reset();
        DispatchError::Backend(error)
    }
}

#[cfg(test)]
#[path = "dispatcher_tests.rs"]
mod tests;
