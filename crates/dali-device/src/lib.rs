//! Hardware-neutral device discovery records and normalization rules.

use std::cmp::Ordering;

/// A host transport through which a Dali device can be observed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Transport {
    /// SWD debug-probe inventory.
    Probe,
    /// USB DFU bootloader inventory.
    Dfu,
    /// USB CDC runtime-console inventory.
    Cdc,
}

impl Transport {
    /// Returns the stable CLI spelling for this transport.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Probe => "probe",
            Self::Dfu => "dfu",
            Self::Cdc => "cdc",
        }
    }
}

/// The normalized availability state of a discovered device.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum State {
    /// The transport can use the visible device.
    Available,
    /// The device identity matches a declared target profile.
    Identified,
    /// The device is visible but target matching is incomplete.
    Unidentified,
    /// The host reports that another process owns the device.
    InUse,
    /// Discovery returned a bounded diagnostic error for the device.
    Error,
}

impl State {
    /// Returns the stable CLI spelling for this state.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Identified => "identified",
            Self::Unidentified => "unidentified",
            Self::InUse => "in_use",
            Self::Error => "error",
        }
    }
}

/// An operation supported by a discovered transport endpoint.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Capability {
    /// The endpoint can be used for debug attachment.
    Attach,
    /// The endpoint can receive a firmware image.
    Flash,
    /// The endpoint can provide a runtime console.
    Console,
}

impl Capability {
    /// Returns the stable CLI spelling for this capability.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Attach => "attach",
            Self::Flash => "flash",
            Self::Console => "console",
        }
    }
}

/// A normalized, non-secret record produced by a transport adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceRecord {
    /// Stable identifier supplied by the transport adapter.
    pub id: String,
    /// Transport through which this record was observed.
    pub transport: Transport,
    /// Matching target profile, when identity resolution succeeded.
    pub target: Option<String>,
    /// Transport-reported vendor identifier.
    pub vendor: Option<String>,
    /// Transport-reported product or probe name.
    pub product: Option<String>,
    /// Transport-reported serial identifier.
    pub serial: Option<String>,
    /// Host path, when the transport exposes one.
    pub path: Option<String>,
    /// Normalized availability state.
    pub state: State,
    /// Operations supported by the observed endpoint.
    pub capabilities: Vec<Capability>,
    /// Bounded diagnostic detail without raw memory or credentials.
    pub diagnostic: Option<String>,
}

/// Removes duplicate records and orders records by the discovery contract.
pub fn normalize(mut records: Vec<DeviceRecord>) -> Vec<DeviceRecord> {
    records.sort_by(record_ordering);
    records.dedup_by(|left, right| left.transport == right.transport && left.id == right.id);
    records
}

fn record_ordering(left: &DeviceRecord, right: &DeviceRecord) -> Ordering {
    left.transport
        .cmp(&right.transport)
        .then_with(|| left.id.cmp(&right.id))
}

#[cfg(test)]
mod tests {
    use super::{Capability, DeviceRecord, State, Transport, normalize};

    fn record(transport: Transport, id: &str) -> DeviceRecord {
        DeviceRecord {
            id: id.to_owned(),
            transport,
            target: None,
            vendor: None,
            product: None,
            serial: None,
            path: None,
            state: State::Available,
            capabilities: vec![Capability::Console],
            diagnostic: None,
        }
    }

    #[test]
    fn normalizes_transport_and_identifier_order() {
        let records = normalize(vec![
            record(Transport::Cdc, "z"),
            record(Transport::Probe, "b"),
            record(Transport::Probe, "a"),
            record(Transport::Dfu, "a"),
        ]);
        let keys: Vec<_> = records
            .iter()
            .map(|record| (record.transport, record.id.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                (Transport::Probe, "a"),
                (Transport::Probe, "b"),
                (Transport::Dfu, "a"),
                (Transport::Cdc, "z"),
            ]
        );
    }

    #[test]
    fn collapses_duplicates_only_within_one_transport() {
        let records = normalize(vec![
            record(Transport::Cdc, "same"),
            record(Transport::Cdc, "same"),
            record(Transport::Probe, "same"),
        ]);
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn exposes_stable_contract_spellings() {
        assert_eq!(Transport::Probe.as_str(), "probe");
        assert_eq!(State::Unidentified.as_str(), "unidentified");
        assert_eq!(Capability::Flash.as_str(), "flash");
    }
}
