extern crate std;

use super::*;

const CONTRACT: v3::Contract = v3::Contract {
    target_id: 2,
    code_load_address: 0x2001_0000,
    code_capacity: 0x4000,
    data_load_address: 0x2001_4000,
    data_capacity: 0x4000,
};

fn valid_package() -> std::vec::Vec<u8> {
    let code = [0, 0, 0, 0];
    let mut package = std::vec![0; HEADER_SIZE + code.len()];
    package[..MAGIC.len()].copy_from_slice(&MAGIC);
    package[4] = FORMAT_VERSION;
    package[5] = CONTRACT.target_id;
    package[6..8].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    package[8..12].copy_from_slice(&(code.len() as u32).to_le_bytes());
    package[20..24].copy_from_slice(&0x1000u32.to_le_bytes());
    package[24..28].copy_from_slice(&0x2000_8000u32.to_le_bytes());
    package[28..32].copy_from_slice(&0x2000_c000u32.to_le_bytes());
    package[32..36].copy_from_slice(&CONTRACT.code_load_address.to_le_bytes());
    package[36..40].copy_from_slice(&CONTRACT.data_load_address.to_le_bytes());
    let relocation_offset = package.len() as u32;
    package[44..48].copy_from_slice(&relocation_offset.to_le_bytes());
    package[52..54].copy_from_slice(&(RELOCATION_ENTRY_SIZE as u16).to_le_bytes());
    package[60] = ABI_VERSION;
    package[80] = 1;
    package[96..98].copy_from_slice(&1u16.to_le_bytes());
    package[102..104].copy_from_slice(&1u16.to_le_bytes());
    let mut payload_crc = Crc32::new();
    payload_crc.update(&code);
    let payload_crc = payload_crc.finish();
    package[56..60].copy_from_slice(&payload_crc.to_le_bytes());
    package[HEADER_SIZE..].copy_from_slice(&code);
    let package_crc = package_checksum(&package, PACKAGE_CRC32_OFFSET);
    package[PACKAGE_CRC32_OFFSET..PACKAGE_CRC32_OFFSET + 4]
        .copy_from_slice(&package_crc.to_le_bytes());
    package
}

#[test]
fn parses_v4_identity_and_compatibility_metadata() {
    let package = valid_package();
    let parsed = parse(&package, CONTRACT).expect("v4 package is valid");
    assert_eq!(parsed.header.metadata.package_id[0], 1);
    assert_eq!(parsed.header.metadata.package_version.major, 1);
    assert_eq!(parsed.header.metadata.minimum_kernel_version.major, 1);
    assert_eq!(parsed.code.len(), 4);
}

#[test]
fn rejects_an_empty_package_identity() {
    let mut package = valid_package();
    package[80] = 0;
    assert_eq!(parse(&package, CONTRACT), Err(Error::InvalidIdentity));
}

#[test]
fn rejects_a_package_checksum_mismatch() {
    let mut package = valid_package();
    package[80] = 2;
    assert_eq!(parse(&package, CONTRACT), Err(Error::CrcMismatch));
}
