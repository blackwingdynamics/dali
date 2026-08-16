pub(super) const V2_MEMORY_FILE: &str = "memory.x";
pub(super) const V3_MEMORY_FILE: &str = "memory.v3.x";

pub(super) fn default_target() -> Result<&'static dali_targets::TargetProfile, String> {
    dali_targets::SUPPORTED_TARGETS
        .first()
        .ok_or_else(|| "no application-supported target profile is declared".to_owned())
}

pub(super) fn render_v2_memory_script(template: &str) -> Result<String, String> {
    let target = default_target()?;
    Ok(template
        .replace(
            "{{ application_origin }}",
            &format!("0x{:08X}", target.memory.application_origin),
        )
        .replace(
            "{{ application_length }}",
            &format!("{}", target.memory.application_length),
        ))
}

pub(super) fn render_v3_memory_script(template: &str) -> Result<String, String> {
    let target = default_target()?;
    let isolation = target
        .memory
        .isolation
        .ok_or_else(|| format!("target `{}` has no isolation memory contract", target.name))?;
    Ok(template
        .replace(
            "{{ code_origin }}",
            &format!("0x{:08X}", isolation.code_origin),
        )
        .replace("{{ code_length }}", &format!("{}", isolation.code_length))
        .replace(
            "{{ data_origin }}",
            &format!("0x{:08X}", isolation.data_origin),
        )
        .replace("{{ data_length }}", &format!("{}", isolation.data_length)))
}
