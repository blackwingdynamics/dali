extern crate std;

use super::*;

const CONTRACT: v3::Contract = v3::Contract {
    target_id: 2,
    code_load_address: 0x2001_0000,
    code_capacity: 0x4000,
    data_load_address: 0x2001_4000,
    data_capacity: 0x4000,
};

fn image() -> Image<'static> {
    Image {
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
        metadata: v4::Metadata {
            package_id: [1; 16],
            package_version: v4::Version {
                major: 1,
                minor: 2,
                patch: 3,
            },
            minimum_kernel_version: v4::Version {
                major: 0,
                minor: 1,
                patch: 0,
            },
            required_services: 0,
            slot_id: 1,
        },
    }
}

#[test]
fn encodes_and_parses_the_signed_container() {
    let signed_capacity = HEADER_SIZE + 4;
    let mut signed = std::vec![0; signed_capacity];
    let signed_size =
        encode_unsigned(image(), CONTRACT, &mut signed).expect("signed range encodes");
    signed.truncate(signed_size);
    let mut package = std::vec![0; signed_size + SIGNATURE_SIZE];
    let size =
        append_signature(&signed, &[7; 16], &[9; 64], &mut package).expect("trailer appends");
    package.truncate(size);
    let parsed = parse(&package, CONTRACT).expect("signed package parses");
    assert_eq!(parsed.header.signature.key_id, &[7; 16]);
    assert_eq!(parsed.signed_bytes(), &package[..signed_size]);
}

#[test]
fn rejects_a_signature_boundary_change() {
    let mut signed = std::vec![0; HEADER_SIZE + 4];
    let signed_size =
        encode_unsigned(image(), CONTRACT, &mut signed).expect("signed range encodes");
    signed.truncate(signed_size);
    let mut package = std::vec![0; signed_size + SIGNATURE_SIZE];
    append_signature(&signed, &[7; 16], &[9; 64], &mut package).expect("trailer appends");
    package[SIGNATURE_OFFSET_FIELD] ^= 1;
    assert_eq!(parse(&package, CONTRACT), Err(Error::InvalidSignature));
}

#[test]
fn rejects_crc_changes_inside_the_signed_range() {
    let mut signed = std::vec![0; HEADER_SIZE + 4];
    let signed_size =
        encode_unsigned(image(), CONTRACT, &mut signed).expect("signed range encodes");
    signed.truncate(signed_size);
    let mut package = std::vec![0; signed_size + SIGNATURE_SIZE];
    append_signature(&signed, &[7; 16], &[9; 64], &mut package).expect("trailer appends");
    package[HEADER_SIZE] ^= 1;
    assert_eq!(parse(&package, CONTRACT), Err(Error::CrcMismatch));
}
