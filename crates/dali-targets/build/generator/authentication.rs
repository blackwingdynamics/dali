use super::super::manifest::{Authentication, TrustAnchor};

pub(super) fn generate_authentication(authentication: &Authentication) -> String {
    format!(
        "AuthenticationProfile {{ development: {}, release: {}, development_trust_anchors: &[{}], release_trust_anchors: &[{}] }}",
        generate_authentication_policy(&authentication.development),
        generate_authentication_policy(&authentication.release),
        authentication
            .development_trust_anchors
            .iter()
            .map(generate_trust_anchor)
            .collect::<Vec<_>>()
            .join(", "),
        authentication
            .release_trust_anchors
            .iter()
            .map(generate_trust_anchor)
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn generate_trust_anchor(anchor: &TrustAnchor) -> String {
    let key_id = parse_hex_array::<16>(&anchor.key_id, "key_id");
    let public_key = parse_hex_array::<32>(&anchor.public_key, "public_key");
    format!(
        "TrustAnchorProfile {{ key_id: {:?}, public_key: {:?} }}",
        key_id, public_key
    )
}

fn parse_hex_array<const N: usize>(value: &str, field: &str) -> [u8; N] {
    let normalized = value.strip_prefix("0x").unwrap_or(value);
    if normalized.len() != N * 2 {
        panic!("authentication trust anchor {field} must contain {N} bytes");
    }
    let mut bytes = [0; N];
    for (index, pair) in normalized.as_bytes().chunks_exact(2).enumerate() {
        let text = core::str::from_utf8(pair).expect("hex input is ASCII");
        bytes[index] = u8::from_str_radix(text, 16)
            .unwrap_or_else(|_| panic!("authentication trust anchor {field} is not hex"));
    }
    bytes
}

fn generate_authentication_policy(policy: &str) -> &'static str {
    match policy {
        "unsigned" => "CartridgeAuthentication::UnsignedAllowed",
        "ed25519" => "CartridgeAuthentication::Ed25519Required",
        _ => panic!("authentication policy was not validated"),
    }
}
