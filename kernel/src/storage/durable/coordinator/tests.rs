use super::*;
use crate::storage::durable::DurableArtifact;

struct TestStorage {
    writes: usize,
    candidate: [u8; 4],
}

impl DurableStorageAdapter for TestStorage {
    type Error = ();

    fn read_artifact(
        &mut self,
        artifact: DurableArtifact,
        output: &mut [u8],
    ) -> Result<usize, Self::Error> {
        if artifact == DurableArtifact::SlotB {
            output[..self.candidate.len()].copy_from_slice(&self.candidate);
            Ok(self.candidate.len())
        } else {
            Ok(0)
        }
    }

    fn write_artifact(&mut self, _: DurableArtifact, _: &[u8]) -> Result<(), Self::Error> {
        self.writes += 1;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn generation(version: u64) -> DurableGeneration {
    DurableGeneration {
        version,
        digest: [0x11; 32],
        length: 4,
    }
}

#[test]
fn advances_only_after_each_durable_transition() {
    let mut coordinator = PersistenceCoordinator::new(
        TestStorage {
            writes: 0,
            candidate: [1, 2, 3, 4],
        },
        JournalSlot::A,
        generation(1),
        2,
    );
    assert_eq!(
        coordinator.write_candidate(&[1, 2, 3, 4], generation(2)),
        Ok(())
    );
    assert_eq!(coordinator.state(), PersistenceState::CandidateWritten);
    let mut readback = [0; 4];
    assert_eq!(
        coordinator.verify_candidate(generation(2), &[1, 2, 3, 4], &mut readback),
        Ok(())
    );
    assert_eq!(coordinator.state(), PersistenceState::CandidateVerified);
    assert_eq!(coordinator.commit(), Ok(()));
    assert_eq!(coordinator.state(), PersistenceState::Active);
    assert_eq!(coordinator.into_storage().writes, 3);
}

#[test]
fn rejects_unverified_commit_and_mismatched_generation() {
    let mut coordinator = PersistenceCoordinator::new(
        TestStorage {
            writes: 0,
            candidate: [1, 2, 3, 4],
        },
        JournalSlot::A,
        generation(1),
        2,
    );
    assert_eq!(
        coordinator.commit(),
        Err(PersistenceError::InvalidTransition)
    );
    coordinator
        .write_candidate(&[1, 2, 3, 4], generation(2))
        .expect("candidate writes");
    assert_eq!(
        coordinator.verify_candidate(generation(3), &[1, 2, 3, 4], &mut [0; 4]),
        Err(PersistenceError::CandidateMismatch)
    );
}

#[test]
fn rejects_equal_and_older_generations_before_writing() {
    let mut coordinator = PersistenceCoordinator::new(
        TestStorage {
            writes: 0,
            candidate: [1, 2, 3, 4],
        },
        JournalSlot::A,
        generation(2),
        3,
    );

    assert_eq!(
        coordinator.write_candidate(&[1, 2, 3, 4], generation(2)),
        Err(PersistenceError::Rollback)
    );
    assert_eq!(
        coordinator.write_candidate(&[1, 2, 3, 4], generation(1)),
        Err(PersistenceError::Rollback)
    );
    assert_eq!(coordinator.into_storage().writes, 0);
}

#[test]
fn authorizes_only_a_newer_generation_before_installation() {
    let coordinator = PersistenceCoordinator::new(
        TestStorage {
            writes: 0,
            candidate: [1, 2, 3, 4],
        },
        JournalSlot::A,
        generation(4),
        5,
    );

    assert_eq!(coordinator.authorize_candidate(generation(5)), Ok(()));
    assert_eq!(
        coordinator.authorize_candidate(generation(4)),
        Err(PersistenceError::Rollback)
    );
}

#[test]
fn rejects_an_unidentifiable_generation_before_writing() {
    let mut coordinator = PersistenceCoordinator::new(
        TestStorage {
            writes: 0,
            candidate: [1, 2, 3, 4],
        },
        JournalSlot::A,
        generation(1),
        2,
    );
    let invalid = DurableGeneration {
        version: 2,
        digest: [0; SHA256_LENGTH],
        length: 4,
    };
    assert_eq!(
        coordinator.write_candidate(&[1, 2, 3, 4], invalid),
        Err(PersistenceError::InvalidGeneration)
    );
}
