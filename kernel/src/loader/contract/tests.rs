use super::*;

const SLOT: IsolationSlot = IsolationSlot {
    id: 1,
    name: "fixture",
    code_origin: 0x2001_0000,
    code_length: 0x4000,
    data_origin: 0x2001_4000,
    data_length: 0x4000,
    stack_length: 0x1000,
};
const TARGET_ID: u8 = 2;
const CODE_BYTES: usize = 4;
const DATA_BYTES: usize = 4;
const UNDECLARED_SLOT_ID: u8 = 7;
fn cartridge() -> (Vec<u8>, v3::Contract) {
    let contract = v3::Contract {
        target_id: TARGET_ID,
        code_load_address: SLOT.code_origin,
        code_capacity: SLOT.code_length,
        data_load_address: SLOT.data_origin,
        data_capacity: SLOT.data_length,
    };
    let image = v4::Image {
        image: v3::Image {
            code: &[0, 191, 0, 191],
            initialized_data: &[1, 2, 3, 4],
            data_zero_size: 4,
            stack_size: SLOT.stack_length,
            linked_code_base: 0x2000_8000,
            linked_data_base: 0x2000_c000,
            execution_offset: 0,
            relocations: &[],
        },
        metadata: v4::Metadata {
            cartridge_id: [1; 16],
            cartridge_version: v4::Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
            minimum_kernel_version: v4::Version {
                major: 0,
                minor: 1,
                patch: 0,
            },
            required_services: 0,
            slot_id: SLOT.id,
        },
    };
    let mut cartridge = vec![0; v4::HEADER_SIZE + CODE_BYTES + DATA_BYTES];
    let size = v4::encode(image, contract, &mut cartridge).expect("fixture encodes");
    cartridge.truncate(size);
    (cartridge, contract)
}

#[test]
fn rejects_an_undeclared_slot() {
    let (cartridge, contract) = cartridge();
    let mut header = [0; v4::HEADER_SIZE];
    header.copy_from_slice(&cartridge[..v4::HEADER_SIZE]);
    header[v4::SLOT_ID_OFFSET] = UNDECLARED_SLOT_ID;
    assert_eq!(
        select_slot(&header, contract.target_id, &[SLOT]),
        Err(SlotSelectionError::UndeclaredSlot)
    );
}
