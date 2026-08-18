//! Shared ABI and AMRN format compatibility rules.

/// ABI family represented by a package contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiFamily {
    /// Fixed-origin AMRN v1 packages and the direct service-table ABI.
    Legacy,
    /// Segmented AMRN packages and the privileged SVC ABI.
    Isolation,
}

/// Compatibility metadata for one supported application ABI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiContract {
    /// Numeric ABI version stored in an AMRN header.
    pub abi_version: u8,
    /// ABI family used by the loader and SDK.
    pub family: AbiFamily,
    /// AMRN format selected when the application omits an explicit format.
    pub default_format_version: u8,
}

impl AbiContract {
    /// Returns whether the AMRN format can represent this ABI contract.
    pub const fn supports_format(self, format_version: u8) -> bool {
        match self.family {
            AbiFamily::Legacy => format_version == crate::FORMAT_VERSION,
            AbiFamily::Isolation => {
                format_version == crate::v2::FORMAT_VERSION
                    || format_version == crate::v3::FORMAT_VERSION
                    || format_version == crate::v4::FORMAT_VERSION
            }
        }
    }
}

/// Compatibility contract for the fixed-origin ABI v2.
pub const LEGACY: AbiContract = AbiContract {
    abi_version: crate::ABI_VERSION,
    family: AbiFamily::Legacy,
    default_format_version: crate::FORMAT_VERSION,
};

/// Compatibility contract for the segmented ABI v3.
pub const ISOLATION: AbiContract = AbiContract {
    abi_version: crate::v2::ABI_VERSION,
    family: AbiFamily::Isolation,
    default_format_version: crate::v2::FORMAT_VERSION,
};

/// Returns the repository's compatibility contract for an ABI version.
pub const fn for_abi(abi_version: u8) -> Option<AbiContract> {
    match abi_version {
        value if value == LEGACY.abi_version => Some(LEGACY),
        value if value == ISOLATION.abi_version => Some(ISOLATION),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{AbiFamily, for_abi};

    #[test]
    fn maps_supported_abis_to_their_format_contracts() {
        let legacy = for_abi(crate::ABI_VERSION).expect("legacy ABI is declared");
        assert_eq!(legacy.family, AbiFamily::Legacy);
        assert!(legacy.supports_format(crate::FORMAT_VERSION));

        let isolation = for_abi(crate::v2::ABI_VERSION).expect("isolation ABI is declared");
        assert_eq!(isolation.family, AbiFamily::Isolation);
        assert!(isolation.supports_format(crate::v2::FORMAT_VERSION));
        assert!(isolation.supports_format(crate::v3::FORMAT_VERSION));
        assert!(isolation.supports_format(crate::v4::FORMAT_VERSION));
    }

    #[test]
    fn rejects_unknown_abis_and_incompatible_formats() {
        assert!(for_abi(u8::MAX).is_none());
        let legacy = for_abi(crate::ABI_VERSION).expect("legacy ABI is declared");
        assert!(!legacy.supports_format(crate::v2::FORMAT_VERSION));
    }
}
