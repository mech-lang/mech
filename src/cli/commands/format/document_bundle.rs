use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use base64::Engine as _;
use mech_core::{MResult, MechSourceCode};
use mech_runtime::{FileSourceResolver, SourceRequest, SourceResolver};

use super::{absolute_path, format_error};

#[derive(Debug, serde::Serialize)]
struct DocumentSourceBundle {
    version: u8,
    #[serde(rename = "rootSpecifier")]
    root_specifier: String,
    sources: Vec<DocumentSourceBundleEntry>,
    resolutions: Vec<DocumentSourceBundleResolution>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct DocumentSourceBundleEntry {
    specifier: String,
    source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct DocumentSourceBundleResolution {
    referrer: String,
    specifier: String,
    target: String,
}

#[derive(Debug)]
struct PendingSourceRequest {
    request: SourceRequest,
    referrer_bundle_key: Option<String>,
}

fn standalone_dependency_error(request: &SourceRequest) -> mech_core::MechError {
    let referrer = request.referrer.as_deref().unwrap_or("<unknown>");
    format_error(format!(
        "standalone HTML cannot bundle dependency `{}` requested by `{referrer}`; \
         only dependencies resolvable and packaged at format time are supported",
        request.specifier,
    ))
}

fn bundle_key(index: usize) -> String {
    format!("bundle/{index:06}.mec")
}

fn record_resolution(
    resolutions: &mut BTreeMap<(String, String), String>,
    referrer: String,
    specifier: String,
    target: String,
) -> MResult<()> {
    let key = (referrer, specifier);
    if let Some(existing) = resolutions.get(&key) {
        if existing == &target {
            return Ok(());
        }
        return Err(format_error(format!(
            "standalone source resolution `{}` from `{}` conflicts: `{existing}` and `{target}`",
            key.1, key.0,
        )));
    }
    resolutions.insert(key, target);
    Ok(())
}

pub(super) fn resolve_document_source_bundle(root: &Path) -> MResult<String> {
    let root = absolute_path(root)?;
    let root_request = SourceRequest::from_filesystem_path(root)?;
    let resolver = FileSourceResolver::empty();
    let mut pending = VecDeque::from([PendingSourceRequest {
        request: root_request,
        referrer_bundle_key: None,
    }]);
    let mut canonical_uri_to_bundle_key = BTreeMap::<String, String>::new();
    let mut sources = BTreeMap::<String, DocumentSourceBundleEntry>::new();
    let mut resolutions = BTreeMap::<(String, String), String>::new();

    while let Some(PendingSourceRequest {
        request,
        referrer_bundle_key,
    }) = pending.pop_front()
    {
        let Some(resolved) = resolver.resolve(&request)? else {
            return Err(standalone_dependency_error(&request));
        };
        let target_key = if let Some(key) = canonical_uri_to_bundle_key.get(&resolved.canonical_uri)
        {
            key.clone()
        } else {
            let key = bundle_key(canonical_uri_to_bundle_key.len());
            canonical_uri_to_bundle_key.insert(resolved.canonical_uri.clone(), key.clone());
            key
        };

        if let Some(referrer) = referrer_bundle_key {
            record_resolution(
                &mut resolutions,
                referrer,
                request.specifier.clone(),
                target_key.clone(),
            )?;
        }

        if sources.contains_key(&target_key) {
            continue;
        }

        let source = match resolved.source {
            MechSourceCode::String(source) => source,
            other => {
                return Err(format_error(format!(
                    "document source dependency `{}` is not Mech text: {:?}",
                    request.specifier, other,
                )));
            }
        };
        let dependencies = resolved.dependencies;
        sources.insert(
            target_key.clone(),
            DocumentSourceBundleEntry {
                specifier: target_key.clone(),
                source,
            },
        );
        pending.extend(
            dependencies
                .into_iter()
                .map(|request| PendingSourceRequest {
                    request,
                    referrer_bundle_key: Some(target_key.clone()),
                }),
        );
    }

    let root_specifier = bundle_key(0);
    if !sources.contains_key(&root_specifier) {
        return Err(format_error("document source bundle root was not resolved"));
    }
    let resolutions = resolutions
        .into_iter()
        .map(
            |((referrer, specifier), target)| DocumentSourceBundleResolution {
                referrer,
                specifier,
                target,
            },
        )
        .collect();
    let encoded = serde_json::to_vec(&DocumentSourceBundle {
        version: 2,
        root_specifier,
        sources: sources.into_values().collect(),
        resolutions,
    })
    .map_err(|error| format_error(format!("failed to encode document source bundle: {error}")))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(encoded))
}

#[cfg(test)]
#[path = "document_bundle/tests.rs"]
mod tests;
