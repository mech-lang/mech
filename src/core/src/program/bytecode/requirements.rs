pub use crate::{
    ApplicationRequirement, ExecutionHostFunctionRequest, ExecutionResourceRequest,
    ResourceDelivery, ResourceIntent,
};

use core::cmp::Ordering;

use crate::MResult;

use super::invalid;

/// Validates the canonical, host-independent representation carried by the
/// bytecode-v1 application-requirements section.
pub(crate) fn validate_application_requirement(
    requirement: &ApplicationRequirement,
) -> MResult<()> {
    match requirement {
        ApplicationRequirement::HostFunction(request) => {
            if request.name.is_empty() {
                return invalid("host function requirement name must not be empty");
            }
        }
        ApplicationRequirement::Resource(request) => {
            if request.operation.is_empty() || request.context_name.is_empty() {
                return invalid("resource requirement operation and context must not be empty");
            }
            validate_canonical_resource_base_uri(&request.base_uri)?;
            validate_normalized_resource_path(&request.path)?;
        }
    }
    Ok(())
}

fn validate_canonical_resource_base_uri(base_uri: &str) -> MResult<()> {
    // This deliberately matches the runtime's canonical resource identity:
    // `canonicalize_resource_base_uri` removes every trailing slash, requires
    // a scheme, and requires a non-empty authority. Bytecode stores only that
    // canonical result, never an equivalent pre-normalized spelling.
    if base_uri != base_uri.trim() {
        return invalid("resource requirement base URI must not have surrounding whitespace");
    }
    if base_uri.ends_with('/') {
        return invalid("resource requirement base URI must be canonical (no trailing slash)");
    }
    let Some((scheme, rest)) = base_uri.split_once("://") else {
        return invalid("resource requirement base URI must contain `://`");
    };
    if scheme.is_empty() {
        return invalid("resource requirement base URI scheme must not be empty");
    }
    let authority_end = rest.find('/').unwrap_or(rest.len());
    if authority_end == 0 {
        return invalid("resource requirement base URI authority must not be empty");
    }
    Ok(())
}

fn validate_normalized_resource_path(path: &str) -> MResult<()> {
    // Runtime normalization trims the whole path, removes empty and `.`
    // segments, and rejects `..`. Requiring bytecode to already equal that
    // result prevents equivalent resource identities from acquiring distinct
    // deterministic byte encodings.
    if path != path.trim() {
        return invalid("resource requirement path must not have surrounding whitespace");
    }
    for segment in path.split('/') {
        match segment {
            "" if !path.is_empty() => {
                return invalid(
                    "resource requirement path must not have leading, trailing, or duplicate separators",
                );
            }
            "." => return invalid("resource requirement path must not contain `.` segments"),
            ".." => return invalid("resource requirement path must not contain `..` segments"),
            _ => {}
        }
    }
    Ok(())
}

/// Compares requirements by their canonical bytecode-v1 serialization key.
pub fn compare_application_requirements(
    lhs: &ApplicationRequirement,
    rhs: &ApplicationRequirement,
) -> Ordering {
    match (lhs, rhs) {
        (ApplicationRequirement::HostFunction(lhs), ApplicationRequirement::HostFunction(rhs)) => {
            lhs.name.cmp(&rhs.name)
        }
        (ApplicationRequirement::HostFunction(_), ApplicationRequirement::Resource(_)) => {
            Ordering::Less
        }
        (ApplicationRequirement::Resource(_), ApplicationRequirement::HostFunction(_)) => {
            Ordering::Greater
        }
        (ApplicationRequirement::Resource(lhs), ApplicationRequirement::Resource(rhs)) => {
            (lhs.intent as u8)
                .cmp(&(rhs.intent as u8))
                .then_with(|| (lhs.delivery as u8).cmp(&(rhs.delivery as u8)))
                .then_with(|| lhs.operation.cmp(&rhs.operation))
                .then_with(|| lhs.context_name.cmp(&rhs.context_name))
                .then_with(|| lhs.base_uri.cmp(&rhs.base_uri))
                .then_with(|| lhs.path.cmp(&rhs.path))
        }
    }
}
