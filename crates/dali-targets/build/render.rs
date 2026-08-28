pub(super) fn string_literal(value: &str) -> String {
    format!("{value:?}")
}

pub(super) fn constant_name(profile_name: &str) -> String {
    let suffix = profile_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("TARGET_{suffix}")
}
