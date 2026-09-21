use super::{Contract, Error, HEADER_SIZE, Image, encode, parse};

const TARGET_ID: u8 = 2;
const CODE_ADDRESS: u32 = 0x2000_8000;
const DATA_ADDRESS: u32 = 0x2001_0000;
const REGION_SIZE: u32 = 32 * 1024;
const CODE: &[u8] = &[0, 191, 0, 191];
const DATA: &[u8] = &[1, 2, 3, 4];

fn contract() -> Contract {
    Contract {
        target_id: TARGET_ID,
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
        execution_offset: 0,
    }
}

#[test]
fn encodes_and_parses_legacy_isolation_cartridge() {
    let mut output = [0; HEADER_SIZE + CODE.len() + DATA.len()];
    let size = encode(image(), contract(), &mut output).unwrap();
    let cartridge = parse(&output[..size], contract()).unwrap();
    assert_eq!(cartridge.code, CODE);
    assert_eq!(cartridge.initialized_data, DATA);
    assert_eq!(cartridge.entry_address, CODE_ADDRESS);
    assert_eq!(cartridge.psp_stack_bottom, DATA_ADDRESS + 12);
    assert_eq!(cartridge.psp_stack_top, DATA_ADDRESS + 28);
}

#[test]
fn rejects_data_region_overflow() {
    let mut output = [0; HEADER_SIZE + CODE.len() + DATA.len()];
    let oversized = Image {
        data_zero_size: REGION_SIZE,
        ..image()
    };
    assert_eq!(
        encode(oversized, contract(), &mut output),
        Err(Error::RegionOverflow)
    );
}

#[test]
fn rejects_trailing_cartridge_bytes() {
    let mut output = [0; HEADER_SIZE + CODE.len() + DATA.len() + 1];
    let size = encode(image(), contract(), &mut output).unwrap();
    assert_eq!(
        parse(&output[..size + 1], contract()),
        Err(Error::InvalidPayload)
    );
}
