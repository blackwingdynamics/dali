#[test]
fn classifies_empty_root_as_no_cartridge() {
    assert_eq!(
        super::RootDirectoryReport { amrn_file_count: 0 }.cartridge_selection(),
        super::RootCartridgeSelection::None
    );
}

#[test]
fn classifies_one_root_cartridge_as_single() {
    assert_eq!(
        super::RootDirectoryReport { amrn_file_count: 1 }.cartridge_selection(),
        super::RootCartridgeSelection::Single
    );
}

#[test]
fn classifies_multiple_root_cartridges_as_ambiguous() {
    assert_eq!(
        super::RootDirectoryReport { amrn_file_count: 2 }.cartridge_selection(),
        super::RootCartridgeSelection::Ambiguous
    );
    assert_eq!(
        super::RootDirectoryReport {
            amrn_file_count: u32::MAX,
        }
        .cartridge_selection(),
        super::RootCartridgeSelection::Ambiguous
    );
}
