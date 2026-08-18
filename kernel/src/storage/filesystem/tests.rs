#[test]
fn classifies_empty_root_as_no_package() {
    assert_eq!(
        super::RootDirectoryReport { amrn_file_count: 0 }.package_selection(),
        super::RootPackageSelection::None
    );
}

#[test]
fn classifies_one_root_package_as_single() {
    assert_eq!(
        super::RootDirectoryReport { amrn_file_count: 1 }.package_selection(),
        super::RootPackageSelection::Single
    );
}

#[test]
fn classifies_multiple_root_packages_as_ambiguous() {
    assert_eq!(
        super::RootDirectoryReport { amrn_file_count: 2 }.package_selection(),
        super::RootPackageSelection::Ambiguous
    );
    assert_eq!(
        super::RootDirectoryReport {
            amrn_file_count: u32::MAX,
        }
        .package_selection(),
        super::RootPackageSelection::Ambiguous
    );
}
