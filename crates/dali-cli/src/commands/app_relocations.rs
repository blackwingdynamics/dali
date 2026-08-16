use std::{fs, path::Path};

use dali_amrn::v3::{Relocation, RelocationKind, Segment};
use object::{Object, ObjectSection, ObjectSymbol, RelocationFlags, RelocationTarget};

const CODE_SECTION: &str = ".dali_code";
const DATA_SECTION: &str = ".dali_data";

/// Relocation records retained from one linked application ELF.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct RelocationArtifact {
    pub(super) linked_code_base: u32,
    pub(super) linked_data_base: u32,
    pub(super) relocations: Vec<Relocation>,
}

pub(super) fn extract(path: &Path) -> Result<RelocationArtifact, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read relocation ELF {}: {error}", path.display()))?;
    let file = object::File::parse(bytes.as_slice())
        .map_err(|error| format!("cannot parse relocation ELF {}: {error}", path.display()))?;
    let code = file
        .section_by_name(CODE_SECTION)
        .ok_or_else(|| format!("relocation ELF is missing {CODE_SECTION}"))?;
    let data = file
        .section_by_name(DATA_SECTION)
        .ok_or_else(|| format!("relocation ELF is missing {DATA_SECTION}"))?;
    let mut relocations = Vec::new();
    collect_section_relocations(&file, &code, Segment::Code, &mut relocations)?;
    collect_section_relocations(&file, &data, Segment::Data, &mut relocations)?;
    Ok(RelocationArtifact {
        linked_code_base: checked_address(code.address(), CODE_SECTION)?,
        linked_data_base: checked_address(data.address(), DATA_SECTION)?,
        relocations,
    })
}

fn collect_section_relocations<'data, 'file, F: Object<'data>>(
    file: &'file F,
    section: &F::Section<'file>,
    segment: Segment,
    output: &mut Vec<Relocation>,
) -> Result<(), String> {
    let section_base = section.address();
    for (offset, relocation) in section.relocations() {
        if output.len() >= dali_amrn::v3::MAX_RELOCATION_ENTRIES {
            return Err(format!(
                "relocation count exceeds AMRN limit {}",
                dali_amrn::v3::MAX_RELOCATION_ENTRIES
            ));
        }
        if relocation.target() == RelocationTarget::Absolute {
            return Err("relocation has an absolute target without a symbol".to_owned());
        }
        let kind = relocation_kind(&relocation)?;
        let patch_offset = offset
            .checked_sub(section_base)
            .ok_or_else(|| "relocation offset precedes its section".to_owned())
            .and_then(|value| checked_address(value, "relocation offset"))?;
        let linked_target = linked_target(file, relocation.target())?;
        let addend = i32::try_from(relocation.addend())
            .map_err(|_| "relocation addend does not fit in AMRN i32".to_owned())?;
        output.push(Relocation {
            segment,
            kind,
            patch_offset,
            linked_target,
            addend,
        });
    }
    Ok(())
}

fn linked_target<'data, 'file, F: Object<'data>>(
    file: &'file F,
    target: RelocationTarget,
) -> Result<u32, String> {
    let address = match target {
        RelocationTarget::Symbol(index) => file
            .symbol_by_index(index)
            .map_err(|error| format!("cannot read relocation symbol: {error}"))?
            .address(),
        RelocationTarget::Section(index) => file
            .section_by_index(index)
            .map_err(|error| format!("cannot read relocation section: {error}"))?
            .address(),
        RelocationTarget::Absolute => {
            return Err("absolute relocation target is unsupported".to_owned());
        }
        _ => return Err("unknown relocation target is unsupported".to_owned()),
    };
    checked_address(address, "relocation target")
}

fn relocation_kind(relocation: &object::Relocation) -> Result<RelocationKind, String> {
    let RelocationFlags::Elf { r_type } = relocation.flags() else {
        return Err("relocation is not an ARM ELF relocation".to_owned());
    };
    match r_type {
        object::elf::R_ARM_ABS32 => Ok(RelocationKind::Abs32),
        object::elf::R_ARM_THM_PC22 => Ok(RelocationKind::ThmCall),
        object::elf::R_ARM_THM_MOVW_ABS_NC => Ok(RelocationKind::ThmMovwAbsNc),
        object::elf::R_ARM_THM_MOVT_ABS => Ok(RelocationKind::ThmMovtAbs),
        _ => Err(format!("unsupported ARM relocation type {r_type:?}")),
    }
}

fn checked_address(value: u64, field: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{field} does not fit in AMRN u32"))
}

#[cfg(test)]
mod tests {
    use super::{CODE_SECTION, DATA_SECTION};

    #[test]
    fn keeps_application_sections_distinct() {
        assert_ne!(CODE_SECTION, DATA_SECTION);
    }
}
