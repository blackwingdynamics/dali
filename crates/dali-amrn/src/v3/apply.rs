use super::{Contract, Error, Header, Relocation, RelocationKind, Segment};

const WORD_BYTES: usize = 4;
const THUMB_CALL_RANGE: i64 = 1 << 24;
const THUMB_CALL_PC_BIAS: u32 = 4;
const THUMB_CALL_ALIGNMENT: i64 = 2;
const MOV_IMMEDIATE_MASK: u16 = 0x000F;
const MOV_I_BIT: u16 = 0x0400;
const MOV_IMMEDIATE3_MASK: u16 = 0x7000;
const MOV_IMMEDIATE8_MASK: u16 = 0x00FF;
const MOV_OPCODE_MASK: u16 = 0xFBF0;
const MOVW_OPCODE: u16 = 0xF240;
const MOVT_OPCODE: u16 = 0xF2C0;
const MOVW_HIGH_SHIFT: u32 = 0;
const MOVT_HIGH_SHIFT: u32 = 16;

/// Applies validated AMRN v3 relocations to copied code and initialized data.
pub fn apply(
    code: &mut [u8],
    data: &mut [u8],
    header: Header,
    contract: Contract,
    relocations: &[Relocation],
) -> Result<(), Error> {
    for relocation in relocations {
        let (target, _delta) = relocated_target(header, contract, relocation)?;
        let patch = match relocation.segment {
            Segment::Code => &mut code[..],
            Segment::Data => &mut data[..],
        };
        match relocation.kind {
            RelocationKind::Abs32 => patch_abs32(patch, relocation.patch_offset, target)?,
            RelocationKind::ThmCall => {
                let patch_address = patch_address(contract, relocation)?;
                patch_thm_call(patch, relocation.patch_offset, patch_address, target)?;
            }
            RelocationKind::ThmMovwAbsNc => {
                patch_thm_mov(patch, relocation.patch_offset, target, MOVW_HIGH_SHIFT)?;
            }
            RelocationKind::ThmMovtAbs => {
                patch_thm_mov(patch, relocation.patch_offset, target, MOVT_HIGH_SHIFT)?;
            }
        }
    }
    Ok(())
}

fn relocated_target(
    header: Header,
    contract: Contract,
    relocation: &Relocation,
) -> Result<(u32, i64), Error> {
    let (linked_base, linked_size, load_address, capacity) =
        target_region(header, contract, relocation.linked_target)?;
    let delta = i64::from(load_address) - i64::from(linked_base);
    let target = i64::from(relocation.linked_target)
        .checked_add(delta)
        .and_then(|value| value.checked_add(i64::from(relocation.addend)))
        .ok_or(Error::RelocationOverflow)?;
    let target_without_thumb = target & !1;
    let selected_end = i64::from(load_address)
        .checked_add(i64::from(capacity))
        .ok_or(Error::AddressOverflow)?;
    let linked_end = i64::from(linked_base)
        .checked_add(i64::from(linked_size))
        .ok_or(Error::AddressOverflow)?;
    if i64::from(relocation.linked_target & !1) >= linked_end
        || target_without_thumb < i64::from(load_address)
        || target_without_thumb >= selected_end
    {
        return Err(Error::RelocationOverflow);
    }
    let target = u32::try_from(target).map_err(|_| Error::RelocationOverflow)?;
    Ok((target, delta))
}

fn target_region(
    header: Header,
    contract: Contract,
    target: u32,
) -> Result<(u32, u32, u32, u32), Error> {
    let code_end = header
        .linked_code_base
        .checked_add(header.code_size)
        .ok_or(Error::AddressOverflow)?;
    if target & !1 >= header.linked_code_base && target & !1 < code_end {
        return Ok((
            header.linked_code_base,
            header.code_size,
            contract.code_load_address,
            contract.code_capacity,
        ));
    }
    let data_size = header
        .data_init_size
        .checked_add(header.data_zero_size)
        .and_then(|size| size.checked_add(header.stack_size))
        .ok_or(Error::AddressOverflow)?;
    let data_end = header
        .linked_data_base
        .checked_add(data_size)
        .ok_or(Error::AddressOverflow)?;
    if target >= header.linked_data_base && target < data_end {
        return Ok((
            header.linked_data_base,
            data_size,
            contract.data_load_address,
            contract.data_capacity,
        ));
    }
    Err(Error::RelocationOverflow)
}

fn patch_address(contract: Contract, relocation: &Relocation) -> Result<u32, Error> {
    let base = match relocation.segment {
        Segment::Code => contract.code_load_address,
        Segment::Data => contract.data_load_address,
    };
    base.checked_add(relocation.patch_offset)
        .ok_or(Error::AddressOverflow)
}

fn patch_abs32(patch: &mut [u8], offset: u32, value: u32) -> Result<(), Error> {
    let bytes = patch_bytes(patch, offset, WORD_BYTES)?;
    bytes.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn patch_thm_call(
    patch: &mut [u8],
    offset: u32,
    patch_address: u32,
    target: u32,
) -> Result<(), Error> {
    let bytes = patch_bytes(patch, offset, WORD_BYTES)?;
    let first = u16::from_le_bytes([bytes[0], bytes[1]]);
    let second = u16::from_le_bytes([bytes[2], bytes[3]]);
    if first & 0xF800 != 0xF000 || second & 0xD000 != 0xD000 {
        return Err(Error::InvalidPatch);
    }
    let source = i64::from(patch_address)
        .checked_add(i64::from(THUMB_CALL_PC_BIAS))
        .ok_or(Error::AddressOverflow)?;
    let displacement = i64::from(target & !1)
        .checked_sub(source)
        .ok_or(Error::RelocationOverflow)?;
    if displacement % THUMB_CALL_ALIGNMENT != 0
        || !(-THUMB_CALL_RANGE..THUMB_CALL_RANGE).contains(&displacement)
    {
        return Err(Error::RelocationOverflow);
    }
    let encoded = displacement as i32 as u32;
    let sign = (encoded >> 24) as u16 & 1;
    let i1 = (encoded >> 23) as u16 & 1;
    let i2 = (encoded >> 22) as u16 & 1;
    let j1 = (!((i1 ^ sign) & 1)) & 1;
    let j2 = (!((i2 ^ sign) & 1)) & 1;
    let first = (first & 0xF800) | (sign << 10) | ((encoded >> 12) as u16 & 0x03FF);
    let second = (second & 0xD000) | (j1 << 13) | (j2 << 11) | ((encoded >> 1) as u16 & 0x07FF);
    bytes[..2].copy_from_slice(&first.to_le_bytes());
    bytes[2..].copy_from_slice(&second.to_le_bytes());
    Ok(())
}

fn patch_thm_mov(patch: &mut [u8], offset: u32, target: u32, high_shift: u32) -> Result<(), Error> {
    let bytes = patch_bytes(patch, offset, WORD_BYTES)?;
    let first = u16::from_le_bytes([bytes[0], bytes[1]]);
    let second = u16::from_le_bytes([bytes[2], bytes[3]]);
    let opcode = if high_shift == MOVW_HIGH_SHIFT {
        MOVW_OPCODE
    } else {
        MOVT_OPCODE
    };
    if first & MOV_OPCODE_MASK != opcode {
        return Err(Error::InvalidPatch);
    }
    let immediate = (target >> high_shift) as u16;
    let immediate4 = (immediate >> 12) & MOV_IMMEDIATE_MASK;
    let immediate3 = (immediate >> 8) & 0x7;
    let first = (first & !MOV_IMMEDIATE_MASK) | immediate4;
    let first = (first & !MOV_I_BIT) | (((immediate >> 11) & 1) * MOV_I_BIT);
    let second =
        (second & !MOV_IMMEDIATE3_MASK) | (immediate3 << 12) | (immediate & MOV_IMMEDIATE8_MASK);
    bytes[..2].copy_from_slice(&first.to_le_bytes());
    bytes[2..].copy_from_slice(&second.to_le_bytes());
    Ok(())
}

fn patch_bytes(patch: &mut [u8], offset: u32, width: usize) -> Result<&mut [u8], Error> {
    let start = usize::try_from(offset).map_err(|_| Error::AddressOverflow)?;
    let end = start.checked_add(width).ok_or(Error::AddressOverflow)?;
    patch.get_mut(start..end).ok_or(Error::InvalidPatch)
}

#[cfg(test)]
mod tests {
    use super::{Contract, Header, Relocation, RelocationKind, Segment, apply};

    const HEADER: Header = Header {
        target_id: 2,
        code_size: 0x20,
        data_init_size: 0x20,
        data_zero_size: 0x10,
        stack_size: 0x10,
        linked_code_base: 0x2000_8000,
        linked_data_base: 0x2001_0000,
        code_load_address: 0x2001_0000,
        data_load_address: 0x2001_8000,
        execution_offset: 0,
        relocation_offset: 0,
        relocation_count: 0,
        crc32: 0,
    };

    const CONTRACT: Contract = Contract {
        target_id: 2,
        code_load_address: 0x2001_0000,
        code_capacity: 0x20,
        data_load_address: 0x2001_8000,
        data_capacity: 0x40,
    };

    #[test]
    fn applies_abs32_with_code_delta() {
        let mut code = [0; 0x20];
        let mut data = [0; 0x20];
        let relocation = Relocation {
            segment: Segment::Data,
            kind: RelocationKind::Abs32,
            patch_offset: 0,
            linked_target: 0x2000_8011,
            addend: 0,
        };
        apply(&mut code, &mut data, HEADER, CONTRACT, &[relocation])
            .expect("absolute relocation should apply");
        assert_eq!(
            u32::from_le_bytes(data[..4].try_into().unwrap()),
            0x2001_0011
        );
    }

    #[test]
    fn rejects_patch_outside_segment() {
        let mut code = [0; 0x20];
        let mut data = [0; 0x20];
        let relocation = Relocation {
            segment: Segment::Code,
            kind: RelocationKind::Abs32,
            patch_offset: 0x20,
            linked_target: 0x2000_8010,
            addend: 0,
        };
        assert_eq!(
            apply(&mut code, &mut data, HEADER, CONTRACT, &[relocation]),
            Err(super::Error::InvalidPatch)
        );
    }

    #[test]
    fn patches_thumb_absolute_immediates() {
        let mut code = [0x41, 0xF2, 0x00, 0x00, 0xC1, 0xF2, 0x00, 0x00];
        let mut data = [0; 0x20];
        let relocations = [
            Relocation {
                segment: Segment::Code,
                kind: RelocationKind::ThmMovwAbsNc,
                patch_offset: 0,
                linked_target: 0x2000_8000,
                addend: 0,
            },
            Relocation {
                segment: Segment::Code,
                kind: RelocationKind::ThmMovtAbs,
                patch_offset: 4,
                linked_target: 0x2000_8000,
                addend: 0,
            },
        ];
        apply(&mut code, &mut data, HEADER, CONTRACT, &relocations)
            .expect("Thumb absolute relocations should apply");
        assert_eq!(&code, &[0x40, 0xF2, 0x00, 0x00, 0xC2, 0xF2, 0x01, 0x00]);
    }

    #[test]
    fn patches_thumb_call() {
        let mut code = [0x00, 0xF0, 0x00, 0xF8, 0, 0, 0, 0];
        let mut data = [0; 0x20];
        let relocation = Relocation {
            segment: Segment::Code,
            kind: RelocationKind::ThmCall,
            patch_offset: 0,
            linked_target: 0x2000_8010,
            addend: 0,
        };
        apply(&mut code, &mut data, HEADER, CONTRACT, &[relocation])
            .expect("Thumb call relocation should apply");
        assert_ne!(&code[..4], &[0x00, 0xF0, 0x00, 0xF8]);
    }
}
