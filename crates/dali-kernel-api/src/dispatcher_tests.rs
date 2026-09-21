use super::*;
use crate::installation::ArtifactInstallationBackend;
use dali_usb::installation::{Command, Frame, State};

#[derive(Debug, Eq, PartialEq)]
enum MockError {
    Write,
}

#[derive(Default)]
struct MockBackend {
    events: [u8; 4],
    count: usize,
    fail_write: bool,
}

impl ArtifactInstallationBackend for MockBackend {
    type Error = MockError;

    fn begin(&mut self, _length: u32) -> Result<(), Self::Error> {
        self.events[0] = 1;
        self.count = 1;
        Ok(())
    }

    fn write(&mut self, _offset: u32, _bytes: &[u8]) -> Result<(), Self::Error> {
        if self.fail_write {
            return Err(MockError::Write);
        }
        self.events[1] = 2;
        self.count = 2;
        Ok(())
    }

    fn publish(&mut self) -> Result<(), Self::Error> {
        self.events[2] = 3;
        self.count = 3;
        Ok(())
    }

    fn abort(&mut self) {
        self.events[3] = 4;
        self.count = 4;
    }
}

fn frame(command: Command, offset: u32, total_length: u32, payload: &[u8]) -> Frame<'_> {
    Frame {
        command,
        offset,
        total_length,
        payload,
    }
}

#[test]
fn dispatcher_routes_lifecycle_to_backend() {
    let mut dispatcher = InstallationDispatcher::new(MockBackend::default());
    dispatcher.accept(frame(Command::Begin, 0, 4, &[])).unwrap();
    dispatcher
        .accept(frame(Command::Data, 0, 0, b"test"))
        .unwrap();
    assert_eq!(
        dispatcher
            .accept(frame(Command::Validate, 0, 0, &[]))
            .unwrap(),
        DispatchEvent::ValidationRequired
    );
    dispatcher.mark_validated().unwrap();
    assert_eq!(
        dispatcher
            .accept(frame(Command::Commit, 0, 0, &[]))
            .unwrap(),
        DispatchEvent::Published
    );
    assert_eq!(dispatcher.state(), State::Idle);
}

#[test]
fn backend_failure_aborts_and_resets_protocol() {
    let mut dispatcher = InstallationDispatcher::new(MockBackend {
        fail_write: true,
        ..MockBackend::default()
    });
    dispatcher.accept(frame(Command::Begin, 0, 4, &[])).unwrap();
    assert_eq!(
        dispatcher.accept(frame(Command::Data, 0, 0, b"test")),
        Err(DispatchError::Backend(MockError::Write))
    );
    assert_eq!(dispatcher.state(), State::Idle);
}

#[test]
fn commit_requires_kernel_validation_result() {
    let mut dispatcher = InstallationDispatcher::new(MockBackend::default());
    dispatcher.accept(frame(Command::Begin, 0, 4, &[])).unwrap();
    dispatcher
        .accept(frame(Command::Data, 0, 0, b"test"))
        .unwrap();
    dispatcher
        .accept(frame(Command::Validate, 0, 0, &[]))
        .unwrap();

    assert!(matches!(
        dispatcher.accept(frame(Command::Commit, 0, 0, &[])),
        Err(DispatchError::Protocol(_))
    ));
    assert_eq!(
        dispatcher.state(),
        State::AwaitingValidation { total_length: 4 }
    );
}
