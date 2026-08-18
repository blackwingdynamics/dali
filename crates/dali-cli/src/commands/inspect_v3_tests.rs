use super::inspect_bytes;

#[test]
fn inspect_reports_v3_relocation_fields() {
    let contract = dali_amrn::v3::Contract {
        target_id: 2,
        code_load_address: 0x2000_8000,
        code_capacity: 16 * 1024,
        data_load_address: 0x2000_C000,
        data_capacity: 16 * 1024,
    };
    let image = dali_amrn::v3::Image {
        code: &[0, 191, 0, 191],
        initialized_data: &[1, 2, 3, 4],
        data_zero_size: 8,
        stack_size: 16,
        linked_code_base: 0x2000_8000,
        linked_data_base: 0x2000_C000,
        execution_offset: 0,
        relocations: &[],
    };
    let mut package = vec![0; dali_amrn::v3::HEADER_SIZE + 8];
    let written = dali_amrn::v3::encode(image, contract, &mut package)
        .expect("v3 test package should encode");
    package.truncate(written);
    let report = inspect_bytes(&package).expect("v3 package should inspect");
    assert!(report.contains("format_version: 3"));
    assert!(report.contains("relocation_count: 0"));
    assert!(report.contains("abi_version: 3"));
}
