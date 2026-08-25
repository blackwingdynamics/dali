use super::super::manifest::Authentication;

pub(super) fn validate_authentication(
    authentication: &Authentication,
    profile_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for policy in [&authentication.development, &authentication.release] {
        if policy != "unsigned" && policy != "ed25519" {
            return Err(format!(
                "target manifest {profile_name} has unsupported authentication policy `{policy}`"
            )
            .into());
        }
    }
    Ok(())
}
