use super::{
    Contract, Crc32, Error, HEADER_SIZE, Image, RELOCATION_ENTRY_SIZE, Relocation, RelocationKind,
    Segment, encode, parse,
};

const CODE_ADDRESS: u32 = 0x2000_8000;
const DATA_ADDRESS: u32 = 0x2001_0000;
const REGION_SIZE: u32 = 32 * 1024;
const CODE: &[u8] = &[0, 191, 0, 191, 0, 191, 0, 191];
const DATA: &[u8] = &[1, 2, 3, 4];
const RELOCATIONS: &[Relocation] = &[
    Relocation {
        segment: Segment::Code,
        kind: RelocationKind::ThmMovwAbsNc,
        patch_offset: 0,
        linked_target: CODE_ADDRESS,
        addend: 0,
    },
    Relocation {
        segment: Segment::Data,
        kind: RelocationKind::Abs32,
        patch_offset: 0,
        linked_target: CODE_ADDRESS,
        addend: 0,
    },
];

fn contract() -> Contract {
    Contract {
        target_id: 2,
        code_load_address: CODE_ADDRESS,
        code_capacity: REGION_SIZE,
        data_load_address: DATA_ADDRESS,
        data_capacity: REGION_SIZE,
    }
}

fn image() -> Image<'static> {
    Image {
        code: CODE,
        initialized_data: DATA,
        data_zero_size: 8,
        stack_size: 16,
        linked_code_base: CODE_ADDRESS,
        linked_data_base: DATA_ADDRESS,
        execution_offset: 0,
        relocations: RELOCATIONS,
    }
}

#[test]
fn encodes_and_parses_relocation_package() {
    let expected_size =
        HEADER_SIZE + CODE.len() + DATA.len() + RELOCATIONS.len() * RELOCATION_ENTRY_SIZE;
    let mut output = [0; 128];
    let size = encode(image(), contract(), &mut output).unwrap();
    assert_eq!(size, expected_size);
    let package = parse(&output[..size], contract()).unwrap();
    assert_eq!(package.code, CODE);
    assert_eq!(package.initialized_data, DATA);
    assert_eq!(package.relocation(0).unwrap(), RELOCATIONS[0]);
    assert_eq!(package.relocation(1).unwrap(), RELOCATIONS[1]);
}

#[test]
fn rejects_relocation_target_outside_linked_image() {
    let invalid = [Relocation {
        linked_target: 0x4000_0000,
        ..RELOCATIONS[0]
    }];
    let image = Image {
        relocations: &invalid,
        ..image()
    };
    let mut output = [0; 128];
    assert_eq!(
        encode(image, contract(), &mut output),
        Err(Error::InvalidRelocation)
    );
}

#[test]
fn rejects_unknown_relocation_kind() {
    let mut output = [0; 128];
    let size = encode(image(), contract(), &mut output).unwrap();
    let relocation_start = HEADER_SIZE + CODE.len() + DATA.len();
    output[relocation_start + 1] = u8::MAX;
    let mut checksum = Crc32::new();
    checksum.update(&output[HEADER_SIZE..relocation_start]);
    checksum.update(&output[relocation_start..size]);
    output[56..60].copy_from_slice(&checksum.finish().to_le_bytes());
    assert_eq!(
        parse(&output[..size], contract()),
        Err(Error::InvalidRelocation)
    );
}
