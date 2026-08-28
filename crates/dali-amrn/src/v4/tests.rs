extern crate std;

use super::*;

const CONTRACT: v3::Contract = v3::Contract {
    target_id: 2,
    code_load_address: 0x2001_0000,
    code_capacity: 0x4000,
    data_load_address: 0x2001_4000,
    data_capacity: 0x4000,
};
const CRC32_FIELD_LENGTH: usize = core::mem::size_of::<u32>();

fn valid_cartridge() -> std::vec::Vec<u8> {
    let code = [0, 0, 0, 0];
    let mut cartridge = std::vec![0; HEADER_SIZE + code.len()];
    cartridge[..MAGIC.len()].copy_from_slice(&MAGIC);
    cartridge[4] = FORMAT_VERSION;
    cartridge[5] = CONTRACT.target_id;
    cartridge[6..8].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    cartridge[8..12].copy_from_slice(&(code.len() as u32).to_le_bytes());
    cartridge[20..24].copy_from_slice(&0x1000u32.to_le_bytes());
    cartridge[24..28].copy_from_slice(&0x2000_8000u32.to_le_bytes());
    cartridge[28..32].copy_from_slice(&0x2000_c000u32.to_le_bytes());
    cartridge[32..36].copy_from_slice(&CONTRACT.code_load_address.to_le_bytes());
    cartridge[36..40].copy_from_slice(&CONTRACT.data_load_address.to_le_bytes());
    let relocation_offset = cartridge.len() as u32;
    cartridge[44..48].copy_from_slice(&relocation_offset.to_le_bytes());
    cartridge[52..54].copy_from_slice(&(RELOCATION_ENTRY_SIZE as u16).to_le_bytes());
    cartridge[60] = ABI_VERSION;
    cartridge[80] = 1;
    cartridge[96..98].copy_from_slice(&1u16.to_le_bytes());
    cartridge[102..104].copy_from_slice(&1u16.to_le_bytes());
    let mut payload_crc = Crc32::new();
    payload_crc.update(&code);
    let payload_crc = payload_crc.finish();
    cartridge[56..60].copy_from_slice(&payload_crc.to_le_bytes());
    cartridge[HEADER_SIZE..].copy_from_slice(&code);
    let cartridge_crc = cartridge_checksum(&cartridge);
    cartridge[CARTRIDGE_CRC32_OFFSET..CARTRIDGE_CRC32_OFFSET + CRC32_FIELD_LENGTH]
        .copy_from_slice(&cartridge_crc.to_le_bytes());
    cartridge
}

#[test]
fn encodes_and_parses_identity_metadata() {
    let image = Image {
        image: v3::Image {
            code: &[0, 0, 0, 0],
            initialized_data: &[],
            data_zero_size: 0,
            stack_size: 0x1000,
            linked_code_base: 0x2000_8000,
            linked_data_base: 0x2000_c000,
            execution_offset: 0,
            relocations: &[],
        },
        metadata: Metadata {
            cartridge_id: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            cartridge_version: Version {
                major: 1,
                minor: 2,
                patch: 3,
            },
            minimum_kernel_version: Version {
                major: 0,
                minor: 1,
                patch: 0,
            },
            required_services: 0,
            slot_id: 1,
        },
    };
    let mut cartridge = std::vec![0; HEADER_SIZE + 4];
    let size = encode(image, CONTRACT, &mut cartridge).expect("v4 cartridge encodes");
    cartridge.truncate(size);
    let parsed = parse(&cartridge, CONTRACT).expect("encoded v4 cartridge parses");
    assert_eq!(parsed.header.metadata.cartridge_version.patch, 3);
    assert_eq!(parsed.header.metadata.slot_id, 1);
}

#[test]
fn parses_identity_and_compatibility_metadata() {
    let cartridge = valid_cartridge();
    let parsed = parse(&cartridge, CONTRACT).expect("v4 cartridge is valid");
    assert_eq!(parsed.header.metadata.cartridge_id[0], 1);
    assert_eq!(parsed.header.metadata.cartridge_version.major, 1);
    assert_eq!(parsed.header.metadata.minimum_kernel_version.major, 1);
    assert_eq!(parsed.code.len(), 4);
}

#[test]
fn rejects_an_empty_cartridge_identity() {
    let mut cartridge = valid_cartridge();
    cartridge[80] = 0;
    assert_eq!(parse(&cartridge, CONTRACT), Err(Error::InvalidIdentity));
}

#[test]
fn rejects_a_cartridge_checksum_mismatch() {
    let mut cartridge = valid_cartridge();
    cartridge[80] = 2;
    assert_eq!(parse(&cartridge, CONTRACT), Err(Error::CrcMismatch));
}

#[test]
fn cartridge_checksum_excludes_its_own_field() {
    let mut cartridge = valid_cartridge();
    let expected = cartridge_checksum(&cartridge);
    cartridge[CARTRIDGE_CRC32_OFFSET..CARTRIDGE_CRC32_OFFSET + CRC32_FIELD_LENGTH].fill(0xA5);
    assert_eq!(cartridge_checksum(&cartridge), expected);
}

#[test]
fn cartridge_checksum_includes_bytes_before_the_checksum_field() {
    let mut cartridge = valid_cartridge();
    let expected = cartridge_checksum(&cartridge);
    cartridge[CARTRIDGE_CRC32_OFFSET - 1] ^= 0x01;
    assert_ne!(cartridge_checksum(&cartridge), expected);
}

#[test]
fn cartridge_checksum_includes_bytes_at_the_reserved_boundary() {
    let mut cartridge = valid_cartridge();
    let expected = cartridge_checksum(&cartridge);
    cartridge[RESERVED_BYTES_OFFSET] ^= 0x01;
    assert_ne!(cartridge_checksum(&cartridge), expected);
}
