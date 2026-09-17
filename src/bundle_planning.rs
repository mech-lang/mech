//! Retained source graph for static browser bundles.
use crate::nominal_provenance::transport_package_id;
use mech_core::{MResult, MechError};

pub(super) fn retained_sources(
    paths: &[std::path::PathBuf],
    base: &std::path::Path,
    project: &std::path::Path,
) -> MResult<(
    mech_runtime::InMemorySourceResolver,
    std::collections::HashMap<String, mech_runtime::SourceDocument>,
    Vec<mech_runtime::SourceResolutionEntry>,
)> {
    use mech_runtime::resolver::{
        ResolvedSource, import_may_resolve_source_dependency, import_requires_source_dependency,
    };
    let mut resolver = mech_runtime::InMemorySourceResolver::new();
    let filesystem = mech_runtime::FileSourceResolver::new(base).with_root(project);
    let mut documents = std::collections::HashMap::new();
    let mut owners = std::collections::HashMap::new();
    let mut resolutions = Vec::new();
    for path in paths {
        let relative = super::relative_source_path(path, base, project)?;
        let uri = format!("bundle:///{}", super::bundle_source_specifier(&relative)?);
        let text = std::fs::read_to_string(path)?;
        let provenance = filesystem.nominal_provenance_for_path(&path.canonicalize()?)?;
        let mut document = mech_runtime::SourceDocument::parse_resolved(
            &uri,
            mech_syntax::document::Revision(0),
            std::sync::Arc::<str>::from(text.as_str()),
            mech_syntax::document::ParseConfig::default(),
        )
        .map_err(|error| super::validation_error(format!("invalid bundle source: {error:?}")))?;
        let mut source = ResolvedSource::new(&uri, &uri, mech_core::MechSourceCode::String(text));
        if let Some((origin, package_id)) = provenance {
            // The retained graph is the public bundle's compilation boundary.
            // Keep path-bearing filesystem IDs out of both its manifest and
            // encoded program while retaining an exact collision discriminator.
            let package_id = transport_package_id(&package_id);
            document = document
                .with_nominal_origin(origin.clone())
                .with_nominal_package_id(package_id.clone());
            source = source
                .with_nominal_origin(origin)
                .with_nominal_package_id(package_id);
        }
        let source = source
            .with_kind(mech_runtime::SourceKind::from_path(path))
            .with_source_document(document.clone())?
            .admit_canonical_document()?;
        resolver.insert_source(uri.clone(), source)?;
        owners.insert(path.canonicalize()?, uri.clone());
        documents.insert(uri, document);
    }
    for (path, uri) in &owners {
        let index = documents[uri]
            .index()
            .map_err(|error| MechError::new(error, None))?;
        for import in index.root.program_imports() {
            if !import_may_resolve_source_dependency(&import) {
                continue;
            }
            let request = mech_runtime::resolver::source_request_for_import(&import, Some(uri));
            let filesystem_request = mech_runtime::SourceRequest::new(&request.specifier)
                .with_referrer(mech_runtime::path_to_file_uri(path)?);
            let resolved = filesystem
                .resolve_filesystem_path(&filesystem_request)?
                .and_then(|candidate| owners.get(&candidate));
            if let Some(target) = resolved {
                resolver.insert_resolution(uri, &request.specifier, target)?;
                resolutions.push(mech_runtime::SourceResolutionEntry::new(
                    uri.strip_prefix("bundle:///").unwrap_or(uri),
                    request.specifier,
                    target.strip_prefix("bundle:///").unwrap_or(target),
                ));
            } else if import_requires_source_dependency(&import) {
                return Err(super::validation_error(format!(
                    "bundle dependency {} from {uri} is absent from the bundled source set",
                    import.specifier
                )));
            }
        }
    }
    resolutions.sort();
    resolutions.dedup();
    Ok((resolver, documents, resolutions))
}
